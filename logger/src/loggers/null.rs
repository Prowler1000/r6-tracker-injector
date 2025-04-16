use crate::Logger;


#[derive(Default)]
pub struct NullLogger {}

impl NullLogger {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Logger for NullLogger {
    fn log(&mut self, _message: &crate::LogMessage) -> bool {
        true
    }
}