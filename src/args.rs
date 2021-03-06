use std::path::PathBuf;
use std::str::FromStr;

use qargparser as arg;

use crate::loglevel::LogLevel;

use crate::err::Error;

#[derive(Debug, Clone)]
pub(crate) enum SvcAction {
  Install,
  Uninstall
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Context {
  pub(crate) do_help: bool,
  pub(crate) do_version: bool,
  pub(crate) service_name: Option<String>,
  pub(crate) svcaction: Option<SvcAction>,
  pub(crate) msvsmon: Option<PathBuf>,
  pub(crate) loglevel: Option<LogLevel>
}


/// Parse the command line.
pub(crate) fn parse() -> Result<Context, Error> {
  let actx = Context {
    ..Default::default()
  };

  let mut prsr = arg::Parser::from_env(actx);

  prsr.add(
    arg::Builder::new()
      .sopt('h')
      .lopt("help")
      .exit(true)
      .help(&["Show this help."])
      .build(|_spec, ctx: &mut Context, _args| {
        ctx.do_help = true;
      })
  )?;
  prsr.add(
    arg::Builder::new()
      .sopt('V')
      .exit(true)
      .lopt("version")
      .help(&["Show version and exit."])
      .build(|_spec, ctx: &mut Context, _args| {
        ctx.do_version = true;
      })
  )?;
  prsr.add(
    arg::Builder::new()
      .sopt('L')
      .lopt("log-level")
      .help(&[
        "Maximum level of log records.",
        "Values: off, error (default), warn, info, debug, trace"
      ])
      .nargs(arg::Nargs::Count(1), &["LEVEL"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.loglevel = Some(LogLevel::from_str(&args[0]).unwrap());
      })
  )?;
  prsr.add(
    arg::Builder::new()
      .sopt('i')
      .lopt("install")
      .nargs(arg::Nargs::Count(1), &["EXEC"])
      .help(
        &[
          "Install service.  The EXEC argument must be the absolute path and \
           filename of msvsmon.exe."
        ]
      )
      .build(|_spec, ctx: &mut Context, args| {
        ctx.svcaction = Some(SvcAction::Install);

        let exec = PathBuf::from(&args[0]);
        if !exec.exists() {
          panic!("msvsmon.exe not found at {:?}", exec);
        }
        let exec = match std::fs::canonicalize(exec) {
          Ok(exec) => {
            const PREFIX: &str = r#"\\?\"#;
            let exec_str = exec.to_str().unwrap();
            if exec_str.starts_with(&PREFIX) {
              PathBuf::from(exec_str.strip_prefix(PREFIX).unwrap())
            } else {
              exec
            }
          }
          Err(_) => panic!("Unable to get the absolute path of msvsmon.")
        };

        ctx.msvsmon = Some(exec);
      })
  )?;
  prsr.add(
    arg::Builder::new()
      .sopt('u')
      .lopt("uninstall")
      .help(&["Uninstall service with name NAME and exit."])
      .build(|_spec, ctx: &mut Context, _args| {
        ctx.svcaction = Some(SvcAction::Uninstall);
      })
  )?;
  prsr.add(
    arg::Builder::new()
      .required(true)
      .nargs(arg::Nargs::Count(1), &["NAME"])
      .help(&["Use service name NAME."])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.service_name = Some(args[0].clone());
      })
  )?;

  prsr.parse()?;

  if prsr.get_ctx().do_help {
    prsr.usage(&mut std::io::stdout());
    return Ok(prsr.into_ctx());
  }

  if prsr.get_ctx().do_version {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("verboten {}", VERSION);
    return Ok(prsr.into_ctx());
  }


  Ok(prsr.into_ctx())
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
