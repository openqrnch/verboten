use std::fmt;
use std::str::FromStr;

use crate::err::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
  Off,
  Error,
  Warn,
  Info,
  Debug,
  Trace
}

impl FromStr for LogLevel {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "off" => Ok(LogLevel::Off),
      "error" => Ok(LogLevel::Error),
      "warn" => Ok(LogLevel::Warn),
      "info" => Ok(LogLevel::Info),
      "debug" => Ok(LogLevel::Debug),
      "trace" => Ok(LogLevel::Trace),
      _ => Err(Error::BadInput(format!("Unknown log level '{}'", s)))
    }
  }
}

impl Default for LogLevel {
  fn default() -> Self {
    LogLevel::Error
  }
}

impl fmt::Display for LogLevel {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s = match self {
      LogLevel::Off => "off",
      LogLevel::Error => "error",
      LogLevel::Warn => "warn",
      LogLevel::Info => "info",
      LogLevel::Debug => "debug",
      LogLevel::Trace => "trace"
    };
    write!(f, "{}", s)
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
