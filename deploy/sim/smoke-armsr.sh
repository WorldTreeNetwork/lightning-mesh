#!/usr/bin/env bash
# TCG smoke: OpenWrt armsr/armv8 (-M virt) runs the *shipped* aarch64 musl
# mjolnir-meshd. Not the daily q35 lab. Does not start vwifi.
set -euo pipefail
RELEASE="${OPENWRT_RELEASE:-25.12.5}"
KERNEL_NAME="openwrt-${RELEASE}-armsr-armv8-generic-initramfs-kernel.bin"
KERNEL_URL="https://downloads.openwrt.org/releases/${RELEASE}/targets/armsr/armv8/${KERNEL_NAME}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CACHE="${REPO}/deploy/sim/imagebuilder"
KERNEL="${CACHE}/${KERNEL_NAME}"
MESH="${REPO}/deploy/openwrt/mjolnir-meshd-aarch64"
SHARE="${CACHE}/armsr-share"
TIMEOUT="${ARMSR_TIMEOUT:-180}"

[ -x "$MESH" ] || { echo "missing $MESH — deploy/openwrt/build.sh" >&2; exit 1; }
command -v qemu-system-aarch64 >/dev/null || { echo "qemu-system-aarch64 not installed" >&2; exit 1; }
mkdir -p "$CACHE" "$SHARE"
cp -f "$MESH" "$SHARE/mjolnir-meshd-aarch64"
chmod +x "$SHARE/mjolnir-meshd-aarch64"

if [ ! -f "$KERNEL" ]; then
	echo ">> fetching $KERNEL_URL"
	curl -fL --retry 3 -o "$KERNEL" "$KERNEL_URL"
fi

export SMOKE_KERNEL="$KERNEL" SMOKE_SHARE="$SHARE" SMOKE_TIMEOUT="$TIMEOUT"
python3 - <<'PY'
import os, pty, select, sys, time

kernel = os.environ["SMOKE_KERNEL"]
share = os.environ["SMOKE_SHARE"]
timeout = int(os.environ["SMOKE_TIMEOUT"])
cmd = [
    "qemu-system-aarch64",
    "-nographic",
    "-machine", "virt",
    "-cpu", "cortex-a53",
    "-m", "512",
    "-smp", "2",
    "-kernel", kernel,
    "-netdev", "user,id=n0",
    "-device", "virtio-net-pci,netdev=n0",
    "-fsdev", f"local,id=host,path={share},security_model=none",
    "-device", "virtio-9p-pci,fsdev=host,mount_tag=host",
]
pid, fd = pty.fork()
if pid == 0:
    os.execvp(cmd[0], cmd)
buf = b""
deadline = time.time() + timeout
stage = "boot"

def send(s: str) -> None:
    os.write(fd, s.encode() + b"\n")

try:
    while time.time() < deadline:
        r, _, _ = select.select([fd], [], [], 1.0)
        if not r:
            continue
        chunk = os.read(fd, 8192)
        if not chunk:
            break
        buf += chunk
        sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.flush()
        low = buf.lower()
        if stage == "boot" and (b"press enter" in low or b"/#" in buf[-80:] or b"root@" in buf[-120:]):
            send("")
            time.sleep(0.3)
            send("uname -m")
            stage = "uname"
            mark = len(buf)
        elif stage == "uname" and b"aarch64" in buf[mark:]:
            send("mkdir -p /tmp/host && mount -t 9p -o trans=virtio,version=9p2000.L host /tmp/host")
            send("/tmp/host/mjolnir-meshd-aarch64 --help >/tmp/meshd.help; echo END_ID:$?")
            stage = "id"
            mark = len(buf)
        elif stage == "id" and b"END_ID:0" in buf[mark:]:
            os.kill(pid, 15)
            time.sleep(0.4)
            print("\n>> armsr smoke: aarch64 guest ran shipped mjolnir-meshd --help", file=sys.stderr)
            sys.exit(0)
    print(">> armsr smoke timed out; last 2k:", file=sys.stderr)
    sys.stderr.buffer.write(buf[-2000:])
    os.kill(pid, 9)
    sys.exit(1)
except Exception:
    try:
        os.kill(pid, 9)
    except OSError:
        pass
    raise
PY
