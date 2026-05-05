#!/usr/bin/env bash
# Headless eglot reproducer for tcl-lsp issue #333.
#
# Drives a real eglot 1.23 (GNU ELPA) against `uv run python -m lsp` and
# diffs eglot's `face` text-properties between (a) an in-buffer edit
# sequence with delta semantic-token updates and (b) a fresh didOpen of
# the same final content.  Mismatches reproduce the bug.
#
# Usage:    scripts/eglot_test/run.sh [LOGFILE]
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

# 2. Ensure eglot 1.23+ from GNU ELPA (the one bundled with Emacs 29 has
#    no semantic-tokens support).
if ! ls "$elpa"/eglot-1.* >/dev/null 2>&1; then
  echo "Installing eglot from GNU ELPA into $elpa..." >&2
  emacs -Q --batch \
    --eval "(setq package-user-dir \"$elpa\")" \
    --eval "(require 'package)" \
    --eval "(setq package-archives '((\"gnu\" . \"https://elpa.gnu.org/packages/\")))" \
    --eval "(setq package-install-upgrade-built-in t)" \
    --eval "(package-initialize)" \
    --eval "(package-refresh-contents)" \
    --eval "(package-install 'eglot)" >&2
fi

# 3. Build -L args so the ELPA eglot beats the built-in one.
load_args=()
for d in "$elpa"/{eglot,jsonrpc,eldoc,flymake}-*; do
  [ -d "$d" ] && load_args+=(-L "$d")
done

export TCL_LSP_REPO="$repo"
cd "$repo"

# 4. Run.  Tee output to LOGFILE; print only the high-signal sections.
emacs -Q -batch \
  "${load_args[@]}" \
  --eval "(setq debug-on-error t)" \
  -l "$repo/scripts/eglot_test/test_issue333.el" 2>&1 | tee "$log" \
  | grep -E "^(==========|  (text-equal|PASS|FAIL|--|pos=|edit=|reload=|[a-z][a-z0-9-]+ +(PASS|FAIL)))"
status="${PIPESTATUS[0]}"
echo
echo "Full log: $log"
exit "$status"
