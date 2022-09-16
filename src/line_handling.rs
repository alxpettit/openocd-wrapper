use core::fmt;
use regex::Regex;

#[derive(Debug)]
pub struct LineSuccess{}

impl LineSuccess {
    fn new() -> LineSuccess {
        LineSuccess {}
    }
}

#[derive(Debug)]
pub enum LineError {
    AddressAlreadyInUse,
    PicoProbeNotFound,
    CantFindOpenOCD,
    NoMoreConnections
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LineError::AddressAlreadyInUse => write!(f, "AddressAlreadyInUse"),
            LineError::PicoProbeNotFound => write!(f, "PicoProbeNotFound"),
            LineError::CantFindOpenOCD => write!(f, "CantFindOpenOCD"),
            LineError::NoMoreConnections => write!(f, "NoMoreConnections")
        }
    }
}

pub(crate) struct LineHandler { 
    pub(crate) re_address_already_in_use: Regex,
    pub(crate) re_picoprobe_not_found: Regex,
    pub(crate) address_already_in_use: bool,
    pub(crate) address_already_in_use_port: String,
    pub(crate) re_cant_find_openocd: Regex,
    pub(crate) re_no_more_connections: Regex
}

impl LineHandler {
    pub(crate) fn handle(& mut self, line: String) -> Result<LineSuccess, LineError> {
        if let Some(result) = self.re_address_already_in_use.captures(&line) {
            let port = &result[1];
            let error_msg = &result[2];
            if error_msg == "Address already in use" {
                self.address_already_in_use = true;
                self.address_already_in_use_port = port.to_string();
                return Err(LineError::AddressAlreadyInUse);
            }
        }

        if let Some(_result) = self.re_picoprobe_not_found.captures(&line) {
            return Err(LineError::PicoProbeNotFound);
        }

        if let Some(_result) = self.re_cant_find_openocd.captures(&line) {
            return Err(LineError::CantFindOpenOCD);
        }

        if let Some(_result) = self.re_no_more_connections.captures(&line) {
            return Err(LineError::NoMoreConnections);
        }

        Ok(LineSuccess::new())
    }

    pub(crate) fn new() -> LineHandler {
        LineHandler {
            re_address_already_in_use: Regex::new(r"bind.*port (\d*): (.*)").unwrap(),
            re_picoprobe_not_found: Regex::new(r".*Can't find a picoprobe device!.*").unwrap(),
            re_cant_find_openocd: Regex::new(r".*Can't find openocd.cfg.*").unwrap(),
            re_no_more_connections: Regex::new(".*rejected.*no more connections allowed.*").unwrap(),
            address_already_in_use: false,
            address_already_in_use_port: "unknown".to_string()
        }
    }
}



// #[macro_export] macro_rules! handle_line {
//     ($io: expr, $arg_mode: expr) => {
//         {
//             let mut line_handler = LineHandler::new();
//             pipe_get_lines!($io).for_each(|line|{
//                 if let Err(result) = line_handler.handle(line) {
//                     result
//                 }
//             });
//         }
//     }
// }

// #[macro_export] macro_rules! handle_line {
//     ($io: expr, $arg_mode: expr) => (
//         let mut line_handler = LineHandler::new();
//         pipe_get_lines!($io).for_each(|line|{
//             if let Err(result) = line_handler.handle(line) {
//                 result
//                 .reason(format!("Launched with argument mode: {:?}", $arg_mode))
//                 .print_and_exit();
//             }
//         });
//     )
// }

