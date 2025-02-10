use crate::{severity::LogSeverity, Logger};


pub struct LogFilter {
    next_logger: Box<dyn Logger + Send>,
    log_level: LogSeverity,
}

impl Logger for LogFilter {
    fn log(&mut self, message: &crate::LogMessage) -> bool {
        if message.severity <= self.log_level {
            self.next_logger.log(message)
        } else {
            true
        }
    }
}

impl LogFilter {
    pub fn new<L: Logger + Send + 'static>(sev: LogSeverity, logger: L) -> Self {
        Self {
            next_logger: Box::new(logger),
            log_level: sev
        }
    }
}