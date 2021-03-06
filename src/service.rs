use std::path::PathBuf;
use std::process::Command;
use std::{ffi::OsString, thread, time::Duration};

use crate::err::Error;

use windows_service::{
  define_windows_service,
  service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceDependency,
    ServiceErrorControl, ServiceExitCode, ServiceInfo, ServiceStartType,
    ServiceState, ServiceStatus, ServiceType
  },
  service_control_handler::{self, ServiceControlHandlerResult},
  service_dispatcher,
  service_manager::{ServiceManager, ServiceManagerAccess}
};

use winreg::{enums::*, RegKey};

use log::{debug, error, info, trace, warn};

use crate::appstate::{state_channel, AppState, AppStateSender};
use crate::args;
use crate::termsig::{self, TermWait};

const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const SERVICE_STARTPENDING_TIME: Duration = Duration::from_secs(10);
const SERVICE_STOPPENDING_TIME: Duration = Duration::from_secs(30);


pub fn run(service_name: &str) -> windows_service::Result<()> {
  service_dispatcher::start(&service_name, ffi_service_main)
}

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(_arguments: Vec<OsString>) {
  // Reparse command line, just so we can get the service name
  let ctx = args::parse().unwrap();

  let service_name = match ctx.service_name {
    Some(s) => s,
    None => {
      // The command line parser should have forced the service name to have
      // been set.
      panic!("Missing service name");
    }
  };

  let lf = match get_service_param(&service_name, "LogLevel") {
    Some(loglevel) => match loglevel.as_ref() {
      "error" => log::LevelFilter::Error,
      "warn" => log::LevelFilter::Warn,
      "info" => log::LevelFilter::Info,
      "debug" => log::LevelFilter::Debug,
      "trace" => log::LevelFilter::Trace,
      _ => log::LevelFilter::Off
    },
    None => log::LevelFilter::Error
  };

  // For some odd reason, setting the loglevel parameter doesn't seem to have
  // any effect, so we set the max level manually after init.
  eventlog::init(&service_name, log::Level::Trace).unwrap();
  log::set_max_level(lf);

  info!("starting service");

  // Create signal for killing application
  let (kill_app_tx, kill_app_rx) = termsig::term_channel();

  // Define system service event handler that will be receiving service events.
  let event_handler = move |control_event| -> ServiceControlHandlerResult {
    match control_event {
      // Notifies a service to report its current status information to the
      // service control manager. Always return NoError even if not
      // implemented.
      ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
      ServiceControl::Stop => {
        debug!("svc signal recieved: stop");
        kill_app_tx.signal();
        ServiceControlHandlerResult::NoError
      }
      ServiceControl::Continue => {
        //info!("svc signal recieved: continue");
        ServiceControlHandlerResult::NotImplemented
      }
      ServiceControl::Pause => {
        debug!("svc signal recieved: pause");
        ServiceControlHandlerResult::NotImplemented
      }
      _ => ServiceControlHandlerResult::NotImplemented
    }
  };


  // Register system service event handler.  (The returned status handle
  // should be used to report service status changes to the system).
  let status_handle =
    service_control_handler::register(&service_name, event_handler).unwrap();

  // .. and then report that we're in the process of starting up.
  trace!("setting service state to 'start pending'");
  status_handle
    .set_service_status(ServiceStatus {
      service_type: SERVICE_TYPE,
      current_state: ServiceState::StartPending,
      controls_accepted: ServiceControlAccept::empty(),
      exit_code: ServiceExitCode::Win32(0),
      checkpoint: 0,
      wait_hint: SERVICE_STARTPENDING_TIME,
      process_id: None
    })
    .unwrap();

  // Create channel for reporting the application state to the application
  // state monitoring loop below.
  let (app_state_tx, app_state_rx) = state_channel();

  let exec = match get_service_param(&service_name, "Exec") {
    Some(exec) => exec,
    None => {
      error!("Unable to get Exec parameter.");
      return;
    }
  };

  let timeout = match get_service_param(&service_name, "Timeout") {
    Some(tm) => {
      let x: Duration;
      x = match tm.parse::<humantime::Duration>() {
        Ok(v) => v.into(),
        Err(e) => {
          error!(
            "Unable to parse Timeout parameter ({}), defaulting to 5 minutes",
            e
          );
          "5min".parse::<humantime::Duration>().unwrap().into()
        }
      };
      Some(x)
    }
    None => None
  };

  let port = match get_service_param(&service_name, "Port") {
    Some(tm) => Some(tm.parse::<u16>().unwrap()),
    None => None
  };


  let ctx = MsVsMonCtx {
    msvsmon: PathBuf::from(exec),
    timeout,
    port: port
  };


  trace!("launching thread for spawning msvsmon");
  let thrd = thread::spawn(move || {
    trace!("msvsmon worker thread reporting in");

    app_state_tx.starting(Some(1));

    let res = match run_msvsmon(ctx, &app_state_tx, kill_app_rx) {
      Ok(_) => {
        debug!("run_msvsmon() terminated successfully");
        true
      }
      Err(_) => {
        error!("run_msvsmon() terminated with an error");
        false
      }
    };

    app_state_tx.stopped();

    trace!("msvsmon worker thread reporting out");

    res
  });

  //
  // Enter a loop that waits for application to report back its status.
  // Terminate the loop once application reports that it has stopped.
  //
  loop {
    trace!("waiting for app state event");
    match app_state_rx.recv() {
      AppState::Starting(checkpoint) => {
        trace!("service starting checkpoint {}", checkpoint);
        status_handle
          .set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: checkpoint,
            wait_hint: SERVICE_STARTPENDING_TIME,
            process_id: None
          })
          .unwrap();
      }
      AppState::Started => {
        trace!("setting service state to 'running'");
        status_handle
          .set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None
          })
          .unwrap();
      }
      AppState::Stopping(checkpoint) => {
        trace!("service stopping checkpoint {}", checkpoint);
        status_handle
          .set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: checkpoint,
            wait_hint: SERVICE_STOPPENDING_TIME,
            process_id: None
          })
          .unwrap();
      }
      AppState::Stopped => {
        trace!("setting service state to 'stopped'");
        status_handle
          .set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None
          })
          .unwrap();

        break;
      }
    }
  }

  trace!("waiting for worker thread to croak");
  match thrd.join() {
    Ok(true) => {
      trace!("worker thread has croaked happy");
    }
    Ok(false) | Err(_) => {
      trace!("worker thread has croaked sad");
    }
  }

  info!("service terminated");
}


