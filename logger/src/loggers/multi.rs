use crate::Logger;

#[derive(Default)]
pub struct MultiLogger {
    loggers: Vec<Box<dyn Logger + Send>>,
}

impl Logger for MultiLogger {
    fn log(&mut self, message: &crate::LogMessage) -> bool {
        let mut res = true;
        for logger in &mut self.loggers {
            res = res && logger.log(message);
        }
        res
    }
}

impl MultiLogger {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_logger<T: Logger + Send + 'static>(mut self, logger: T) -> Self {
        self.loggers.push(Box::new(logger));
        self
    }
}
