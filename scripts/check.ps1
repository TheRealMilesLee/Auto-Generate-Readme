# Local quality gate for Windows (PowerShell). No GitHub Actions.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

if (Get-Command cargo-fmt -ErrorAction SilentlyContinue) { cargo fmt --check }
if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) { cargo clippy --all-targets -- -D warnings }
cargo test
cargo build --release

if ((Test-Path README.md) -and (Select-String -Path README.md -Pattern "readme-gen:meta" -Quiet)) {
    .\target\release\readme-gen.exe check --strict
}