pub(crate) fn install(
  service_name: &str,
  ctx: &args::Context
) -> Result<(), Error> {
  let msvsmon = match &ctx.msvsmon {
    Some(msvsmon) => msvsmon,
    None => {
      // The command line parser should have made sure that this is set, so
      // this should neber happen.
      panic!("Missing msvsmon");
    }
  };

  println!(
    "==> Installing as service {} using {:?} ..",
    service_name, msvsmon
  );


  println!("==> Opening up firewall ..");
  let eargs = &["/prepcomputer", "/quiet"];
  let res = Command::new(msvsmon).args(eargs).output();
  let success = match res {
    Ok(output) => {
      if output.status.success() {
        true
      } else {
        println!("{:?} {:?} returned failure", msvsmon, eargs);

        let raw_output = String::from_utf8_lossy(&output.stdout);
        for line in raw_output.lines() {
          println!("[stdout] {}", line);
        }
        let raw_output = String::from_utf8_lossy(&output.stderr);
        for line in raw_output.lines() {
          println!("[stderr] {}", line);
        }
        false
      }
    }
    Err(_e) => {
      println!("Unable to run: {:?} {:?}", msvsmon, eargs);

      false
    }
  };

  println!("==> Registering event log source '{}' ..", service_name);
  eventlog::register(&service_name)?;

  let manager_access =
    ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
  let service_manager =
    ServiceManager::local_computer(None::<&str>, manager_access)?;

  let service_binary_path = ::std::env::current_exe()?;
  println!("==> Service exec path: {:?}", service_binary_path);


  let service_info = ServiceInfo {
    name: OsString::from(service_name),
    display_name: OsString::from("Verboten msvsmon"),
    service_type: ServiceType::OWN_PROCESS,
    start_type: ServiceStartType::AutoStart,
    error_control: ServiceErrorControl::Normal,
    executable_path: service_binary_path,
    launch_arguments: vec![OsString::from(service_name)],
    dependencies: vec![ServiceDependency::Service(OsString::from("Tcpip"))],
    account_name: None, // run as System
    account_password: None
  };
  //println!("==> Registering service '{}' ..", service_name);
  let service = service_manager
    .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
  service.set_description(
    "A service for launching msvsmon in maximum Bad Idea Mode."
  )?;

  // Once the service has been successfully registered, set up configuration
  // parameters in registry.
  let params = create_service_params(service_name)?;

  params.set_value("Exec", &msvsmon.to_str().unwrap())?;
  params.set_value("Port", &"4024")?;
  params.set_value("Timeout", &"1days")?;
  let ll = match &ctx.loglevel {
    Some(lev) => lev.to_string(),
    None => String::from("error")
  };
  params.set_value("LogLevel", &ll)?;


  Ok(())
}


