# Build Constellations Windows Installer (P9-01, EDGE_CASES §13).
# Staging CEF binaries, UI assets, and packing with NSIS.

$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")

Write-Host "=== Step 1: Build release binary ==="
Push-Location $root
try {
    & cargo build --release -p shell
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Release build failed."
        exit 1
    }
} finally {
    Pop-Location
}

Write-Host "=== Step 2: Stage installer payload ==="
$outDir = Join-Path $root "out\installer"
$payloadDir = Join-Path $outDir "payload"

if (Test-Path $payloadDir) {
    Remove-Item -Recurse -Force $payloadDir
}
New-Item -ItemType Directory -Path $payloadDir -Force | Out-Null

# 1. Main executable
Copy-Item (Join-Path $root "target\release\constellations.exe") -Destination $payloadDir

# 2. CEF Runtime DLLs & resources
$cefDirs = Get-ChildItem (Join-Path $root "target\release\build\cef-dll-sys-*\out\cef_windows_x86_64") -Directory
if (-not $cefDirs) {
    $cefDirs = Get-ChildItem (Join-Path $root "target\debug\build\cef-dll-sys-*\out\cef_windows_x86_64") -Directory
}
if ($cefDirs) {
    $cefDir = $cefDirs[0].FullName
    Write-Host "Found CEF binaries at: $cefDir"
    # Runtime files only (AUDIT-24): the SDK folders, import library and bootstrap tools are build-time.
    $devOnly = @('include', 'cmake', 'libcef_dll', 'CMakeLists.txt', 'archive.json', 'bootstrap.exe', 'bootstrapc.exe', '*.lib')
    Get-ChildItem $cefDir -Force | Where-Object {
        $n = $_.Name
        -not ($devOnly | Where-Object { $n -like $_ })
    } | ForEach-Object { Copy-Item $_.FullName -Destination $payloadDir -Recurse -Force }
} else {
    Write-Warning "CEF runtime files not located automatically. Ensure libcef.dll is in the payload."
}

# 3. UI Assets
$uiDest = Join-Path $payloadDir "ui"
Copy-Item (Join-Path $root "ui") -Destination $uiDest -Recurse -Force

Write-Host "Payload staged at $payloadDir"

Write-Host "=== Step 3: Run NSIS compiler ==="
$nsisExe = Get-Command makensis.exe -ErrorAction SilentlyContinue
if (-not $nsisExe) {
    # Check default install path
    $candidate = "C:\Program Files (x86)\NSIS\makensis.exe"
    if (Test-Path $candidate) {
        $nsisExe = $candidate
    }
}

if ($nsisExe) {
    & $nsisExe (Join-Path $PSScriptRoot "installer.nsi")
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Installer created successfully in out\installer!"
    } else {
        Write-Error "makensis failed."
        exit 1
    }
} else {
    Write-Host "makensis not found on PATH. The installer script is ready at tools\installer\installer.nsi."
    Write-Host "Install NSIS (https://nsis.sourceforge.io/) or let GitHub Actions compile the installer."
}
