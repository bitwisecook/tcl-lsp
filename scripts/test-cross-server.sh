#!/usr/bin/env bash
#
# Smoke-test cross-compiled `tcl-lsp-server` binaries with a minimal LSP
# initialize/shutdown handshake over stdio.
#
#   scripts/test-cross-server.sh            # test whatever is already built
#   scripts/test-cross-server.sh --build    # `make server-cross-build` first
#
# Host-arch binaries run natively; foreign Linux arches run under QEMU user-mode
# (qemu-<arch>, with the matching cross sysroot).  Targets that cannot run on
# this host (e.g. Windows .exe on Linux, arm64 with no emulator) are SKIPPED
# loudly — never silently passed.  Exit status is non-zero only on a genuine
# handshake failure of a runnable binary.
#
# The target list mirrors SERVER_TARGET_MAP in the Makefile.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# triple|os|arch|qemu|sysroot   (qemu/sysroot blank when not applicable)
TARGETS=(
	"x86_64-apple-darwin|darwin|x86_64||"
	"aarch64-apple-darwin|darwin|arm64||"
	"x86_64-unknown-linux-gnu|linux|x86_64|qemu-x86_64|"
	"aarch64-unknown-linux-gnu|linux|aarch64|qemu-aarch64|/usr/aarch64-linux-gnu"
	"riscv64gc-unknown-linux-gnu|linux|riscv64|qemu-riscv64|/usr/riscv64-linux-gnu"
	"x86_64-pc-windows-msvc|windows|x86_64||"
	"aarch64-pc-windows-msvc|windows|arm64||"
)

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
case "$HOST_OS" in
	Linux) HOST_OS=linux ;;
	Darwin) HOST_OS=darwin ;;
	*) HOST_OS=windows ;;
esac

# A timeout wrapper guards against a hung server.  GNU coreutils ships
# `timeout`; macOS has it only as `gtimeout` (brew coreutils) or not at all —
# fall back to running without a wrapper (stdin EOF makes the server exit).
TIMEOUT=()
if command -v timeout >/dev/null 2>&1; then
	TIMEOUT=(timeout 30)
elif command -v gtimeout >/dev/null 2>&1; then
	TIMEOUT=(gtimeout 30)
fi

if [[ "${1:-}" == "--build" ]]; then
	echo "==> Building host targets first (make server-cross-build)"
	make server-cross-build
fi

# Print a single framed JSON-RPC message: Content-Length header + CRLFs + body.
frame() {
	local body="$1"
	printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
}

# The handshake: a single `initialize` request, then stdin EOF.  A healthy
# server answers with a result containing `capabilities`, then exits on EOF.
# (Sending `exit` in the same pipe would cancel the in-flight initialize, so we
# keep the probe to the one request the smoke test cares about.)
handshake_input() {
	frame '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
}

# Run one binary (optionally via a runner) through the handshake; assert the
# initialize result came back.
run_handshake() {
	local label="$1"; shift
	local out
	# Branch on array length: "${TIMEOUT[@]}" on an empty array trips `set -u`
	# under macOS's bash 3.2, so only expand it when populated.
	if [ "${#TIMEOUT[@]}" -gt 0 ]; then
		out="$(handshake_input | "${TIMEOUT[@]}" "$@" 2>/dev/null || true)"
	else
		out="$(handshake_input | "$@" 2>/dev/null || true)"
	fi
	if echo "$out" | grep -q '"capabilities"'; then
		echo "  PASS  $label"
		return 0
	fi
	echo "  FAIL  $label (no initialize result)"
	return 1
}

fail=0
ran=0
for entry in "${TARGETS[@]}"; do
	IFS='|' read -r triple os arch qemu sysroot <<<"$entry"
	exe="tcl-lsp-server"
	[[ "$os" == windows ]] && exe="tcl-lsp-server.exe"
	bin="target/$triple/release/$exe"

	if [[ ! -f "$bin" ]]; then
		echo "  ----  $triple (not built — skipping)"
		continue
	fi

	# Native arch on the same OS?
	native=0
	if [[ "$os" == "$HOST_OS" ]]; then
		if [[ "$arch" == "$HOST_ARCH" ]] || { [[ "$os" == darwin ]] && [[ "$arch" == arm64 && "$HOST_ARCH" == arm64 ]]; }; then
			native=1
		fi
	fi

	if [[ "$native" == 1 ]]; then
		ran=$((ran+1))
		run_handshake "$triple (native)" "./$bin" || fail=1
	elif [[ "$HOST_OS" == linux && "$os" == linux && -n "$qemu" ]] && command -v "$qemu" >/dev/null 2>&1; then
		ran=$((ran+1))
		if [[ -n "$sysroot" ]]; then
			run_handshake "$triple (qemu)" "$qemu" -L "$sysroot" "./$bin" || fail=1
		else
			run_handshake "$triple (qemu)" "$qemu" "./$bin" || fail=1
		fi
	else
		echo "  SKIP  $triple (cannot run on $HOST_OS/$HOST_ARCH — built OK, not executed)"
	fi
done

echo "==> Smoke test: ran $ran binary handshake(s)"
if [[ "$ran" == 0 ]]; then
	echo "WARNING: no binaries were executed on this host (built-only targets were skipped)."
fi
exit "$fail"
