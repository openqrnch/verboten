
# ----------------------------------------------------------------------------
# For sanity.
# ----------------------------------------------------------------------------
Set-StrictMode -Version Latest

# Default to croak on any error
$ErrorActionPreference = "Stop"

# Release build
& cargo build --release
if($LastExitCode -ne 0) {
  Exit
}


# Microsoft Jump-Through-Hoops(tm)
$progdir = "${env:ProgramFiles(x86)}"
$vsinstdir = "${progdir}\Microsoft Visual Studio\Installer"
$vswhere = "$vsinstdir\vswhere.exe"

$cmdargs = '-latest', '-find', 'VC\Tools\**\HostX64\x64\dumpbin.exe'
$DumpBin = & $vswhere $cmdargs
$DumpBin = $DumpBin[0]


# Dump load-time dependencies so we can make sure we haven't pulled in some
# annoying dependencies.
& $DumpBin /nologo /dependents target\release\verboten.exe

$Exists = Test-Path -Path $HOME\vboxshares\win10 -PathType Container
if($Exists) {
  Copy-Item target\release\verboten.exe -Destination $HOME\vboxshares\win10\
}

# vim: set ft=ps1 et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
