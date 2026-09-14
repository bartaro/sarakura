param([switch]$Offline)
$ErrorActionPreference = 'Stop'
# Resolve the repository from this script so the caller can build from another
# directory; preserve the caller's working directory and Rust compiler flags.
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$previousRustFlags = $env:RUSTFLAGS
Push-Location $repositoryRoot
try {
    # Append the static MSVC runtime flag for the Windows x64 CLI release. Keep
    # existing flags; --locked prevents dependency resolution from changing the lockfile.
    $env:RUSTFLAGS = (($previousRustFlags + ' -C target-feature=+crt-static').Trim())
    $cargoArgs = @('build','--release','--locked','--target','x86_64-pc-windows-msvc','-p','sarakura-cli','--bin','sarakura')
    if ($Offline) { $cargoArgs += '--offline' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    # Ask Cargo for its effective target directory, including configuration
    # overrides, rather than assuming a repository-local target folder.
    $metadataText = & cargo metadata --no-deps --format-version 1 --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Unable to locate the Cargo target directory' }
    $metadata = $metadataText | ConvertFrom-Json
    $binary = Join-Path $metadata.target_directory 'x86_64-pc-windows-msvc/release/sarakura.exe'
    # Replace the root executable only after a successful build and metadata query.
    Copy-Item -LiteralPath $binary -Destination (Join-Path $repositoryRoot 'sarakura.exe') -Force
    Write-Host 'Ready: sarakura.exe (Windows x64 CLI)'
} finally {
    # Restore caller state on success or failure.
    $env:RUSTFLAGS = $previousRustFlags
    Pop-Location
}
