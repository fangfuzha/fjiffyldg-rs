param(
    [string]$Compiler = "cc"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$source = Join-Path $repoRoot "tests/c_smoke.c"
$include = Join-Path $repoRoot "include"
$outDir = Join-Path $repoRoot "target/c-abi-smoke"
$outFile = Join-Path $outDir "c_smoke.o"

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

& $Compiler -std=c11 -Wall -Wextra -Werror -I $include -c $source -o $outFile
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "C ABI header smoke compile succeeded: $outFile"
