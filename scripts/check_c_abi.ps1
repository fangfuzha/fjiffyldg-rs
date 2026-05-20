param(
    [string]$CCompiler = 'cc',
    [string]$CppCompiler = 'g++'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$include = Join-Path $repoRoot 'include'
$outDir = Join-Path $repoRoot 'target/c-abi-smoke'
$releaseDir = Join-Path $repoRoot 'target/release'
$cSource = Join-Path $repoRoot 'tests/c_smoke.c'
$cppSource = Join-Path $repoRoot 'tests/cpp_smoke.cpp'
$cOutFile = Join-Path $outDir 'c_smoke.o'
$cppOutFile = Join-Path $outDir 'cpp_smoke.o'
$cExe = Join-Path $outDir 'c_smoke.exe'
$cppExe = Join-Path $outDir 'cpp_smoke.exe'
$inputFile = Join-Path $outDir 'input.txt'
$generateHeader = Join-Path $PSScriptRoot 'generate_c_header.ps1'

& pwsh -File $generateHeader -Verify
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

New-Item -ItemType Directory -Force -Path $outDir | Out-Null
# Write UTF-8 bytes without BOM to avoid adding EF BB BF which affects byte-based APIs
$s = "alpha`nbeta`ngamma`n"
$bytes = [System.Text.Encoding]::UTF8.GetBytes($s)
[System.IO.File]::WriteAllBytes($inputFile, $bytes)

& $CCompiler -std=c11 -Wall -Wextra -Werror -I $include -c $cSource -o $cOutFile
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $CppCompiler -std=c++17 -Wall -Wextra -Werror -I $include -c $cppSource -o $cppOutFile
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

cargo build --release
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$dynamicLibrary = Join-Path $releaseDir 'fjiffyldg.dll'
if (-not (Test-Path $dynamicLibrary)) {
    $dynamicLibrary = Join-Path $releaseDir 'libfjiffyldg.so'
}
if (-not (Test-Path $dynamicLibrary)) {
    $dynamicLibrary = Join-Path $releaseDir 'libfjiffyldg.dylib'
}
if (-not (Test-Path $dynamicLibrary)) {
    throw "Could not find a release dynamic library in $releaseDir"
}

& $CCompiler -std=c11 -Wall -Wextra -Werror -DFJIFFYLDG_SMOKE_MAIN -I $include $cSource $dynamicLibrary -o $cExe
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $CppCompiler -std=c++17 -Wall -Wextra -Werror -DFJIFFYLDG_SMOKE_MAIN -I $include $cppSource $dynamicLibrary -o $cppExe
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$oldPath = $env:PATH
try {
    $env:PATH = "$releaseDir$([System.IO.Path]::PathSeparator)$oldPath"
    & $cExe $inputFile
    if ($LASTEXITCODE -ne 0) {
        Write-Error "C ABI smoke executable failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    & $cppExe $inputFile
    if ($LASTEXITCODE -ne 0) {
        Write-Error "C++ ABI smoke executable failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
}
finally {
    $env:PATH = $oldPath
}

Write-Host "C/C++ ABI smoke compile and run succeeded: $cOutFile, $cppOutFile, $cExe, $cppExe"
