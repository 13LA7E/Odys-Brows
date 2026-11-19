# One verify command for Windows (PHASES.md P1-06).
# Runs formatting, linting, tests, JS checks, token checks, and visual checks.
#   powershell -ExecutionPolicy Bypass -File tools\check.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

Write-Host "=== 1/6: cargo fmt --check ==="
& cargo fmt --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo fmt --check failed. Run 'cargo fmt' to fix."
    exit 1
}

Write-Host "=== 2/6: cargo clippy ==="
& cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo clippy failed with warnings."
    exit 1
}

Write-Host "=== 3/6: cargo test ==="
& cargo test
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo test failed."
    exit 1
}

Write-Host "=== 4/6: JS syntax check (node --check) ==="
$jsFiles = Get-ChildItem -Path (Join-Path $root "ui\*.js")
if (-not $jsFiles) {
    Write-Error "No JS files found in ui/"
    exit 1
}
foreach ($file in $jsFiles) {
    & node --check $file.FullName
    if ($LASTEXITCODE -ne 0) {
        Write-Error "JS syntax check failed for $($file.Name)"
        exit 1
    }
}
Write-Host "Checked $($jsFiles.Count) JS files."

Write-Host "=== 5/6: Token check (no raw hex colours outside tokens.css) ==="
$cssFiles = Get-ChildItem -Path (Join-Path $root "ui\*.css") | Where-Object { $_.Name -ne 'tokens.css' }
$badHex = @()
foreach ($file in $cssFiles) {
    $matches = Select-String -Path $file.FullName -Pattern '#[0-9a-fA-F]{3,8}'
    if ($matches) {
        $badHex += $matches
    }
}
if ($badHex.Count -gt 0) {
    Write-Error "Raw hex colours found outside ui/tokens.css:`n$($badHex | Out-String)"
    exit 1
}
Write-Host "Token check passed: no raw hex colours found."

Write-Host "=== 6/6: Visual check (fixture mode) ==="
$hasPlaywright = $false
try {
    & python -c "import playwright" 2>$null
    if ($LASTEXITCODE -eq 0) {
        $hasPlaywright = $true
    }
} catch {
    $hasPlaywright = $false
}

if ($hasPlaywright) {
    & python -u (Join-Path $root "tools\visual_check.py") --report-only
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Visual check failed."
        exit 1
    }
} else {
    Write-Host "Playwright not installed; skipping visual check."
}

Write-Host ""
Write-Host "All checks PASSED."
exit 0
