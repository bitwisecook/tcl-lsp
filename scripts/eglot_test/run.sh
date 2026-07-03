#!/usr/bin/env bash
# Headless eglot reproducer for tcl-lsp issue #333.
#
# Drives a real eglot (GNU ELPA >= 1.20) against the native `tcl-lsp-server`
# binary and:
#   * diffs eglot's `face` text-properties between (a) an in-buffer edit
#     sequence and (b) a fresh didOpen of the same final content, and
#   * verifies our server-side eglot-compatibility mode
#     (`tclLsp.compatibility.eglot`) end-to-end: the advertised capability
#     shape flips and real eglot stops issuing `semanticTokens/full/delta`.
#
# This is NOT part of `make test` / CI gates — it needs network (to install
# eglot) and a real Emacs, and it exercises upstream eglot internals.
#
# Usage:    scripts/eglot_test/run.sh [LOGFILE]
# Env:      TCL_LSP_SERVER_BIN  path to a prebuilt tcl-lsp-server (skips build)
# Exit:     0=all scenarios pass, 1=at least one mismatch, 2=script error
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
elpa="$repo/tmp/elpa"
log="${1:-$repo/tmp/eglot_test/run.log}"
mkdir -p "$(dirname "$log")"

# 1. Ensure emacs is installed.
if ! command -v emacs >/dev/null 2>&1; then
  echo "emacs not installed; install with: sudo apt-get install -y emacs-nox" >&2
  exit 2
fi

# 1b. Locate (or build) the native tcl-lsp-server binary.
if [ -n "${TCL_LSP_SERVER_BIN:-}" ] && [ -x "${TCL_LSP_SERVER_BIN}" ]; then
  server_bin="$TCL_LSP_SERVER_BIN"
else
  server_bin=""
  for cand in "$repo/target/release/tcl-lsp-server" "$repo/target/debug/tcl-lsp-server"; do
    if [ -x "$cand" ]; then server_bin="$cand"; break; fi
  done
  if [ -z "$server_bin" ]; then
    echo "Building tcl-lsp-server (release)..." >&2
    # The workspace manifest is the repo-root Cargo.toml; run cargo from there
    # (there is no rust/Cargo.toml) with the -p form used elsewhere.
    if ! (cd "$repo" && cargo build --release -p tcl-lsp-server) >&2; then
      echo "cargo build failed; build tcl-lsp-server yourself and set TCL_LSP_SERVER_BIN" >&2
      exit 2
    fi
    server_bin="$repo/target/release/tcl-lsp-server"
  fi
fi
export TCL_LSP_SERVER_BIN="$server_bin"
echo "Using server binary: $server_bin" >&2

# 2. Build -L args from any existing ELPA dirs (used both for the
#    capability probe below and for the final harness invocation).
build_load_args() {
  load_args=()
  local d
  for d in "$elpa"/{eglot,jsonrpc,eldoc,flymake}-*; do
    if [ -d "$d" ]; then
      load_args+=(-L "$d")
    fi
  done
  return 0
}
build_load_args

# 3. Probe whether the eglot already on disk has the symbols we need
#    (`eglot-semantic-tokens-mode' was added in eglot 1.20 and the
#    bundled Emacs-29 eglot doesn't have it).  An older 1.x build
#    cached in tmp/elpa would skip a directory-based "is it installed"
#    check yet still fail the harness, so probe the symbol directly.
have_eglot=0
if [ ${#load_args[@]} -gt 0 ]; then
  if emacs -Q --batch "${load_args[@]}" \
       --eval "(unless (and (require 'eglot nil t) \
                            (fboundp 'eglot-semantic-tokens-mode)) \
                 (kill-emacs 1))" \
       >/dev/null 2>&1; then
    have_eglot=1
  fi
fi

if [ "$have_eglot" -eq 0 ]; then
  echo "Installing eglot from GNU ELPA into $elpa (need >= 1.20)..." >&2
  emacs -Q --batch \
    --eval "(setq package-user-dir \"$elpa\")" \
    --eval "(require 'package)" \
    --eval "(setq package-archives '((\"gnu\" . \"https://elpa.gnu.org/packages/\")))" \
    --eval "(setq package-install-upgrade-built-in t)" \
    --eval "(package-initialize)" \
    --eval "(package-refresh-contents)" \
    --eval "(package-install 'eglot)" >&2
  build_load_args
fi

export TCL_LSP_REPO="$repo"
cd "$repo"

# 4. Run.  Tee output to LOGFILE; print only the high-signal sections.
#
# `grep' returns 1 when no lines match — that can happen if emacs dies
# before producing any "==========" headers (e.g. .elc load error).
# Wrap it in `|| true' so a non-matching grep doesn't trip pipefail
# and bypass the explicit emacs-status capture below.
emacs -Q -batch \
  "${load_args[@]}" \
  --eval "(setq debug-on-error t)" \
  -l "$repo/scripts/eglot_test/test_issue333.el" 2>&1 | tee "$log" \
  | { grep -E "^(==========|  (text-equal|protocol|default |compat |PASS|FAIL|XFAIL|XPASS|--|pos=|edit=|reload=|[a-z][a-z0-9-]+ +(PASS|FAIL|XFAIL|XPASS)))" || true; }
status="${PIPESTATUS[0]}"
echo
echo "Full log: $log"
if [ "$status" -ne 0 ]; then
  echo "(emacs exited with status $status — see log for details)" >&2
fi
exit "$status"
