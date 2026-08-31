# Quick smoke test: cargo build (debug) of just the src-tauri crate.
# Faster than tauri-bundle.ps1 — use this when iterating on Rust changes.

$ErrorActionPreference = "Continue"
function Log($m) { Write-Host ("[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $m) }

Log "=== cargo-smoke START ==="
$env:Path = "$env:USERPROFILE\.cargo\bin;${env:ProgramFiles}\Git\bin;${env:ProgramFiles}\Git\cmd;$env:Path"

Log ("rustc: " + ((& rustc --version) 2>&1))
Log ("cargo: " + ((& cargo --version) 2>&1))

Set-Location C:\Users\A\lightning-admin\src-tauri
Log "cwd: $(Get-Location)"

& cargo build 2>&1 | ForEach-Object { Log $_ }
$rc = $LASTEXITCODE
Log "cargo build exit: $rc"

if ($rc -eq 0) {
    Log "=== cargo-smoke OK ==="
    Get-ChildItem C:\Users\A\lightning-admin\src-tauri\target\debug\*.exe -ErrorAction SilentlyContinue |
        ForEach-Object { Log "produced: $($_.FullName) ($($_.Length) bytes)" }
} else {
    Log "=== cargo-smoke FAILED ==="
}
exit $rc
