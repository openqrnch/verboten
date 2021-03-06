# verboten
If you've needed to attach a debugger to a process before login (while
developing credential providers, Lsa modules, etc) you may have used remote
debugging using msvsmon-as-service and Visual Studio.

Setting up msvsmon as a service can be a little fiddly.  This crate is a
simple service wrapper for msvsmon.  It is intended to be maximally simple to
use.  This has the unfortunate side effect that it is maximally unsafe.  Only
run this on isolated machines.

# Installation

Build `verboten` from crates.io:

```
cargo install verboten
```

.. or from the repository:

```
cargo build --release
```

(It will statically link the CRT, so don't worry about vcredist if you don't
need it for other reasons).

Copy verboten.exe into the remote system.  The location of it isn't really
important.

Copy `msvsmon.exe` and its dependencies to the remote system.

On the remote system, install the service using:

```
verboten.exe --install <path and name of msvsmon.exe> --install <service name>
```

Example:

```
verboten.exe --install C:\Temp\x64\msvsmon.exe --install verboten
```

The installer will create a `Parameters` subkey under the  service's registry
subkey with some useful settings, in particular:

`LogLevel` can be set to `error`, `warn`, `info`, `debug` or `trace`.
(Warning, some of the higher levels are very spammy).

`Timeout` can be set how long the msvsmon process will live before
self-terminating.

The service will output its log to the Windows event log.

