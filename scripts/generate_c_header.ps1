param(
    [switch]$Verify
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$config = Join-Path $repoRoot "cbindgen.toml"
$output = Join-Path $repoRoot "include/fjiffyldg.h"

$cbindgen = Get-Command cbindgen -ErrorAction SilentlyContinue
if ($null -eq $cbindgen) {
    Write-Error "cbindgen is required to generate the C API header. Install it with: cargo install cbindgen --locked"
}

$args = @(
    "--quiet",
    "--config", $config,
    "--crate", "fjiffyldg",
    "--output", $output
)

if ($Verify) {
    $args += "--verify"
}

Push-Location $repoRoot
try {
    & $cbindgen.Source @args
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}

if ($Verify) {
    Write-Host "C API header is up to date: $output"
} else {
    Write-Host "Generated C API header: $output"
}
