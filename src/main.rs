use std::{env};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::thread::{self};
use kill::process_kill;
use user_error::{UserFacingError, UFE};
use std::iter::Iterator;
use std::sync::mpsc;

mod kill;
mod state;
mod line_handling;
mod util;

use line_handling::{LineHandler, LineError, LineSuccess};
use state::ArgMode::{self, Picoprobe, GeneralRp2040};


#[macro_export] macro_rules! buf_reader_get_lines {
    ($buf_reader: expr) => ($buf_reader.lines().filter_map(|line| line.ok()))
}

#[macro_export] macro_rules! pipe_get_lines {
    ($pipe: expr) => (buf_reader_get_lines!(BufReader::new($pipe)))   
}


#[macro_export] macro_rules! handle_line {
    ($io: expr, $callback: expr) => {
        {
            pipe_get_lines!($io).for_each(|line|{
                $callback;
            });
        }
    }
}

struct App {
    line_handler: LineHandler,
    args: Vec<String>,
    subprocess_args: Vec<String>,
    arg_mode: ArgMode,
    restart_openocd: bool 
}

impl App {
    pub(crate) fn new() -> Self {
        let line_handler = LineHandler::new();
        let args = env::args().collect();
        let subprocess_args = Vec::new();
        let arg_mode = ArgMode::None;
        let restart_openocd = false;
        Self {
            line_handler,
            args,
            subprocess_args,
            arg_mode,
            restart_openocd
        }
    }

    pub(crate) fn handle_line(&mut self, line: String) -> Result<LineSuccess, LineError> {
        let result = self.line_handler.handle(line);

        match result {
            Err(LineError::AddressAlreadyInUse) => {
                UserFacingError::new("OpenOCD reports: Address already in use")
                    .reason("Maybe an old instance of OpenOCD?")
                    .reason("Maybe another program has port open on same machine?")
                    .reason(format!("Port number: {}", self.line_handler.address_already_in_use_port))
                    .print();
                process_kill("openocd".to_string());
            }

            Err(LineError::PicoProbeNotFound) => {
                UserFacingError::new("OpenOCD failed to connect to PicoProbe.")
                    .reason("Maybe it's unplugged?")
                    .reason("Maybe your udev is misconfigured?")
                    .print_and_exit(); // we are launched by udev,
                    // and we don't want to run as a service,
                    // So we should stop if there's no picoprobe detected...
            }

            Err(LineError::CantFindOpenOCD) => {
                UserFacingError::new("OpenOCD can't find config.")
                    .reason("Maybe you are launching with the wrong mode?")
                    .print_and_exit(); // This will not fix with retries, so, goodbye cruel world :')
            }
            
            Err(LineError::NoMoreConnections) => {
                UserFacingError::new("OpenOCD reports too many connections")
                    .reason("Could be too many ghost connections.");
                self.restart_openocd = true;
            }

            Ok(_) => {
                // Nothing to do
            }
        }

        result
    }


    fn launch_child(&mut self) {
        // MPSC is *necessary* for this application, if you don't like polling.
        // It stands for Multiple Producer, Single Consumer
        // It allows us to push from multiple sources to a queue and then pop that queue from the main thread.
        // In particular, these tx and rx are for sending strings from the threads watching for STDOUT & STDERR.
        // Originally, I thought this design was stupid and unnecessary, but then I saw someone also converge toward it
        // on stack overflow. 
        // Which means it's dumb but at least not _completely_ insane, right?
        // Right?
        let (tx, rx) = mpsc::channel::<String>();

        println!("Running command...");

        // Based heavily on the journalctl example here:
        // https://rust-lang-nursery.github.io/rust-cookbook/os/external.html
        let result = Command::new("/opt/rpi-openocd/src/openocd")
            .args(self.subprocess_args.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        if result.is_err() {
            UserFacingError::new("Could not spawn process")
            .help("Maybe a problem with PATH?").print_and_exit();
        }

        let mut child = result.unwrap();
    
        let option = child.stdout.take();
    
        if option.is_none() {
            UserFacingError::new("Could not grab STDOUT. Weird :S").print_and_exit();
        }

        let stdout = option.unwrap();
        let tx_stdout = tx.clone();
        thread::spawn(move || {
            pipe_get_lines!(stdout).for_each(|line| { tx_stdout.send(line).unwrap(); });
        });

        let option = child.stderr.take();

        if option.is_none() {
            UserFacingError::new("Could not grab STDERR... WTF?").print_and_exit();
        }

        let stderr = option.unwrap();
        let tx_stderr = tx.clone();
        thread::spawn(move || {
            pipe_get_lines!(stderr).for_each(|line| { tx_stderr.send(line).unwrap(); });
        });
        
        for line in rx {
            println!("Line: {}", line);
            self.handle_line(line).unwrap();
            if self.restart_openocd {
                self.restart_openocd = false; // Don't restart on next iteration
                child.kill().unwrap(); // What brutality Y~Y
            }
        }
    }

    pub fn main(&mut self) {
        let args_in_picoprobe_mode: Vec<&str> = vec! (
            "-s", "/opt/rpi-openocd/tcl",
            "-f", "interface/picoprobe.cfg",
            "-f", "target/rp2040.cfg"
        );

        
        let args_in_general_rp2040_mode: Vec<&str> = vec! (
            "-s", "/opt/rpi-openocd/tcl",
            "-f", "target/rp2040.cfg"
        );


        // This is true if we are called via a symlink like `rpi-openocd-picoprobe`
        if self.args[0].ends_with("picoprobe") {
            self.arg_mode = Picoprobe;
        }

        // Iterate through all after 0th (call path) arg
        // If we recognize a flag for us, intercept it (e.g., --picoprobe-mode)
        // else, push it to the vector for subprocess. Probably meant for it.
        for arg in &self.args.as_slice()[1..] {
            if arg.eq("--picoprobe-mode") {
                self.arg_mode = Picoprobe;
            } else if arg.eq("--general-rp2040-mode") {
                self.arg_mode = GeneralRp2040;
            } else {
                self.subprocess_args.push(arg.clone());
            }
        }


        // If we're in picoprobe mode, push the picoprobe args from stack
        match self.arg_mode {
            Picoprobe => {
                for arg in args_in_picoprobe_mode {
                    self.subprocess_args.push(String::from(arg));
                }
            },
            ArgMode::GeneralRp2040 => {
                for arg in args_in_general_rp2040_mode {
                    self.subprocess_args.push(String::from(arg));
                }
            },
            ArgMode::None => {}
        }

        loop {
            self.launch_child();
            // Grace period to avoid taxing CPU
            sleep_ms!(1000);
        }
    }
}


fn main() {
    let mut app = App::new();
    app.main();
}