pub(crate) fn uninstall(service_name: &str) -> Result<(), Error> {
  let manager_access = ServiceManagerAccess::CONNECT;
  let service_manager =
    ServiceManager::local_computer(None::<&str>, manager_access)?;
  let service_access =
    ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
  let service = service_manager.open_service(&service_name, service_access)?;

  // Make sure service is stopped before trying to delete it
  loop {
    let service_status = service.query_status()?;
    if service_status.current_state == ServiceState::Stopped {
      break;
    }
    println!("==> Requesting service '{}' to stop ..", service_name);
    service.stop()?;
    thread::sleep(Duration::from_secs(2));
  }


  println!("==> Removing service '{}' ..", service_name);
  service.delete()?;

  println!("==> Deregistering event log source '{}' ..", service_name);
  eventlog::deregister(&service_name)?;

  println!("==> Service uninstallation successful");

  Ok(())
}


struct MsVsMonCtx {
  msvsmon: PathBuf,
  timeout: Option<Duration>,
  port: Option<u16>
}

fn run_msvsmon(
  ctx: MsVsMonCtx,
  state_tx: &AppStateSender,
  kill_rx: TermWait
) -> Result<(), Error> {
  let mut eargs: Vec<String> = Vec::new();

  if let Some(port) = ctx.port {
    eargs.push(String::from("/port"));
    eargs.push(port.to_string());
  }

  if let Some(timeout) = ctx.timeout {
    eargs.push(String::from("/timeout"));
    eargs.push(timeout.as_secs().to_string());
  }

  // chest hairs mode
  // ``Our path was set by the travel agency.
  //   That's for schoolgirls.
  //   Now here's a route with some chest hair!''
  eargs.push(String::from("/noauth"));
  eargs.push(String::from("/anyuser"));
  eargs.push(String::from("/nosecuritywarn"));

  // background process
  eargs.push(String::from("/silent"));

  state_tx.starting(Some(2));

  debug!("Running: {:?} {:?}", ctx.msvsmon, eargs);
  let mut child = Command::new(&ctx.msvsmon).args(&eargs).spawn()?;

  // Report back to the service monitoring loop that we consider outselves to
  // be "started"
  state_tx.started();

  let mut do_kill = true;

  // Once the service event receiver get a "stop" request, we'll send a kill
  // request on this channel.  So wait here for it.
  loop {
    match kill_rx.wait() {
      termsig::Reason::Die => {
        debug!("kill switch activated");
        break;
      }
      termsig::Reason::Timeout => {
        trace!(
          "timed out while waiting for kill event -- check if msvsmon is \
           still alive"
        );
        match child.try_wait() {
          Ok(Some(status)) => {
            info!("Apparently msvsmon self-croaked with status {}", status);
            do_kill = false;
            break;
          }
          Ok(None) => {
            trace!("status not ready -- assuming msvsmon still running");
          }
          Err(e) => {
            warn!("error during try_wait(): {}", e);
            break;
          }
        }
      }
      termsig::Reason::Error => {
        error!("An error occured while waiting for kill event");
        break;
      }
    }
  }
  state_tx.stopping(Some(0));

  if do_kill {
    match child.kill() {
      Ok(_) => {
        debug!("msvsmon process killed successfully.");
      }
      Err(e) => match e.kind() {
        std::io::ErrorKind::InvalidInput => {
          warn!("msvsmon already dead");
        }
        _ => {
          error!("unable to kill msvsmon");
        }
      }
    }
  }

  Ok(())
}


/// Create a Parameters subkey for a service.
pub fn create_service_params(
  service_name: &str
) -> Result<winreg::RegKey, Error> {
  let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
  let services = hklm.open_subkey("SYSTEM\\CurrentControlSet\\Services")?;
  let asrv = services.open_subkey(service_name)?;
  let (subkey, _disp) = asrv.create_subkey("Parameters")?;

  Ok(subkey)
}


/// Load a service Parameter from the registry.
pub fn get_service_param(service_name: &str, key: &str) -> Option<String> {
  let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
  let services = match hklm.open_subkey("SYSTEM\\CurrentControlSet\\Services")
  {
    Ok(k) => k,
    Err(_) => return None
  };
  let asrv = match services.open_subkey(service_name) {
    Ok(k) => k,
    Err(_) => return None
  };
  let params = match asrv.open_subkey("Parameters") {
    Ok(k) => k,
    Err(_) => return None
  };

  match params.get_value::<String, &str>(key) {
    Ok(v) => Some(v),
    Err(_) => None
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
