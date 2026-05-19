param(
    [string]$CCompiler = "cc",
    [string]$CppCompiler = "g++"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$include = Join-Path $repoRoot "include"
$outDir = Join-Path $repoRoot "target/c-abi-smoke"
$cSource = Join-Path $repoRoot "tests/c_smoke.c"
$cppSource = Join-Path $repoRoot "tests/cpp_smoke.cpp"
$cOutFile = Join-Path $outDir "c_smoke.o"
$cppOutFile = Join-Path $outDir "cpp_smoke.o"
$generateHeader = Join-Path $PSScriptRoot "generate_c_header.ps1"

& pwsh -File $generateHeader -Verify
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

& $CCompiler -std=c11 -Wall -Wextra -Werror -I $include -c $cSource -o $cOutFile
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $CppCompiler -std=c++17 -Wall -Wextra -Werror -I $include -c $cppSource -o $cppOutFile
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "C/C++ ABI header smoke compile succeeded: $cOutFile, $cppOutFile"
