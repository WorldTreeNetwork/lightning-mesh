# Bootstrap full Windows build toolchain for Lightning Admin (Tauri/Rust).
# Copied from Papyrus scripts/windows-build/ps1/bootstrap-toolchain.ps1.
# Idempotent — skips components already installed. Designed to be invoked over
# SSH from release.sh; runs unattended.
#
# Installs:
#   - Visual Studio Build Tools 2022 (MSVC x64 + Windows 11 SDK 22621)
#   - rustup + stable-x86_64-pc-windows-msvc toolchain
#   - Git for Windows (winget, best-effort; build does not require it)
#   - Bun (pinned) — drives the tauri CLI
#   - Node.js — runs the vite frontend build (bun can't load the vite config)

$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"  # speeds up Invoke-WebRequest

function Log($msg) {
    Write-Host ("[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $msg)
}

Log "=== bootstrap-toolchain START ==="

# ---------- 1. Visual Studio Build Tools (MSVC) ----------
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$haveMSVC = $false
if (Test-Path $vswhere) {
    $inst = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    if ($inst) { $haveMSVC = $true; Log "MSVC already installed at: $inst" }
}
if (-not $haveMSVC) {
    Log "Installing Visual Studio Build Tools 2022 (slow: 5-10 min)..."
    $vsExe = "$env:TEMP\vs_BuildTools.exe"
    Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_BuildTools.exe" -OutFile $vsExe -UseBasicParsing
    $args = @(
        "--quiet","--wait","--norestart","--nocache",
        "--add","Microsoft.VisualStudio.Workload.VCTools",
        "--add","Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "--add","Microsoft.VisualStudio.Component.Windows11SDK.22621",
        "--includeRecommended"
    )
    $p = Start-Process -FilePath $vsExe -ArgumentList $args -Wait -PassThru
    Log "  installer exit code: $($p.ExitCode)"
    if ($p.ExitCode -ne 0 -and $p.ExitCode -ne 3010) {
        Log "  WARNING: MSVC installer returned $($p.ExitCode); continuing."
    }
}

# ---------- 2. Rustup / Rust ----------
$rustupExe = "$env:USERPROFILE\.cargo\bin\rustup.exe"
if (-not (Test-Path $rustupExe)) {
    Log "Installing rustup..."
    $rustupInit = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit -UseBasicParsing
    & $rustupInit -y --default-toolchain stable-x86_64-pc-windows-msvc --profile default
    Log "  rustup install exit: $LASTEXITCODE"
} else {
    Log "rustup already installed."
}
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
& $rustupExe default stable-x86_64-pc-windows-msvc 2>&1 | Out-Null
& $rustupExe target add x86_64-pc-windows-msvc 2>&1 | Out-Null

# ---------- 3. Git ----------
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Log "Installing Git for Windows via winget..."
    winget install --id Git.Git --silent --accept-source-agreements --accept-package-agreements 2>&1 | Out-Null
} else {
    Log "git already installed."
}

# ---------- 4. Bun (pinned) + Node ----------
# bun's Windows module interop can fail vite config load, so bun drives the
# tauri CLI and Node runs the vite frontend build (see tauri-bundle.ps1).
$bunVersion = "1.3.11"
$bunExe = "$env:USERPROFILE\.bun\bin\bun.exe"
$haveBun = (Test-Path $bunExe) -and (((& $bunExe --version) 2>$null) -eq $bunVersion)
if (-not $haveBun) {
    Log "Installing bun $bunVersion ..."
    & ([scriptblock]::Create((Invoke-RestMethod https://bun.sh/install.ps1))) -Version $bunVersion
} else {
    Log "bun $bunVersion already installed."
}

$nodeVersion = "v22.12.0"
$nodeDir = "$env:USERPROFILE\node"
if (-not (Test-Path "$nodeDir\node.exe")) {
    Log "Installing Node $nodeVersion ..."
    $nodeZip = "$env:TEMP\node.zip"
    Invoke-WebRequest -UseBasicParsing -Uri "https://nodejs.org/dist/$nodeVersion/node-$nodeVersion-win-x64.zip" -OutFile $nodeZip
    if (Test-Path $nodeDir) { Remove-Item -Recurse -Force $nodeDir }
    Expand-Archive -Path $nodeZip -DestinationPath $env:USERPROFILE -Force
    Rename-Item "$env:USERPROFILE\node-$nodeVersion-win-x64" $nodeDir
} else {
    Log "Node already installed."
}

# ---------- 5. Verify ----------
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.bun\bin;$nodeDir;$env:Path;${env:ProgramFiles}\Git\bin"
Log "=== VERIFY ==="
Log ("rustc:  " + ((& rustc --version) 2>&1))
Log ("cargo:  " + ((& cargo --version) 2>&1))
Log ("rustup: " + ((& rustup show active-toolchain) 2>&1))
Log ("bun:    " + ((& $bunExe --version) 2>&1))
Log ("node:   " + ((& "$nodeDir\node.exe" --version) 2>&1))
$gitV = (Get-Command git -ErrorAction SilentlyContinue)
if ($gitV) { Log ("git:    " + ((& git --version) 2>&1)) } else { Log "git: NOT ON PATH (optional; build does not require it)" }

if (Test-Path $vswhere) {
    $vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsRoot) {
        $cl = Get-ChildItem -Path "$vsRoot\VC\Tools\MSVC" -Filter cl.exe -Recurse -ErrorAction SilentlyContinue |
              Where-Object { $_.FullName -match 'Hostx64\\x64\\cl\.exe$' } | Select-Object -First 1
        if ($cl) { Log "cl.exe:  $($cl.FullName)" } else { Log "cl.exe NOT found under $vsRoot" }
    }
}

Log "=== bootstrap-toolchain DONE ==="
