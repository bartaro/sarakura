param(
    [string]$OutDir = "dist"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$version = "1.0.0"
$stamp = Get-Date -Format "yyyyMMdd"
$releaseDir = Join-Path $root $OutDir
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

Push-Location $root
try {
    cargo fmt --all -- --check
    cargo test --workspace
    cargo run -p sarakura-cli -- schema export --out schemas
    $zip = Join-Path $releaseDir "sarakura_v1_0_0_rust_workspace_$stamp.zip"
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
    $items = @("Cargo.toml","Cargo.lock","README.md","LICENSE","crates","docs","examples","schemas","scripts","tests")
    Compress-Archive -LiteralPath $items -DestinationPath $zip
    Write-Host "wrote $zip"
}
finally {
    Pop-Location
}
