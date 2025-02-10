use crate::Logger;


#[derive(Default)]
pub struct ConsoleLogger {}

impl Logger for ConsoleLogger {
    fn log(&mut self, message: &crate::LogMessage) -> bool {
        println!("({}) {} : {}", message.time.format("%I:%M:%S%p"), message.severity, message.content);
        true
    }
}

impl ConsoleLogger {
    pub fn new() -> Self {
        Default::default()
    }
}
