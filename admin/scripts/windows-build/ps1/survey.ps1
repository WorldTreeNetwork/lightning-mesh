# Diagnostics: report toolchain state. Used to check whether the VM is ready
# to build, or to debug a "missing tool" failure from release.sh.
$tools = @("winget","git","rustc","cargo","rustup","node","bun","npm","mise","tar","cl","link")
foreach ($t in $tools) {
    $path = (Get-Command $t -ErrorAction SilentlyContinue).Source
    if ($path) { Write-Host "$t -> $path" } else { Write-Host "$t MISSING" }
}
Write-Host ""
Write-Host "=== OS / arch ==="
Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, OSArchitecture | Format-List

Write-Host "=== WebView2 ==="
$wv2 = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if ($wv2) { Write-Host "WebView2 installed: $($wv2.pv)" } else { Write-Host "WebView2 NOT installed (Tauri needs it at runtime)" }

Write-Host "=== source trees ==="
foreach ($p in @("C:\Users\A\lightning-admin","C:\Users\A\Papyrus")) {
    if (Test-Path $p) {
        $count = (Get-ChildItem $p -ErrorAction SilentlyContinue | Measure-Object).Count
        Write-Host ("  {0}  ({1} top-level entries)" -f $p, $count)
    } else {
        Write-Host "  $p  MISSING"
    }
}
