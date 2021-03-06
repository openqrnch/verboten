use std::fmt;
use std::io;

use qargparser as ap;

#[derive(Debug)]
pub enum Error {
  IO(String),
  BadFormat(String),
  BadInput(String),
  ArgParser(String),
  Service(String),
  EventLog(String),
  RegistryKey(String)
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Error::IO(err.to_string())
  }
}

impl<T> From<ap::ErrKind<T>> for Error {
  fn from(err: ap::ErrKind<T>) -> Self {
    Error::ArgParser(err.to_string())
  }
}

impl From<windows_service::Error> for Error {
  fn from(err: windows_service::Error) -> Self {
    Error::Service(err.to_string())
  }
}

impl From<eventlog::Error> for Error {
  fn from(err: eventlog::Error) -> Self {
    Error::EventLog(err.to_string())
  }
}

impl From<registry::key::Error> for Error {
  fn from(err: registry::key::Error) -> Self {
    Error::RegistryKey(err.to_string())
  }
}


impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &*self {
      Error::IO(s) => write!(f, "I/O error; {}", s),
      Error::BadFormat(s) => write!(f, "Bad format error; {}", s),
      Error::BadInput(s) => write!(f, "Bad input error; {}", s),
      Error::ArgParser(s) => write!(f, "ArgParser error; {}", s),
      Error::Service(s) => write!(f, "Service error; {}", s),
      Error::EventLog(s) => write!(f, "EventLog error; {}", s),
      Error::RegistryKey(s) => write!(f, "Registry Key error; {}", s)
    }
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
