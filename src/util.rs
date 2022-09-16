
#[macro_export] 
macro_rules! sleep_ms {
    ($dur: expr) => (std::thread::sleep(std::time::Duration::from_millis($dur)))
}