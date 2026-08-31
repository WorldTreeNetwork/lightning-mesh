#!/usr/bin/env bash
# Build Lightning Admin for Windows from the Linux host via the morphist-win11 VM.
# Copied from Papyrus scripts/windows-build/release.sh (same SSH/tar methods).
#
# Usage:
#   ./scripts/windows-build/release.sh                # full bundle (MSI + NSIS)
#   ./scripts/windows-build/release.sh smoke          # quick cargo build (debug)
#   ./scripts/windows-build/release.sh bootstrap      # one-time toolchain install
#   ./scripts/windows-build/release.sh survey         # diagnostics
#   ./scripts/windows-build/release.sh sync           # just sync source, don't build
#
# Env overrides:
#   WIN_HOST    SSH alias for the VM (default: win11)
#   WIN_USER    Windows username (default: A)
#   WIN_ROOT    Source root on the VM (default: C:\Users\A)
#   ADMIN       Path to admin/ on host (default: this script's repo admin/)
#   SKIP_SYNC=1 Skip source sync (reuse what's already on the VM)
#
# Why no scp anywhere: Windows OpenSSH wraps every channel through the
# DefaultShell (PowerShell), whose startup banner corrupts scp's protocol
# stream ("Received message too long"). We stream scripts via ssh stdin and
# pull artifacts with `tar` over ssh — same approach as sync, no temp files.

set -euo pipefail

WIN_HOST="${WIN_HOST:-win11}"
WIN_USER="${WIN_USER:-A}"
WIN_ROOT="${WIN_ROOT:-C:\\Users\\${WIN_USER}}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADMIN="${ADMIN:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

PS1_DIR="$SCRIPT_DIR/ps1"
ARTIFACTS="$ADMIN/dist-windows"
REMOTE_NAME="lightning-admin"

# --- helpers ---------------------------------------------------------------

log() { printf '\033[1;36m[release.sh]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[release.sh ERROR]\033[0m %s\n' "$*" >&2; exit 1; }

# Strip the PowerShell/sshd banner noise that prefixes every session.
ssh_clean() {
    grep -vE 'post-quantum|store now|may need to be upgraded|mise.*PowerShell|warning\.$|^is 5\.1' || true
}

# Run a .ps1 file on the VM. We base64 the script, ship it on the command
# line, and have PowerShell decode → write to a temp .ps1 → invoke. This
# avoids both scp (which the PowerShell banner corrupts) and `-Command -`
# (which parses stdin line-by-line and silently drops multi-statement blocks).
#
# Quoting trap: Windows OpenSSH uses PowerShell as DefaultShell, so the entire
# ssh command line is parsed by an OUTER PowerShell before reaching the inner
# powershell.exe -Command. That outer PowerShell interpolates `$var` inside
# double-quoted strings, so every $ that should reach the inner PowerShell
# unmodified must be escaped with PowerShell's escape character — backtick (`).
# In bash double-quotes that means we emit literal `` `$var ``, written as
# `\`\$var` (escape both the backtick and the dollar from bash).
run_ps_file() {
    local local_ps="$1"
    local b64
    b64=$(base64 -w0 "$local_ps")
    ssh "$WIN_HOST" "powershell -NoProfile -ExecutionPolicy Bypass -Command \"\`\$s = \`\$env:TEMP + '\\runner.ps1'; [System.IO.File]::WriteAllText(\`\$s, [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('${b64}'))); & \`\$s; \`\$rc = \`\$LASTEXITCODE; Remove-Item \`\$s -Force; exit \`\$rc\"" 2>&1 | ssh_clean
    return "${PIPESTATUS[0]}"
}

require_local() {
    [ -d "$1" ] || die "Missing source tree: $1"
}

# --- subcommands -----------------------------------------------------------

cmd_sync() {
    require_local "$ADMIN"

    local -a excludes=(
        --exclude=.git --exclude=target --exclude=node_modules
        --exclude=.svelte-kit --exclude=build --exclude=dist
        --exclude=dist-windows --exclude=.beads --exclude=.omc
        --exclude=.claude --exclude=src-tauri/target
    )

    local dest="${WIN_ROOT}\\${REMOTE_NAME}"
    log "syncing admin → $dest"
    local untar="\`\$d='${dest}'; New-Item -ItemType Directory -Force -Path \`\$d | Out-Null; Set-Location \`\$d; tar -xf -; Write-Host \"extracted to \`\$d\""
    ( cd "$ADMIN" && tar -cf - "${excludes[@]}" . ) | \
        ssh "$WIN_HOST" "powershell -NoProfile -ExecutionPolicy Bypass -Command \"${untar}\"" \
        2>&1 | ssh_clean
}

cmd_bootstrap() { run_ps_file "$PS1_DIR/bootstrap-toolchain.ps1"; }
cmd_survey()    { run_ps_file "$PS1_DIR/survey.ps1"; }

cmd_smoke() {
    [ "${SKIP_SYNC:-0}" = "1" ] || cmd_sync
    run_ps_file "$PS1_DIR/cargo-smoke.ps1"
}

cmd_bundle() {
    [ "${SKIP_SYNC:-0}" = "1" ] || cmd_sync
    run_ps_file "$PS1_DIR/tauri-bundle.ps1"
    cmd_fetch
}

# Pull artifacts by streaming a tar of the bundle dir back over ssh.
cmd_fetch() {
    log "fetching artifacts to $ARTIFACTS"
    mkdir -p "$ARTIFACTS"
    rm -rf "$ARTIFACTS/bundle"
    local remote="${WIN_ROOT}\\${REMOTE_NAME}\\src-tauri\\target\\release\\bundle"
    local tarcmd="if (Test-Path '${remote}') { Set-Location '${remote}\\..'; tar -cf - bundle } else { Write-Error 'no bundle dir' }"
    if ssh "$WIN_HOST" "powershell -NoProfile -ExecutionPolicy Bypass -Command \"${tarcmd}\"" 2>/dev/null | \
            tar -xf - -C "$ARTIFACTS"; then
        log "bundle pulled:"
        find "$ARTIFACTS/bundle" -type f -printf '  %p  (%s bytes)\n'
    else
        log "WARNING: no bundle found on VM (build may have failed)"
    fi
    local release="${WIN_ROOT}\\${REMOTE_NAME}\\src-tauri\\target\\release"
    local exe="${release}\\lightning-admin.exe"
    local exetar="if (Test-Path '${exe}') { Set-Location '${release}'; tar -cf - lightning-admin.exe }"
    ssh "$WIN_HOST" "powershell -NoProfile -ExecutionPolicy Bypass -Command \"${exetar}\"" 2>/dev/null | \
        tar -xf - -C "$ARTIFACTS" 2>/dev/null || true
}

# --- dispatch --------------------------------------------------------------

cmd="${1:-bundle}"
case "$cmd" in
    bundle|sync|smoke|bootstrap|survey|fetch) "cmd_$cmd" ;;
    -h|--help|help)
        sed -n '2,18p' "$0"
        exit 0
        ;;
    *) die "unknown subcommand: $cmd (try --help)" ;;
esac
