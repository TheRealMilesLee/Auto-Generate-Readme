# Local quality gate for Windows (PowerShell). No GitHub Actions.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

& cargo fmt --version *> $null
if ($LASTEXITCODE -eq 0) { cargo fmt --check } else { Write-Host "note: rustfmt not installed, skipping format check" }

& cargo clippy --version *> $null
if ($LASTEXITCODE -eq 0) { cargo clippy --all-targets -- -D warnings } else { Write-Host "note: clippy not installed, skipping lint" }

cargo test
cargo build --release

if ((Test-Path README.md) -and (Select-String -Path README.md -Pattern "readme-gen:meta" -Quiet)) {
    .\target\release\readme-gen.exe check --strict
}
