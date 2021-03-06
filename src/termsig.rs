use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

pub struct TermSignal {
  tx: Sender<()>
}

/// In order to terminate the fs monitor loop we'll send a nonsense event that
/// we can detect in the loop and croak when received.
impl TermSignal {
  /// Tell the fs monitor loop to self-croak.
  pub fn signal(&self) {
    self.tx.send(()).unwrap();
  }
}


pub struct TermWait {
  rx: Receiver<()>
}

pub enum Reason {
  Timeout,
  Die,
  Error
}

impl TermWait {
  pub fn wait(&self) -> Reason {
    let d = Duration::from_secs(1);
    match self.rx.recv_timeout(d) {
      Ok(_) => Reason::Die,
      Err(RecvTimeoutError::Timeout) => Reason::Timeout,
      Err(RecvTimeoutError::Disconnected) => Reason::Error
    }
  }
}


pub fn term_channel() -> (TermSignal, TermWait) {
  let (tx, rx) = channel();

  let termsig = TermSignal { tx };
  let termwait = TermWait { rx };

  (termsig, termwait)
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
