use std::fmt::Display;

use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd)]
pub enum LogSeverity {
    Error,
    Warning,
    Info,
    Debug,
    Verbose,
}

impl Display for LogSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sev = match self {
            LogSeverity::Error => "Error",
            LogSeverity::Warning => "Warn",
            LogSeverity::Info => "Info",
            LogSeverity::Debug => "Debug",
            LogSeverity::Verbose => "Verbose",
        };
        write!(f, "{}", sev)
    }
}

