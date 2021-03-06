use std::sync::mpsc::{channel, Receiver, Sender};

use log::trace;

/// States that can be reported back to a service subsystem.
#[derive(Debug)]
pub enum AppState {
  Starting(u32),
  Started,
  Stopping(u32),
  Stopped
}

pub struct AppStateSender {
  tx: Sender<AppState>
}

impl AppStateSender {
  pub fn starting(&self, checkpoint: Option<u32>) {
    trace!("sending WinAppState::Starting");
    if let Some(cp) = checkpoint {
      self.tx.send(AppState::Starting(cp)).unwrap();
    } else {
      self.tx.send(AppState::Starting(0)).unwrap();
    }
    trace!("WinAppState::Starting sent");
  }

  pub fn started(&self) {
    trace!("sending WinAppState::Started");
    self.tx.send(AppState::Started).unwrap();
    trace!("WinAppState::Started sent");
  }


  /// Called to notify the service module that shutdown process has been
  /// initiated.
  pub fn stopping(&self, checkpoint: Option<u32>) {
    trace!("sending AppState::Stopping");
    if let Some(cp) = checkpoint {
      self.tx.send(AppState::Stopping(cp)).unwrap();
    } else {
      self.tx.send(AppState::Stopping(0)).unwrap();
    }
    trace!("AppState::Stopping sent");
  }

  /// Called to notify the service module that the shutdown process has been
  /// complete.
  pub fn stopped(&self) {
    trace!("sending AppState::Stopped");
    self.tx.send(AppState::Stopped).unwrap();
    trace!("AppState::Stopped sent");
  }
}


pub struct AppStateReceiver {
  rx: Receiver<AppState>
}

impl AppStateReceiver {
  pub fn recv(&self) -> AppState {
    self.rx.recv().expect("Unable to receive app state")
  }
}


pub fn state_channel() -> (AppStateSender, AppStateReceiver) {
  let (tx, rx) = channel();

  let sender = AppStateSender { tx };
  let receiver = AppStateReceiver { rx };

  (sender, receiver)
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
