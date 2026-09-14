param(
    [string]$OutDir = "dist"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$version = "1.0.0"
$stamp = Get-Date -Format "yyyyMMdd"
# Create a dated source-workspace archive in the chosen output directory.
# This script does not build or include the root release executable.
$releaseDir = Join-Path $root $OutDir
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

Push-Location $root
try {
    # Run formatting, workspace tests and schema export before collecting files.
    # Native exit codes are not explicitly checked here; on Windows PowerShell,
    # ErrorActionPreference alone does not make every failed Cargo command fatal.
    cargo fmt --all -- --check
    cargo test --workspace
    cargo run -p sarakura-cli -- schema export --out schemas
    $zip = Join-Path $releaseDir "sarakura_v1_0_0_rust_workspace_$stamp.zip"
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
    # Archive the fixed allowlist recursively as it exists on disk. It is not a
    # git-tracked-files export; use a clean staging tree to exclude generated files.
    $items = @("Cargo.toml","Cargo.lock","README.md","LICENSE","crates","docs","examples","schemas","scripts","tests")
    Compress-Archive -LiteralPath $items -DestinationPath $zip
    Write-Host "wrote $zip"
}
finally {
    # Return to the caller's directory even when an archive operation fails.
    Pop-Location
}
