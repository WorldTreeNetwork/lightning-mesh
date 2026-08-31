# Build the full Tauri release bundle (MSI + NSIS).
# Run on the VM via SSH; expects source already synced to C:\Users\A\lightning-admin.
# Methods copied from Papyrus scripts/windows-build/ps1/tauri-bundle.ps1.

$ErrorActionPreference = "Continue"

function Log($m) { Write-Host ("[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $m) }

Log "=== tauri-bundle START ==="

# Fresh SSH sessions don't inherit PATH from installers, so reassemble it.
$mise = "$env:USERPROFILE\AppData\Local\mise"
$bunBin = Get-ChildItem "$mise\installs\bun" -Directory -ErrorAction SilentlyContinue |
          Sort-Object Name -Descending | Select-Object -First 1
$bunPath = if ($bunBin) { "$($bunBin.FullName)\bin" } else { "" }
$pathParts = @(
    "$env:USERPROFILE\node",
    "$env:USERPROFILE\.cargo\bin",
    "${env:ProgramFiles}\Git\bin",
    "${env:ProgramFiles}\Git\cmd",
    "$env:USERPROFILE\.bun\bin",
    $bunPath,
    "$mise\shims",
    "${env:LOCALAPPDATA}\Microsoft\WinGet\Packages\jdx.mise_Microsoft.Winget.Source_8wekyb3d8bbwe\mise\bin"
) | Where-Object { $_ }
$env:Path = ($pathParts -join ";") + ";" + $env:Path

Set-Location C:\Users\A\lightning-admin
Log "cwd:   $(Get-Location)"
Log ("bun:   " + ((& bun --version) 2>&1))
Log ("cargo: " + ((& cargo --version) 2>&1))
Log ("node:  " + ((& node --version) 2>&1))

# Install JS deps if node_modules absent or package.json newer.
$needsInstall = -not (Test-Path "node_modules\.bin")
if (-not $needsInstall) {
    $pkg = (Get-Item package.json).LastWriteTime
    $stamp = (Get-Item "node_modules\.bin" -ErrorAction SilentlyContinue).LastWriteTime
    if ($pkg -gt $stamp) { $needsInstall = $true }
}
if ($needsInstall) {
    Log "Running: bun install ..."
    & bun install 2>&1 | ForEach-Object { Log $_ }
    if ($LASTEXITCODE -ne 0) { Log "=== bun install FAILED ==="; exit $LASTEXITCODE }
} else {
    Log "node_modules looks current, skipping bun install."
}

# bun's Windows module interop can break vite config load, so the frontend
# must build under Node. Override tauri's beforeBuildCommand (default
# `bun run build`) to invoke vite via node; the rest still runs under bun/cargo.
$override = '{"build":{"beforeBuildCommand":"node node_modules/vite/bin/vite.js build"}}'
$overrideFile = "$env:TEMP\tauri-win-override.json"
Set-Content -Path $overrideFile -Value $override -Encoding ascii
Log "Running: bun tauri build (frontend via node) ..."
& bun tauri build --config $overrideFile 2>&1 | ForEach-Object { Log $_ }
$rc = $LASTEXITCODE
Log "tauri build exit: $rc"

if ($rc -eq 0) {
    Log "=== tauri-bundle OK ==="
    Get-ChildItem -Path C:\Users\A\lightning-admin\src-tauri\target\release\bundle -Recurse -ErrorAction SilentlyContinue |
        Where-Object { -not $_.PSIsContainer } |
        ForEach-Object { Log ("  {0} ({1:N1} MB)" -f $_.FullName, ($_.Length/1MB)) }
} else {
    Log "=== tauri-bundle FAILED ==="
}
exit $rc
