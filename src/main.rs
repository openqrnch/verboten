mod appstate;
mod args;
mod err;
mod loglevel;
mod service;
mod termsig;

#[cfg(not(windows))]
compile_error!("verboten is only supported on Windows");

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let ctx = args::parse()?;
  if ctx.do_help || ctx.do_version {
    return Ok(());
  }

  let service_name = match &ctx.service_name {
    Some(nm) => nm,
    None => {
      // The command line parser should have made certain that we have the
      // service name at this point.
      panic!("Missing service name");
    }
  };

  match ctx.svcaction {
    Some(args::SvcAction::Install) => {
      service::install(&service_name, &ctx)?;
      return Ok(());
    }
    Some(args::SvcAction::Uninstall) => {
      service::uninstall(&service_name)?;
      return Ok(());
    }
    _ => {}
  }

  service::run(&service_name)?;

  Ok(())
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
