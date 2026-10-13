$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not $IsWindows -and $env:OS -ne 'Windows_NT') { throw 'Windows x64 is required.' }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $rustBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path (Join-Path $rustBin 'cargo.exe')) { $env:PATH = "$rustBin;$env:PATH" }
}
foreach ($tool in @('node','npm','cargo')) { if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing prerequisite: $tool" } }
function Run-Step([string] $Command, [string[]] $Arguments) { & $Command @Arguments; if ($LASTEXITCODE -ne 0) { throw "Failed: $Command $Arguments" } }
Run-Step npm @('ci')
Run-Step npm @('run','lint')
Run-Step npm @('run','test')
Run-Step npm @('run','build')
Run-Step cargo @('fmt','--manifest-path','src-tauri/Cargo.toml','--','--check')
Run-Step cargo @('clippy','--manifest-path','src-tauri/Cargo.toml','--locked','--all-targets','--','-D','warnings')
Run-Step cargo @('test','--manifest-path','src-tauri/Cargo.toml','--locked')
Run-Step npm @('run','tauri','--','build','--bundles','nsis')
$installers = Get-ChildItem -LiteralPath 'src-tauri/target/release/bundle/nsis' -Filter '*.exe'
if (-not $installers) { throw 'NSIS installer not found.' }
$installers | ForEach-Object { Write-Output ("Installer: {0} ({1} bytes)" -f $_.FullName,$_.Length) }
