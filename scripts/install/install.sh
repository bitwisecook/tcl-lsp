#!/usr/bin/env sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# tcl-lsp installer for the released `tcl` / `f5` CLIs, the MCP server,
# and the Claude Code skills bundle.
#
# Detects the host OS (macOS, Debian/Ubuntu, RHEL/CentOS/Fedora, Arch,
# Alpine), checks for a usable Python 3.10+, installs dependencies
# through the native package manager when missing, prompts for an
# install directory (scanning $PATH for writable candidates), and
# optionally writes shell completion.
#
# Before prompting, looks for an existing `tcl` / `f5` on $PATH and
# offers to update in place. Does NOT use sudo: if a step needs root
# (installing OS packages, writing to /usr/local/bin, ...) the script
# stops and prints how to re-run as root, or how to avoid needing it.
#
# All prompts run up front, before any download or install action — so
# you can answer the questions and then walk away.
#
# If the `claude` or `codex` CLI is detected (or ~/.claude / ~/.codex
# exists), offers to install the MCP server and — for Claude Code —
# the skills zip from the same GitHub release.
#
# Downloaded release artefacts are verified against the release's
# SHA256SUMS file (and, if `cosign` is installed and the release
# publishes one, the SHA256SUMS.cosign.bundle). A missing SUMS file
# aborts the install — set TCL_LSP_NO_VERIFY=1 to install unverified
# (only do this when the network path is trustworthy end-to-end).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install/install.sh | sh
#
# Environment overrides:
#   TCL_LSP_VERSION         - release tag to install (default: latest)
#   TCL_LSP_PREFIX          - install dir (default: prompt; non-interactive: ~/.local/bin)
#   TCL_LSP_REPO            - GitHub owner/repo (default: bitwisecook/tcl-lsp)
#   TCL_LSP_ONLY            - "tcl", "f5", or "both" (default: both)
#   TCL_LSP_OS              - bypass /etc/os-release: debian|rhel|fedora|arch|alpine|macos
#   TCL_LSP_NO_DEPS         - 1 to skip OS-package install (Python, curl, unzip)
#   TCL_LSP_NO_PATH         - 1 to skip PATH/rc modification
#   TCL_LSP_NO_COMP         - 1 to skip shell completion
#   TCL_LSP_NO_MCP          - 1 to skip MCP-server install for AI clients
#   TCL_LSP_NO_SKILLS       - 1 to skip Claude Code skills install
#   TCL_LSP_NO_CLAUDE       - 1 to ignore Claude Code even if detected
#   TCL_LSP_NO_CODEX        - 1 to ignore Codex even if detected
#   TCL_LSP_NO_VERIFY            - 1 to install without SHA256SUMS verification
#   TCL_LSP_REQUIRE_COSIGN       - 1 to fail when cosign signature is missing/invalid
#   TCL_LSP_ALLOW_INSECURE_WGET  - 1 to allow wget without --https-only (DANGEROUS)
#   TCL_LSP_ALLOW_TLS12          - 1 to skip the TLS 1.3 + HTTP/2 attempt and go
#                                  straight to TLS 1.2 + HTTP/1.1 (e.g. behind a
#                                  proxy that doesn't negotiate TLS 1.3)
#
# Proxy: curl and wget both honour the standard http_proxy / https_proxy /
# no_proxy environment variables. On macOS, the system proxy (System
# Settings > Network > Proxies) is NOT auto-discovered — export the env
# vars manually if you need the proxy.
#   TCL_LSP_ASSUME_YES      - 1 to answer "yes" non-interactively
#   TCL_LSP_ASSUME_NO       - 1 to answer "no" non-interactively
#   TCL_LSP_QUIET           - 1 to suppress the per-prompt stderr breadcrumb
#   TCL_LSP_TRACE           - 1 to print extra trace lines for downloads etc.
#   TCL_LSP_SUFFIX          - suffix for installed binaries (e.g. "-lsp"; default: empty)
#
# Every interactive prompt prints a `ask: [kind] question -> answer`
# breadcrumb to stderr so an aborted run leaves a visible record in
# the user's scrollback. Set TCL_LSP_QUIET=1 to suppress.

set -eu
# IFS=\n only — defence against a poisoned caller IFS. Loops that need
# whitespace splitting save and restore locally.
IFS='
'

# Stamped at release time by the `create-release` CI job when it
# uploads install.sh as a release asset. Unstamped copies (e.g.
# running from main) report as "dev".
INSTALLER_VERSION_TAG="@@INSTALLER_VERSION_TAG@@"
INSTALLER_VERSION_SHA="@@INSTALLER_VERSION_SHA@@"
case "$INSTALLER_VERSION_TAG" in
    @@*) INSTALLER_VERSION_TAG="dev"; INSTALLER_VERSION_SHA="(unstamped)" ;;
esac
INSTALLER_VERSION="${INSTALLER_VERSION_TAG} ${INSTALLER_VERSION_SHA}"

DEFAULT_REPO="bitwisecook/tcl-lsp"
REPO="${TCL_LSP_REPO:-$DEFAULT_REPO}"
VERSION="${TCL_LSP_VERSION:-latest}"

# ONLY: which CLIs to install (tcl / f5 / both). ONLY_EXPLICIT=1 when
# pinned via env so choose_clis() skips the prompt.
if [ -n "${TCL_LSP_ONLY:-}" ]; then
    ONLY="$TCL_LSP_ONLY"
    ONLY_EXPLICIT=1
else
    ONLY=both
    ONLY_EXPLICIT=0
fi

# Plan flags set by choose_clis() / choose_ai_components().
# All "" (unset) until then, at which point each becomes "1" or "0".
# install_cli / install_ai_* read these instead of
# asking again later.
WANT_TCL=""
WANT_F5=""
WANT_MCP=""
WANT_SKILLS=""

GREEN=''; YELLOW=''; RED=''; BOLD=''; RESET=''
if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    GREEN="$(tput setaf 2 2>/dev/null || true)"
    YELLOW="$(tput setaf 3 2>/dev/null || true)"
    RED="$(tput setaf 1 2>/dev/null || true)"
    BOLD="$(tput bold 2>/dev/null || true)"
    RESET="$(tput sgr0 2>/dev/null || true)"
fi

log()  { printf '%s==>%s %s\n'  "$GREEN" "$RESET" "$*"; }
warn() { printf '%swarn:%s %s\n' "$YELLOW" "$RESET" "$*" >&2; }
die()  { printf '%serror:%s %s\n' "$RED"   "$RESET" "$*" >&2; exit 1; }

# Stderr breadcrumb for every interactive prompt, so a question + answer
# pair stays visible in scrollback. TCL_LSP_QUIET=1 silences these.
prompt_record() {
    # prompt_record <KIND> <QUESTION> <ANSWER>
    [ "${TCL_LSP_QUIET:-0}" = "1" ] && return 0
    printf '%sask:%s [%s] %s -> %s\n' \
           "$YELLOW" "$RESET" "$1" "$2" "$3" >&2
}

# No-op shim for download() / future call sites that want to leave a
# breadcrumb without forcing a stderr line.  Kept as a function so a
# future TCL_LSP_TRACE=1 mode can light it up without further edits.
record() {
    [ "${TCL_LSP_TRACE:-0}" = "1" ] || return 0
    printf '%strace:%s %s\n' "$YELLOW" "$RESET" "$*" >&2
}

TTY_IN=""               # stdin | /dev/tty | none (set by detect_tty)
TTY_OUT=""              # stderr | /dev/tty | none
TTY_PROBED=0

# Probe once; `curl | sh` pipes stdin but /dev/tty is usually reachable.
detect_tty() {
    if [ -t 0 ]; then TTY_IN=stdin
    elif (exec 3</dev/tty) 2>/dev/null; then TTY_IN=/dev/tty
    else TTY_IN=none
    fi
    if [ -t 2 ]; then TTY_OUT=stderr
    elif (exec 3>/dev/tty) 2>/dev/null; then TTY_OUT=/dev/tty
    else TTY_OUT=none
    fi
}

tty_available() {
    [ "$TTY_PROBED" = 1 ] || { detect_tty; TTY_PROBED=1; }
    [ "$TTY_IN" != none ] && [ "$TTY_OUT" != none ]
}

print_prompt() {
    tty_available || return 1
    if [ "$TTY_OUT" = stderr ]; then printf '%s ' "$1" >&2
    else printf '%s ' "$1" >/dev/tty
    fi
}

# Terminating-newline sibling for prompts that span multiple lines.
print_line() {
    tty_available || return 1
    if [ "$TTY_OUT" = stderr ]; then printf '%s\n' "$1" >&2
    else printf '%s\n' "$1" >/dev/tty
    fi
}

read_user_line() {
    tty_available || return 1
    if [ "$TTY_IN" = stdin ]; then IFS= read -r reply
    else IFS= read -r reply </dev/tty
    fi
}

# --help prints the leading comment block as documentation.
usage() {
    awk 'NR > 1 && /^[^#]/ {exit} NR > 1 {sub(/^# ?/, ""); print}' "$0"
    exit 0
}

for arg in "$@"; do
    case "$arg" in
        -h|--help)    usage ;;
        -V|--version) printf 'tcl-lsp installer %s\n' "$INSTALLER_VERSION"; exit 0 ;;
        --) break ;;
        -*) die "unknown flag: $arg (try --help)" ;;
    esac
done

case "$REPO" in
    */*) : ;;
    *)   die "TCL_LSP_REPO must be in 'owner/repo' form: $REPO" ;;
esac
case "$REPO" in
    *[!A-Za-z0-9._/-]*) die "TCL_LSP_REPO contains invalid characters: $REPO" ;;
esac
if [ "$REPO" != "$DEFAULT_REPO" ]; then
    warn "Using non-default release source: github.com/$REPO"
    warn "(default is github.com/$DEFAULT_REPO)"
fi

_ask_yesno_text() {
    # Text-mode yes/no. $1 = prompt, $2 = default (yes/no).
    default="$2"
    print_prompt "$1"
    read_user_line || { prompt_record yesno "$1" "read-failed"; return 1; }
    case "$reply" in
        y|Y|yes|YES|Yes)        prompt_record yesno "$1" "yes ($reply)";       return 0 ;;
        n|N|no|NO|No)           prompt_record yesno "$1" "no ($reply)";        return 1 ;;
        '')
            if [ "$default" = yes ]; then
                prompt_record yesno "$1" "yes (default)";    return 0
            else
                prompt_record yesno "$1" "no (default)";     return 1
            fi ;;
        *)
            if [ "$default" = yes ]; then
                prompt_record yesno "$1" "yes ($reply)";     return 0
            else
                prompt_record yesno "$1" "no ($reply)";      return 1
            fi ;;
    esac
}

# Opt-in prompt — default no when headless.
ask() {
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}"  = "1" ]; then prompt_record ask "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    tty_available || { prompt_record ask "$1" "no (no-tty, default-no)"; return 1; }
    _ask_yesno_text "$1" yes
}

# Opt-out prompt — default yes when headless (upstream signal was the opt-in).
ask_optout() {
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then prompt_record ask-optout "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask-optout "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    tty_available || { prompt_record ask-optout "$1" "yes (no-tty, default-yes)"; return 0; }
    _ask_yesno_text "$1" yes
}

# Strict default-NO — destructive / privileged choices. Empty input, headless
# run, anything other than y/yes counts as no.
ask_default_no() {
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask-no "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then prompt_record ask-no "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    tty_available || { prompt_record ask-no "$1" "no (no-tty, default-no)"; return 1; }
    print_prompt "$1"
    read_user_line || { prompt_record ask-no "$1" "read-failed"; return 1; }
    case "$reply" in
        y|Y|yes|YES|Yes) prompt_record ask-no "$1" "yes ($reply)"; return 0 ;;
        *)               prompt_record ask-no "$1" "no (${reply:-default})"; return 1 ;;
    esac
}

# Root-requirement accumulator. Each entry: "<what>|<how to avoid>".
# require_root_or_die() is called once in phase 2.5; if non-empty and the
# script isn't already running as root, we print the full report and exit.
ROOT_REASONS=""
need_root() {
    # need_root <what> <how-to-avoid>
    ROOT_REASONS="${ROOT_REASONS}${ROOT_REASONS:+
}$1|$2"
}

require_root_or_die() {
    [ -z "$ROOT_REASONS" ] && return 0
    [ "$(id -u)" = 0 ] && return 0
    msg="this install needs root for the following steps:
"
    OLD_IFS="$IFS"; IFS='
'
    for line in $ROOT_REASONS; do
        what="${line%%|*}"
        avoid="${line#*|}"
        msg="${msg}  - ${what}
      avoid by: ${avoid}
"
    done
    IFS="$OLD_IFS"
    msg="${msg}
Re-run as root with the two-step pattern (safer than \`curl | sudo sh\` —
you can inspect the script first):
  curl -fsSLo /tmp/tcl-lsp-install.sh \\
       https://raw.githubusercontent.com/${REPO}/main/scripts/install/install.sh
  sudo sh /tmp/tcl-lsp-install.sh"
    die "$msg"
}

# Validate + assign $PREFIX. Rejects values that would escape the
# rc-file `export PATH="…:$PATH"` quoting or aren't absolute.
set_prefix() {
    p="$1"
    case "$p" in
        '')
            die "install location cannot be empty" ;;
        [!/]*)
            die "install location must be an absolute path: $p" ;;
        *[!A-Za-z0-9._/+~-]*)
            die "install location contains unsupported characters: $p
Allowed: A-Z a-z 0-9 . _ / + ~ - (no quotes, colons, spaces, dollars, newlines)" ;;
    esac
    PREFIX="$p"
}

# Tri-state $PREFIX: env wins; otherwise picker may overwrite.
if [ -n "${TCL_LSP_PREFIX:-}" ]; then
    set_prefix "$TCL_LSP_PREFIX"
    PREFIX_EXPLICIT=1
else
    set_prefix "$HOME/.local/bin"
    PREFIX_EXPLICIT=0
fi

# Per-CLI override dirs (split-directory update). Empty = use $PREFIX.
TCL_PREFIX_OVERRIDE=""
F5_PREFIX_OVERRIDE=""
MCP_PREFIX_OVERRIDE=""

# Install dir for binary $1.
prefix_for() {
    case "$1" in
        tcl)              printf '%s' "${TCL_PREFIX_OVERRIDE:-$PREFIX}" ;;
        f5)               printf '%s' "${F5_PREFIX_OVERRIDE:-$PREFIX}" ;;
        mcp)              printf '%s' "${MCP_PREFIX_OVERRIDE:-$PREFIX}" ;;
        *)                printf '%s' "$PREFIX" ;;
    esac
}

# Read /etc/os-release (uid-0, not world-writable) and set $ID/$ID_LIKE.
read_os_release_safe() {
    f=/etc/os-release
    [ -e "$f" ] || return 1
    # `ls -ldnL` is POSIX. The `-L` follows symlinks so we check the
    # ownership/mode of the *target* file — on every modern Linux
    # distro /etc/os-release is a symlink into /usr/lib/, and the
    # symlink itself is world-writable (777) by definition. The
    # subsequent read also follows the symlink, so this is consistent.
    lsout=$(ls -ldnL "$f" 2>/dev/null)
    if [ -z "$lsout" ]; then
        die "cannot stat $f.
Re-run with TCL_LSP_OS=<debian|rhel|fedora|arch|alpine|macos> to bypass detection."
    fi
    _osr_mode=$(printf '%s' "$lsout" | awk '{print $1}')
    _osr_uid=$(printf '%s' "$lsout" | awk '{print $3}')
    # Mode string: type + 9 perm chars. Position 9 is the others-write bit.
    case "$_osr_mode" in
        ????????w*) _osr_world_writable=1 ;;
        *)          _osr_world_writable=0 ;;
    esac
    if [ "$_osr_uid" != 0 ] || [ "$_osr_world_writable" = 1 ]; then
        die "$f is not root-owned or is world-writable — refusing to read it.
Re-run with TCL_LSP_OS=<debian|rhel|fedora|arch|alpine|macos> to bypass detection."
    fi
    ID=""
    ID_LIKE=""
    while IFS='=' read -r _osr_key _osr_val; do
        case "$_osr_key" in
            ID|ID_LIKE) : ;;
            *) continue ;;
        esac
        case "$_osr_val" in
            \"*\") _osr_val="${_osr_val#\"}"; _osr_val="${_osr_val%\"}" ;;
        esac
        case "$_osr_key" in
            ID)      ID="$_osr_val" ;;
            ID_LIKE) ID_LIKE="$_osr_val" ;;
        esac
    done <"$f"
    unset _osr_key _osr_val
    return 0
}

detect_os() {
    UNAME="$(uname -s 2>/dev/null || echo unknown)"
    if [ -n "${TCL_LSP_OS:-}" ]; then
        OS="$TCL_LSP_OS"
        log "OS override (TCL_LSP_OS): $OS"
        return
    fi
    case "$UNAME" in
        Darwin) OS=macos ;;
        Linux)
            if read_os_release_safe; then
                case "${ID:-}:${ID_LIKE:-}" in
                    debian:*|ubuntu:*|*:*debian*|*:*ubuntu*) OS=debian ;;
                    fedora:*|*:*fedora*)                     OS=fedora ;;
                    rhel:*|centos:*|rocky:*|almalinux:*|*:*rhel*|*:*centos*) OS=rhel ;;
                    arch:*|manjaro:*|*:*arch*)               OS=arch ;;
                    alpine:*|*:*alpine*)                     OS=alpine ;;
                    *) OS=linux ;;
                esac
            else
                OS=linux
            fi
            ;;
        *) die "unsupported OS: $UNAME (only macOS and Linux are supported)" ;;
    esac
    log "detected OS: $OS"
}

# $SHELL is the login shell — not necessarily what's running us.
# Trusted for rc-file path resolution.
detect_shell() {
    SHELL_NAME=""
    case "${SHELL:-}" in
        */zsh)  SHELL_NAME=zsh ;;
        */bash) SHELL_NAME=bash ;;
        */fish) SHELL_NAME=fish ;;
        */ksh)  SHELL_NAME=ksh ;;
        */dash) SHELL_NAME=dash ;;
        *) SHELL_NAME="$(basename "${SHELL:-sh}")" ;;
    esac

    case "$SHELL_NAME" in
        zsh)  RC="${ZDOTDIR:-$HOME}/.zshrc" ;;
        bash)
            if [ "$OS" = macos ]; then
                RC="$HOME/.bash_profile"
            else
                RC="$HOME/.bashrc"
            fi
            ;;
        fish) RC="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" ;;
        *)    RC="$HOME/.profile" ;;
    esac
    log "detected shell: $SHELL_NAME (rc: $RC)"
}


have() { command -v "$1" >/dev/null 2>&1; }

python_ok() {
    # Returns 0 if $1 resolves to a Python >= 3.10 interpreter.
    [ -n "${1:-}" ] || return 1
    have "$1" || return 1
    "$1" - <<'PY' 2>/dev/null
import sys
sys.exit(0 if sys.version_info >= (3, 10) else 1)
PY
}

find_python() {
    # Probe well-known absolute paths first so a poisoned $PATH cannot
    # silently shadow the system interpreter we're about to use.
    PYTHON=""
    for candidate in \
        /opt/homebrew/bin/python3.14 /opt/homebrew/bin/python3.13 /opt/homebrew/bin/python3.12 \
        /opt/homebrew/bin/python3.11 /opt/homebrew/bin/python3.10 /opt/homebrew/bin/python3 \
        /usr/local/bin/python3.14 /usr/local/bin/python3.13 /usr/local/bin/python3.12 \
        /usr/local/bin/python3.11 /usr/local/bin/python3.10 /usr/local/bin/python3 \
        /usr/bin/python3.14 /usr/bin/python3.13 /usr/bin/python3.12 \
        /usr/bin/python3.11 /usr/bin/python3.10 /usr/bin/python3 \
        python3.14 python3.13 python3.12 python3.11 python3.10 \
        python3 python; do
        if python_ok "$candidate"; then
            PYTHON="$(command -v "$candidate")"
            return 0
        fi
    done
    return 1
}

# Echo a command before running it; do not suppress its stdout/stderr.
run_cmd() {
    log "+ $*"
    "$@"
}

run_as_root() {
    # Run a command that requires root. Caller must have already gone
    # through need_root + require_root_or_die, so by the time we get here
    # we're either uid 0 (just run it) or something has gone wrong.
    log "+ $*"
    if [ "$(id -u)" = 0 ]; then
        "$@"
    else
        die "internal: tried to run as root without escalation gate: $*"
    fi
}

# Phase 1: detect Python and record (without installing) whether we'll
# need to install it. install_python() is called in phase 3.
PYTHON_INSTALL_PLANNED=0
plan_python_if_needed() {
    if find_python; then return 0; fi
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Python 3.10+ not found and TCL_LSP_NO_DEPS=1 — install Python manually and retry."
    fi
    case "$OS" in
        macos)
            if ! have brew; then
                die "Python 3.10+ not found and Homebrew is not installed.
Install Homebrew (https://brew.sh) or Python 3.10+ manually, then re-run."
            fi
            ;;
        debian|rhel|fedora|arch|alpine)
            need_root "install Python 3.10+ via $(pkg_manager_label)" \
                      "install Python 3.10+ manually (e.g. \`$(pkg_manager_install_cmd) python3\`) or set TCL_LSP_NO_DEPS=1"
            ;;
        *)
            die "Cannot auto-install Python on this OS. Install Python 3.10+ manually and retry." ;;
    esac
    PYTHON_INSTALL_PLANNED=1
}

# Phase 3: actually install Python — only reached after require_root_or_die.
install_python() {
    [ "$PYTHON_INSTALL_PLANNED" = 1 ] || return 0
    log "Python 3.10+ not found — installing via package manager"
    case "$OS" in
        macos)
            run_cmd brew install python@3.14 \
                || run_cmd brew install python@3.13 \
                || run_cmd brew install python3
            ;;
        debian)
            apt_get_update_once
            run_as_root apt-get install -y python3 ca-certificates curl
            ;;
        rhel)
            PM="yum"; have dnf && PM="dnf"
            run_as_root "$PM" install -y ca-certificates curl
            installed=0
            for v in python3.12 python3.11; do
                if "$PM" list --available "$v" >/dev/null 2>&1 \
                   || "$PM" info "$v" >/dev/null 2>&1; then
                    if run_as_root "$PM" install -y "$v"; then installed=1; break; fi
                fi
            done
            [ "$installed" = 0 ] && run_as_root "$PM" install -y python3
            ;;
        fedora)
            PM="yum"; have dnf && PM="dnf"
            run_as_root "$PM" install -y python3 ca-certificates curl
            ;;
        arch)
            run_as_root pacman -Sy --noconfirm python ca-certificates curl
            ;;
        alpine)
            run_as_root apk add --no-cache python3 ca-certificates curl
            ;;
    esac
    find_python || die "Python install completed but no python3.10+ found on PATH."
}

# Phase 1: pick a downloader, recording an install need if both are missing.
CURL_INSTALL_PLANNED=0
plan_downloader() {
    if have curl; then DOWNLOADER="curl"; return; fi
    if have wget; then DOWNLOADER="wget"; return; fi
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Neither curl nor wget found and TCL_LSP_NO_DEPS=1 — install one and retry."
    fi
    case "$OS" in
        macos) die "curl missing on macOS — install Xcode Command Line Tools and retry." ;;
        debian|rhel|fedora|arch|alpine)
            need_root "install curl via $(pkg_manager_label)" \
                      "install curl or wget manually, then re-run" ;;
        *) die "Need curl or wget to download release artefacts." ;;
    esac
    CURL_INSTALL_PLANNED=1
    DOWNLOADER="curl"
}

install_downloader() {
    [ "$CURL_INSTALL_PLANNED" = 1 ] || return 0
    log "installing curl via package manager"
    case "$OS" in
        debian)        apt_get_update_once
                       run_as_root apt-get install -y curl ;;
        rhel|fedora)   if have dnf; then run_as_root dnf install -y curl
                       else run_as_root yum install -y curl; fi ;;
        arch)          run_as_root pacman -Sy --noconfirm curl ;;
        alpine)        run_as_root apk add --no-cache curl ;;
    esac
    have curl || die "curl install failed."
}

# apt-get install -y on a minimal image fails when /var/lib/apt/lists is
# empty. Run apt-get update once per session before the first install.
APT_UPDATED=0
apt_get_update_once() {
    [ "$APT_UPDATED" = 1 ] && return 0
    run_as_root apt-get update
    APT_UPDATED=1
}


pkg_name_for() {
    # OS-package name for CMD on OS. Empty = no equivalent on this OS.
    os="$1"; cmd="$2"
    case "$os:$cmd" in
        # OpenSSH client (ships scp alongside ssh)
        debian:ssh)                    printf 'openssh-client' ;;
        rhel:ssh|fedora:ssh)           printf 'openssh-clients' ;;
        arch:ssh)                      printf 'openssh' ;;
        alpine:ssh)                    printf 'openssh-client' ;;
        macos:ssh)                     printf '' ;;

        # tshark — packaged as wireshark-cli on RPM-style distros
        rhel:tshark|fedora:tshark)     printf 'wireshark-cli' ;;
        arch:tshark)                   printf 'wireshark-cli' ;;
        macos:tshark)                  printf 'wireshark' ;;

        # tclsh (system Tcl interpreter)
        macos:tclsh)                   printf 'tcl-tk' ;;
        debian:tclsh|alpine:tclsh|arch:tclsh|rhel:tclsh|fedora:tclsh)
                                       printf 'tcl' ;;

        # default — package name = command name
        *:*)                           printf '%s' "$cmd" ;;
    esac
}

install_pkg() {
    # install_pkg CMD — best-effort install of the OS package providing CMD.
    # Caller must have passed require_root_or_die already.
    cmd="$1"
    pkg="$(pkg_name_for "$OS" "$cmd")"
    if [ -z "$pkg" ]; then
        warn "no $cmd package mapping for $OS — install manually"
        return 1
    fi
    case "$OS" in
        macos)
            if ! have brew; then
                warn "brew not found — install $pkg manually"
                return 1
            fi
            run_cmd brew install "$pkg" \
                || { warn "brew install $pkg failed"; return 1; }
            log "installed $cmd via brew"
            ;;
        debian)
            apt_get_update_once
            run_as_root apt-get install -y "$pkg" \
                || { warn "apt install $pkg failed"; return 1; }
            log "installed $cmd via apt-get"
            ;;
        rhel|fedora)
            PM="yum"; have dnf && PM="dnf"
            run_as_root "$PM" install -y "$pkg" \
                || { warn "$PM install $pkg failed"; return 1; }
            log "installed $cmd via $PM"
            ;;
        arch)
            run_as_root pacman -Sy --noconfirm "$pkg" \
                || { warn "pacman install $pkg failed"; return 1; }
            log "installed $cmd via pacman"
            ;;
        alpine)
            run_as_root apk add --no-cache "$pkg" \
                || { warn "apk install $pkg failed"; return 1; }
            log "installed $cmd via apk"
            ;;
        *)
            warn "no package manager known for $OS — install $pkg manually"
            return 1 ;;
    esac
}

pkg_manager_label() {
    # Human-readable package-manager identifier for the host.
    case "$OS" in
        macos)        printf 'brew' ;;
        debian)       printf 'apt-get' ;;
        rhel|fedora)  if have dnf; then printf 'dnf'; else printf 'yum'; fi ;;
        arch)         printf 'pacman' ;;
        alpine)       printf 'apk' ;;
        *)            printf 'unknown' ;;
    esac
}

pkg_manager_install_cmd() {
    # The exact install invocation a user would type by hand.
    case "$OS" in
        macos)        printf 'brew install' ;;
        debian)       printf 'sudo apt-get install -y' ;;
        rhel|fedora)  if have dnf; then printf 'sudo dnf install -y'; else printf 'sudo yum install -y'; fi ;;
        arch)         printf 'sudo pacman -Sy --noconfirm' ;;
        alpine)       printf 'sudo apk add --no-cache' ;;
        *)            printf '<unknown package manager>' ;;
    esac
}

# Phase 2 planning state. Set by plan_cli_runtime_dependencies (in the
# question phase); read by install_cli_runtime_dependencies (in the
# execute phase) after require_root_or_die has cleared.
DEPS_PLAN_TCLSH=0
DEPS_PLAN_SSH=0
DEPS_PLAN_SSHPASS=0
DEPS_PLAN_TSHARK=0

# Ask the user about a single optional/required dep. $1=tool $2=required
# (REQUIRED|optional) $3=description. Returns 0 if user accepted (and
# updates DEPS_PLAN_<TOOL>); the package name is also accumulated for the
# "needs root" report.
_plan_one_dep() {
    tool="$1"; status="$2"; description="$3"
    pkg="$(pkg_name_for "$OS" "$tool")"
    pm_label="$(pkg_manager_label)"
    print_line ""
    print_line "${BOLD}${tool}${RESET} — ${status}"
    print_line "  ${description}"
    if [ -n "$pkg" ]; then
        print_line "  package: ${pkg} (via ${pm_label})"
    else
        print_line "  package: <no mapping for $OS — install manually>"
        return 1
    fi
    if [ "$status" = "REQUIRED" ]; then
        if ask "Install ${tool}? [Y/n]"; then
            PLANNED_PKGS="${PLANNED_PKGS} ${pkg}"
            return 0
        fi
        warn "skipped ${tool}: ${description}"
        return 1
    fi
    if ask_default_no "Install ${tool}? [y/N]"; then
        PLANNED_PKGS="${PLANNED_PKGS} ${pkg}"
        return 0
    fi
    return 1
}

plan_cli_runtime_dependencies() {
    # Survey external tools the tcl/f5 CLIs may shell out to. Asks the
    # user about each missing dep individually. The install itself runs
    # later in phase 3.
    [ "${TCL_LSP_NO_DEPS:-0}" = "1" ] && return

    needs_tclsh=0
    needs_ssh=0
    needs_sshpass=0
    needs_tshark=0

    case "$ONLY" in
        tcl|both)
            if ! have tclsh && ! have tclsh9.0 && ! have tclsh8.6 && ! have tclsh8.5; then
                needs_tclsh=1
            fi
            ;;
    esac
    case "$ONLY" in
        f5|both)
            have ssh     || needs_ssh=1
            have sshpass || needs_sshpass=1
            have tshark  || needs_tshark=1
            ;;
    esac

    total=$((needs_tclsh + needs_ssh + needs_sshpass + needs_tshark))
    [ "$total" = 0 ] && return

    log "CLI runtime dependencies missing (${total})"
    PLANNED_PKGS=""

    if [ "$needs_ssh" = 1 ]; then
        _plan_one_dep ssh "REQUIRED" \
            "f5 fetch downloads BIG-IP configs over SSH; without this, fetch will fail." \
            && DEPS_PLAN_SSH=1
    fi
    if [ "$needs_tclsh" = 1 ]; then
        _plan_one_dep tclsh "optional" \
            "the real Tcl interpreter; required for 'tcl pkg', 'tcl venv', and 'tcl explore' against live code." \
            && DEPS_PLAN_TCLSH=1
    fi
    if [ "$needs_sshpass" = 1 ]; then
        _plan_one_dep sshpass "optional" \
            "fallback password auth for 'f5 fetch'; not needed if you use SSH keys." \
            && DEPS_PLAN_SSHPASS=1
    fi
    if [ "$needs_tshark" = 1 ]; then
        _plan_one_dep tshark "optional" \
            "Wireshark CLI; powers 'f5 explain-flow --tshark', 'enrich-pcapng', and 'pcap-remap'." \
            && DEPS_PLAN_TSHARK=1
    fi

    PLANNED_PKGS="${PLANNED_PKGS# }"
    [ -z "$PLANNED_PKGS" ] && return

    log "will install: $(pkg_manager_install_cmd) $PLANNED_PKGS"

    # macOS brew runs as the user; everything else needs root.
    if [ "$OS" != "macos" ]; then
        need_root "install $PLANNED_PKGS via $(pkg_manager_label)" \
                  "decline the prompts above, or set TCL_LSP_NO_DEPS=1 (you'll lose the listed features)"
    fi
}

install_cli_runtime_dependencies() {
    # Phase 3 — execute the plan recorded by plan_cli_runtime_dependencies.
    [ "$DEPS_PLAN_SSH"     = 1 ] && install_pkg ssh
    [ "$DEPS_PLAN_TCLSH"   = 1 ] && install_pkg tclsh
    [ "$DEPS_PLAN_SSHPASS" = 1 ] && install_pkg sshpass
    [ "$DEPS_PLAN_TSHARK"  = 1 ] && install_pkg tshark
    return 0
}

WGET_HAS_HTTPS_ONLY=""
require_wget_https_only() {
    # Aborts when wget lacks --https-only; TCL_LSP_ALLOW_INSECURE_WGET=1 to bypass.
    case "$WGET_HAS_HTTPS_ONLY" in
        1) return 0 ;;
        0) return 1 ;;
    esac
    if wget --help 2>&1 | grep -q -- --https-only; then
        WGET_HAS_HTTPS_ONLY=1
        return 0
    fi
    WGET_HAS_HTTPS_ONLY=0
    if [ "${TCL_LSP_ALLOW_INSECURE_WGET:-0}" = "1" ]; then
        warn "wget here does not support --https-only — HTTPS redirect pinning OFF"
        warn "(TCL_LSP_ALLOW_INSECURE_WGET=1 set, proceeding anyway)"
        return 1
    fi
    die "wget on this host does not support --https-only.
A MITM that redirected an HTTPS request to http:// could replace
the downloaded artefact AND the SHA256SUMS file in the same response.
Options:
  - install curl (\`apt-get install curl\` / \`brew install curl\` / etc.) and re-run
  - upgrade wget to >= 1.14
  - set TCL_LSP_ALLOW_INSECURE_WGET=1 to bypass (do NOT do this on an
    untrusted network)"
}


detect_proxy() {
    h="${https_proxy:-${HTTPS_PROXY:-}}"
    p="${http_proxy:-${HTTP_PROXY:-}}"
    n="${no_proxy:-${NO_PROXY:-}}"

    if [ -n "$h" ] || [ -n "$p" ]; then
        log "proxy: https_proxy=${h:-(unset)} http_proxy=${p:-(unset)}${n:+ no_proxy=$n}"
        return
    fi

    if [ "$OS" = macos ] && have scutil; then
        if scutil --proxy 2>/dev/null | grep -qE '^[[:space:]]*HTTP(S)?Enable[[:space:]]*:[[:space:]]*1'; then
            warn "macOS system proxy is configured but http_proxy / https_proxy are unset."
            warn "curl/wget read only environment variables — the system proxy will be bypassed."
            warn "If you need the proxy, export https_proxy=http://host:port before re-running."
        fi
    fi
}


# TLS_FALLBACK: 0 = strict (TLS 1.3 + HTTP/2); 1 = lax (TLS 1.2 + HTTP/1.1).
TLS_FALLBACK=0
if [ "${TCL_LSP_ALLOW_TLS12:-0}" = "1" ]; then
    TLS_FALLBACK=1
fi

WGET_HAS_TLS13=""
wget_supports_tls13() {
    case "$WGET_HAS_TLS13" in
        1) return 0 ;;
        0) return 1 ;;
    esac
    if wget --help 2>&1 | grep -q -- '--secure-protocol'; then
        WGET_HAS_TLS13=1
        return 0
    fi
    WGET_HAS_TLS13=0
    return 1
}

CURL_HAS_HTTP2=
CURL_HAS_TLS13=
curl_probe_setopt() {
    # Probe whether `curl <flag>` is accepted at setopt time. Drives a
    # fast-failing request against a refused local socket. Exit 4 means
    # the option was compiled out; anything else means it was accepted.
    rc=0
    curl "$1" --connect-timeout 1 -sS -o /dev/null \
        "https://127.0.0.1:1" >/dev/null 2>&1 || rc=$?
    [ "$rc" != 4 ]
}

curl_probe_capabilities() {
    [ -n "$CURL_HAS_HTTP2" ] && return 0
    # The first parenthesised token after "libcurl/<version>" in curl -V
    # is the active TLS backend. SecureTransport (Apple's stock curl on
    # macOS) parses --tlsv1.3 fine but raises CURLE_NOT_BUILT_IN during
    # the real TLS handshake, so treat it as TLS-1.3-incapable up front.
    cv=$(curl -V 2>/dev/null || true)
    backend=$(printf '%s\n' "$cv" | awk '
        /^curl / {
            for (i=1; i<=NF; i++) {
                if ($i ~ /^libcurl\//) {
                    gsub(/[()]/, "", $(i+1))
                    print $(i+1)
                    exit
                }
            }
        }')
    if curl_probe_setopt --http2; then CURL_HAS_HTTP2=1; else CURL_HAS_HTTP2=0; fi
    tls13_by_backend=0
    case "$backend" in
        SecureTransport) CURL_HAS_TLS13=0; tls13_by_backend=1 ;;
        *)
            if curl_probe_setopt --tlsv1.3; then CURL_HAS_TLS13=1; else CURL_HAS_TLS13=0; fi
            ;;
    esac
    missing=""
    [ "$CURL_HAS_TLS13" = 0 ] && [ "$tls13_by_backend" = 0 ] && missing="${missing} TLS 1.3"
    [ "$CURL_HAS_HTTP2" = 0 ] && missing="${missing} HTTP/2"
    if [ -n "$missing" ]; then
        warn "curl (${backend:-unknown}) missing support for:${missing} — falling back automatically."
    fi
}

curl_invoke() {
    # curl with the current transport-strictness, plus the caller's args.
    # Capabilities are probed once on first use because not every libcurl
    # build supports --http2 or --tlsv1.3 (e.g. Apple's stock curl).
    curl_probe_capabilities
    set -- --proto '=https' --proto-redir '=https' "$@"
    if [ "$TLS_FALLBACK" = 1 ]; then
        set -- --tlsv1.2 --http1.1 "$@"
    else
        if [ "$CURL_HAS_TLS13" = 1 ]; then
            set -- --tlsv1.3 "$@"
        else
            set -- --tlsv1.2 "$@"
        fi
        if [ "$CURL_HAS_HTTP2" = 1 ]; then
            set -- --http2 "$@"
        else
            set -- --http1.1 "$@"
        fi
    fi
    curl "$@"
}

wget_invoke() {
    # wget with the current transport-strictness, plus the caller's args.
    # HTTP/2 isn't supported by wget(1), so we only switch TLS levels.
    https_arg=
    require_wget_https_only && https_arg=--https-only
    if [ "$TLS_FALLBACK" = 1 ] || ! wget_supports_tls13; then
        if [ -n "$https_arg" ]; then
            wget "$https_arg" "$@"
        else
            wget "$@"
        fi
    else
        if [ -n "$https_arg" ]; then
            wget "$https_arg" --secure-protocol=TLSv1_3 "$@"
        else
            wget --secure-protocol=TLSv1_3 "$@"
        fi
    fi
}

maybe_fallback_tls() {
    # Prompt once per run for TLS 1.2 + HTTP/1.1 fallback. Dies on decline.
    [ "$TLS_FALLBACK" = 1 ] && return 1
    warn "TLS 1.3 / HTTP/2 negotiation failed (corporate proxy or older intermediary?)"
    if ! ask_default_no "Fall back to TLS 1.2 + HTTP/1.1 for this and subsequent downloads? [y/N]"; then
        die "aborted: TLS 1.3 + HTTP/2 download failed and fallback declined.
Set TCL_LSP_ALLOW_TLS12=1 to opt in non-interactively, or investigate
why TLS 1.3 / HTTP/2 isn't reachable from your network."
    fi
    TLS_FALLBACK=1
    return 0
}

_download_attempt() {
    if [ "$DOWNLOADER" = "wget" ]; then
        wget_invoke -qO "$1" "$2"
    else
        curl_invoke -fsSL -o "$1" "$2"
    fi
}

# Did the curl exit code suggest a transport/TLS problem worth retrying
# under TLS 1.2 fallback? 22 (HTTP error response) and 6 (couldn't
# resolve host) and 7 (couldn't connect) won't be fixed by lowering TLS.
_curl_rc_implies_tls_retry() {
    case "$1" in
        22|6|7|18|23) return 1 ;;
        *) return 0 ;;
    esac
}

# Set by _download_run after a terminal failure; cleared on success.
LAST_DOWNLOAD_ERR=""

# Returns 0 on success, non-zero on terminal failure. Captures the
# downloader's stderr summary into LAST_DOWNLOAD_ERR. Used by both the
# default `download()` (which dies) and the optional `try_download()`
# (which lets the caller decide).
_download_run() {
    url="$1"; out="$2"
    log "fetching $(basename "$out")"
    record "DOWNLOAD start url=$url"
    err_file="$WORKDIR/.dl.err"
    LAST_DOWNLOAD_ERR=""
    : >"$err_file" 2>/dev/null || true

    rc=0
    if [ "$DOWNLOADER" = "wget" ]; then
        if wget_invoke -qO "$out" "$url" 2>"$err_file"; then
            record "DOWNLOAD ok url=$url"
            rm -f "$err_file" 2>/dev/null
            return 0
        else
            rc=$?
        fi
    else
        if curl_invoke -fsSL -o "$out" "$url" 2>"$err_file"; then
            record "DOWNLOAD ok url=$url"
            rm -f "$err_file" 2>/dev/null
            return 0
        else
            rc=$?
        fi
    fi

    err_summary=""
    if [ -s "$err_file" ]; then
        err_summary="$(tr '\n' ' ' <"$err_file" 2>/dev/null | sed 's/  */ /g')"
        record "DOWNLOAD stderr: $err_summary"
        warn "$(basename "$out") download failed ($DOWNLOADER rc=$rc): $(head -n 1 "$err_file")"
    else
        warn "$(basename "$out") download failed ($DOWNLOADER rc=$rc, no stderr captured)"
    fi
    rm -f "$err_file" 2>/dev/null
    LAST_DOWNLOAD_ERR="$err_summary"

    # Skip TLS fallback for errors that won't be fixed by lowering TLS.
    if [ "$DOWNLOADER" = "curl" ] && ! _curl_rc_implies_tls_retry "$rc"; then
        return 1
    fi
    if maybe_fallback_tls; then
        : >"$err_file" 2>/dev/null || true
        if [ "$DOWNLOADER" = "wget" ]; then
            if wget_invoke -qO "$out" "$url" 2>"$err_file"; then
                rm -f "$err_file" 2>/dev/null
                LAST_DOWNLOAD_ERR=""
                return 0
            fi
        else
            if curl_invoke -fsSL -o "$out" "$url" 2>"$err_file"; then
                rm -f "$err_file" 2>/dev/null
                LAST_DOWNLOAD_ERR=""
                return 0
            fi
        fi
        if [ -s "$err_file" ]; then
            LAST_DOWNLOAD_ERR="$(tr '\n' ' ' <"$err_file" 2>/dev/null | sed 's/  */ /g')"
        fi
        rm -f "$err_file" 2>/dev/null
    fi
    return 1
}

# download URL OUTPUT — dies on failure with the URL + captured error.
# Use for mandatory artefacts (CLI zipapps, SHA256SUMS). For files
# that may legitimately be absent on a release, use try_download.
download() {
    _download_run "$@" && return 0
    die "download failed: $1${LAST_DOWNLOAD_ERR:+
${LAST_DOWNLOAD_ERR}}"
}

# try_download URL OUTPUT — returns 0 on success, 1 on failure.
# Caller is responsible for surfacing whatever error message makes
# sense in context. LAST_DOWNLOAD_ERR holds the captured stderr.
try_download() {
    _download_run "$@"
}

resolve_latest_tag() {
    # Resolve via the /releases/latest HTML redirect (no API rate-limit).
    redirect_url="https://github.com/$REPO/releases/latest"
    final_url=""
    if [ "$DOWNLOADER" = "wget" ]; then
        final_url="$(wget_invoke -qS --max-redirect=10 -O /dev/null "$redirect_url" 2>&1 \
            | awk '/^  Location:/ {loc=$2} END {print loc}' | tr -d '\r')"
        if [ -z "$final_url" ] && maybe_fallback_tls; then
            final_url="$(wget_invoke -qS --max-redirect=10 -O /dev/null "$redirect_url" 2>&1 \
                | awk '/^  Location:/ {loc=$2} END {print loc}' | tr -d '\r')"
        fi
    else
        final_url="$(curl_invoke -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url")"
        if [ -z "$final_url" ] && maybe_fallback_tls; then
            final_url="$(curl_invoke -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url")"
        fi
    fi
    case "$final_url" in
        *"/releases/tag/"*) printf '%s\n' "${final_url##*/tag/}" ;;
        *) return 1 ;;
    esac
}

RESOLVED_TAG=""
ensure_tag() {
    # Lazy + memoised. Sets RESOLVED_TAG and VER_NO_V.
    if [ -n "$RESOLVED_TAG" ]; then return; fi
    if [ "$VERSION" = "latest" ]; then
        RESOLVED_TAG="$(resolve_latest_tag)" \
            || die "could not resolve latest release tag from https://github.com/$REPO/releases/latest. Set TCL_LSP_VERSION=vX.Y.Z to bypass."
    else
        RESOLVED_TAG="$VERSION"
    fi
    VER_NO_V="${RESOLVED_TAG#v}"
}

asset_url() {
    ensure_tag
    printf 'https://github.com/%s/releases/download/%s/%s\n' \
        "$REPO" "$RESOLVED_TAG" "$1"
}


WORKDIR=""
init_workdir() {
    WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-install.XXXXXX")"
    # Cleanup runs unconditionally on exit. Signals additionally `exit`
    # — without this, the INT/TERM/HUP traps would run the cleanup and
    # then return control to where the script was, so Ctrl+C just
    # cleaned the workdir without stopping the install.
    trap 'rm -rf -- "$WORKDIR"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    trap 'exit 129' HUP
}

SUMS_PATH=""
SUMS_STATE=""   # "present" | "absent" | ""
ensure_sums() {
    # Download SHA256SUMS once. Missing SUMS aborts the install by
    # default; set TCL_LSP_NO_VERIFY=1 to install without integrity
    # checks (only do this if you trust the network path end-to-end).
    [ "${TCL_LSP_NO_VERIFY:-0}" = "1" ] && return 1
    [ "$SUMS_STATE" = "present" ] && return 0
    ensure_tag
    sums_tmp="$WORKDIR/SHA256SUMS"
    if try_download "$(asset_url SHA256SUMS)" "$sums_tmp"; then
        SUMS_PATH="$sums_tmp"
        SUMS_STATE=present
        verify_sums_signature "$SUMS_PATH" || die "cosign verification of SHA256SUMS failed"
        return 0
    fi
    die "release $RESOLVED_TAG has no SHA256SUMS — refusing to install unverified artefacts.
URL: $(asset_url SHA256SUMS)
${LAST_DOWNLOAD_ERR:+Last error: $LAST_DOWNLOAD_ERR}
Options:
  - install a newer release with checksums published, or
  - set TCL_LSP_NO_VERIFY=1 to install without integrity verification
    (only do this if you trust the network path end-to-end)."
}

verify_sums_signature() {
    # Verify SUMS against cosign keyless signature. TCL_LSP_REQUIRE_COSIGN=1 makes missing fatal.
    sums="$1"
    if ! have cosign; then
        if [ "${TCL_LSP_REQUIRE_COSIGN:-0}" = "1" ]; then
            die "cosign not installed but TCL_LSP_REQUIRE_COSIGN=1.
Install cosign (\`brew install cosign\` / \`apt-get install cosign\` / etc.) and retry."
        fi
        return 0
    fi
    bundle="$WORKDIR/SHA256SUMS.cosign.bundle"
    if ! try_download "$(asset_url SHA256SUMS.cosign.bundle)" "$bundle"; then
        if [ "${TCL_LSP_REQUIRE_COSIGN:-0}" = "1" ]; then
            die "SHA256SUMS.cosign.bundle not published for $RESOLVED_TAG
and TCL_LSP_REQUIRE_COSIGN=1. Refusing to proceed with hash-only verification."
        fi
        return 0
    fi
    if cosign verify-blob \
        --bundle "$bundle" \
        --certificate-identity-regexp "^https://github.com/$REPO/\\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        "$sums" >/dev/null 2>&1; then
        log "SHA256SUMS cosign signature verified"
        return 0
    fi
    return 1
}

verify_artefact() {
    # verify_artefact ASSET_NAME LOCAL_PATH
    # No-op when SUMS unavailable and not required (warning was already emitted).
    asset="$1"; local="$2"
    ensure_sums || return 0
    # Match `<hash> NAME` or `<hash> *NAME` (binary mode). Assumes no whitespace in NAME.
    expected="$(awk -v a="$asset" '$2 == a || $2 == "*"a {print $1; exit}' "$SUMS_PATH")"
    [ -n "$expected" ] || die "no SHA256 entry for $asset in SHA256SUMS"
    if have sha256sum; then
        actual="$(sha256sum "$local" | awk '{print $1}')"
    elif have shasum; then
        actual="$(shasum -a 256 "$local" | awk '{print $1}')"
    else
        warn "neither sha256sum nor shasum available — skipping verify for $asset"
        return 0
    fi
    [ "$expected" = "$actual" ] \
        || die "checksum mismatch for $asset
expected: $expected
actual:   $actual"
    log "verified $asset (sha256)"
}


path_contains() {
    case ":${PATH}:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

dir_writable() {
    # 0 if $1 is writable (or its deepest existing ancestor is). Decorative.
    d="$1"
    while [ -n "$d" ] && [ "$d" != "/" ]; do
        if [ -d "$d" ]; then
            [ -w "$d" ]
            return $?
        fi
        d="$(dirname "$d")"
    done
    return 1
}

annotate_candidate() {
    d="$1"
    on_path="not on PATH"
    if path_contains "$d"; then on_path="on PATH"; fi
    if dir_writable "$d"; then
        if [ -d "$d" ]; then perms="writable"; else perms="will create"; fi
    else
        perms="not writable (needs root)"
    fi
    printf '(%s, %s)' "$on_path" "$perms"
}

collect_install_candidates() {
    # Emit one candidate install dir per line.
    seen=""
    emit() {
        case "$seen" in *":$1:"*) return ;; esac
        seen="$seen:$1:"
        printf '%s\n' "$1"
    }
    OLD_IFS="$IFS"
    IFS=:
    # shellcheck disable=SC2086
    for d in $PATH; do
        IFS="$OLD_IFS"
        case "$d" in
            # Literal-prefix match — `${HOME}/` (with trailing slash)
            # cannot be glob-confused even if $HOME has metachars.
            "${HOME}/"*)
                case "$d" in
                    */.*) IFS=:; continue ;;
                esac
                emit "$d"
                ;;
        esac
        IFS=:
    done
    IFS="$OLD_IFS"
    emit "$HOME/.local/bin"
    emit "$HOME/bin"
    for d in /usr/local/bin /opt/homebrew/bin /opt/local/bin; do
        [ -d "$d" ] && emit "$d"
    done
    return 0
}

choose_clis() {
    # Phase 1: which CLI binaries to install. Sets WANT_TCL / WANT_F5
    # and reflects the result into $ONLY (tcl|f5|both|none).
    case "$ONLY" in
        tcl)    d_tcl=1; d_f5=0; d_default=tcl ;;
        f5)     d_tcl=0; d_f5=1; d_default=f5 ;;
        both|*) d_tcl=1; d_f5=1; d_default=both ;;
    esac

    if [ "$ONLY_EXPLICIT" = "1" ] || ! tty_available \
       || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        WANT_TCL="$d_tcl"; WANT_F5="$d_f5"
        log "CLIs (non-interactive): tcl=$WANT_TCL f5=$WANT_F5"
        _finalise_only
        return
    fi

    menu_default=3
    case "$d_default" in tcl) menu_default=1 ;; f5) menu_default=2 ;; both) menu_default=3 ;; esac
    print_line ""
    print_line "Choose which CLIs to install:"
    print_line "  1) tcl"
    print_line "  2) f5"
    print_line "  3) both (tcl + f5)"
    print_line "  4) none (AI-only install)"
    print_prompt "$(printf '  selection [%s]: ' "$menu_default")"
    read_user_line || reply=""
    ans="${reply:-$menu_default}"
    case "$ans" in
        1) WANT_TCL=1; WANT_F5=0 ;;
        2) WANT_TCL=0; WANT_F5=1 ;;
        3) WANT_TCL=1; WANT_F5=1 ;;
        4) WANT_TCL=0; WANT_F5=0 ;;
        *) die "invalid selection: $ans (expected 1..4)" ;;
    esac
    prompt_record cli-plan "menu pick (1..4)" "$ans"
    _finalise_only
    log "CLIs: tcl=$WANT_TCL f5=$WANT_F5"
}

_finalise_only() {
    if   [ "$WANT_TCL" = "1" ] && [ "$WANT_F5" = "1" ]; then ONLY=both
    elif [ "$WANT_TCL" = "1" ]; then ONLY=tcl
    elif [ "$WANT_F5"  = "1" ]; then ONLY=f5
    else ONLY=none
    fi
}

# Anything that lands a binary under $PREFIX — used to decide whether
# the prefix-related setup (location prompt, PATH update, conflict
# detection, etc.) needs to run.
needs_prefix() {
    [ "$WANT_TCL" = "1" ] || [ "$WANT_F5" = "1" ]
}

choose_ai_components() {
    # Phase 2: which AI components to install. Sets WANT_MCP / WANT_SKILLS.
    # Short-circuits when no AI client was detected.
    WANT_MCP=0; WANT_SKILLS=0
    if [ "$HAS_CLAUDE" != "1" ] && [ "$HAS_CODEX" != "1" ]; then
        log "no AI client detected — skipping AI component selection"
        return
    fi
    # Env opt-outs.
    if [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && [ "${TCL_LSP_NO_SKILLS:-0}" = "1" ]; then
        log "AI components disabled via TCL_LSP_NO_MCP=1 TCL_LSP_NO_SKILLS=1"
        return
    fi
    has_skills=0
    [ "$HAS_CLAUDE" = "1" ] && has_skills=1

    # Defaults — install everything the env didn't disable.
    d_mcp=0; d_skills=0
    [ "${TCL_LSP_NO_MCP:-0}"    != "1" ] && d_mcp=1
    [ "${TCL_LSP_NO_SKILLS:-0}" != "1" ] && [ "$has_skills" = 1 ] && d_skills=1

    if ! tty_available \
       || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        WANT_MCP="$d_mcp"; WANT_SKILLS="$d_skills"
        log "AI (non-interactive): mcp=$WANT_MCP skills=$WANT_SKILLS"
        return
    fi

    if [ "$has_skills" = 1 ]; then
        menu_default=3
        print_line ""
        print_line "Choose which AI components to install:"
        print_line "  1) mcp"
        print_line "  2) skills"
        print_line "  3) both (mcp + skills)"
        print_line "  4) none"
        print_prompt "$(printf '  selection [%s]: ' "$menu_default")"
        read_user_line || reply=""
        ans="${reply:-$menu_default}"
        case "$ans" in
            1) WANT_MCP=1; WANT_SKILLS=0 ;;
            2) WANT_MCP=0; WANT_SKILLS=1 ;;
            3) WANT_MCP=1; WANT_SKILLS=1 ;;
            4) WANT_MCP=0; WANT_SKILLS=0 ;;
            *) die "invalid selection: $ans (expected 1..4)" ;;
        esac
    else
        menu_default=1
        print_line ""
        print_line "Choose AI components (Codex only — no skills):"
        print_line "  1) mcp"
        print_line "  2) none"
        print_prompt "$(printf '  selection [%s]: ' "$menu_default")"
        read_user_line || reply=""
        ans="${reply:-$menu_default}"
        case "$ans" in
            1) WANT_MCP=1 ;;
            2) WANT_MCP=0 ;;
            *) die "invalid selection: $ans (expected 1..2)" ;;
        esac
    fi
    prompt_record ai-plan "menu pick" "$ans"
    log "AI: mcp=$WANT_MCP skills=$WANT_SKILLS"
}

guard_install_plan() {
    if [ "$WANT_TCL" != "1" ] && [ "$WANT_F5" != "1" ] \
       && [ "$WANT_MCP" != "1" ] && [ "$WANT_SKILLS" != "1" ]; then
        die "nothing selected — at least one component must be picked"
    fi
    if [ "$WANT_MCP" = "1" ] && [ "$HAS_CLAUDE" != "1" ] && [ "$HAS_CODEX" != "1" ]; then
        warn "MCP requested but no AI client detected — installing anyway"
    fi
    if [ "$WANT_SKILLS" = "1" ] && [ "$HAS_CLAUDE" != "1" ]; then
        warn "skills requested but Claude Code not detected — installing anyway"
    fi
}


find_on_path() {
    # Print the first executable named $1 found on $PATH.
    name="$1"
    OLD_IFS="$IFS"; IFS=:
    # shellcheck disable=SC2086
    for d in $PATH; do
        if [ -x "$d/$name" ] && [ -f "$d/$name" ]; then
            IFS="$OLD_IFS"
            printf '%s\n' "$d/$name"
            return 0
        fi
    done
    IFS="$OLD_IFS"
    return 1
}

looks_like_our_zipapp() {
    # Tier 1: python shebang + ZIP signature. Tier 2: unzip or python
    # zipfile to look for our markers.
    f="$1"
    [ -r "$f" ] || return 1

    first="$(head -c 200 "$f" 2>/dev/null)"
    case "$first" in
        '#!'*python*) : ;;
        *) return 1 ;;
    esac
    head -c 2048 "$f" 2>/dev/null | grep -aq 'PK' || return 1

    # Tier 2a — unzip.  Both the post-restructure markers
    # (``shared/_build_info.py`` + ``tooling.{tcl,f5,wasm}.main`` /
    # ``server._build_info`` / ``server.server``) AND the legacy
    # pre-restructure markers (``lsp/_build_info.py`` +
    # ``explorer.{tcl,f5,wasm}_cli`` / ``lsp._build_info`` /
    # ``lsp.server``) match — both shapes were shipped on
    # different release tags and an installer rolling over either of
    # them must recognise it as ours, not refuse to overwrite.
    if have unzip; then
        if unzip -l -- "$f" 2>/dev/null \
            | grep -qE 'shared/_build_info\.py|lsp/_build_info\.py|static/index\.html'; then
            return 0
        fi
        if unzip -p -- "$f" __main__.py 2>/dev/null \
            | grep -qE 'tooling\.(tcl|f5|wasm|explorer\.cli)|tooling\.explorer|explorer\.(tcl|f5|wasm)_cli|ai\.mcp\.tcl_mcp_server|shared\._build_info|server\.(server|_build_info)|lsp\.(server|_build_info)|Tcl Compiler Explorer'; then
            return 0
        fi
        return 1
    fi

    # Tier 2b — Python zipfile
    if [ -n "${PYTHON:-}" ]; then
        if "$PYTHON" - "$f" >/dev/null 2>&1 <<'PY'
import sys, zipfile
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        names = set(z.namelist())
        if 'shared/_build_info.py' in names or 'lsp/_build_info.py' in names:
            sys.exit(0)
        # Explorer GUI zipapps bundle the static web payload — no
        # shared/_build_info.py, no Python source from the main tree.
        if 'static/index.html' in names:
            sys.exit(0)
        try:
            main = z.read('__main__.py').decode('utf-8', errors='replace')
        except KeyError:
            sys.exit(1)
        markers = (
            # Post-restructure entry points (Phase 4+).
            'tooling.tcl.main', 'tooling.f5.main', 'tooling.wasm.main',
            'tooling.explorer.cli', 'tooling.explorer',
            'server.server', 'server._build_info',
            'shared._build_info',
            # Pre-restructure entry points (shipped on earlier tags).
            'explorer.tcl_cli', 'explorer.f5_cli', 'explorer.wasm_cli',
            'lsp._build_info', 'lsp.server',
            # MCP server stays valid across the restructure.
            'ai.mcp.tcl_mcp_server',
            # Explorer GUI zipapp uses scripts/zipapp-main/gui.py as
            # its __main__ — match on the distinctive banner string.
            'Tcl Compiler Explorer',
        )
        for m in markers:
            if m in main:
                sys.exit(0)
except (zipfile.BadZipFile, FileNotFoundError, OSError):
    pass
sys.exit(1)
PY
        then
            return 0
        fi
        return 1
    fi

    # Neither unzip nor Python — can't confirm. Treat as not-ours so
    # downstream logic surfaces the safer "overwrite anyway?" prompt.
    return 1
}

# Find an existing tcl-lsp MCP server zipapp by checking, in order:
# Claude Code's registration, Codex's config.toml, $PATH.
find_existing_mcp() {
    if have claude; then
        line="$(claude mcp list 2>/dev/null | awk '$1 == "tcl-lsp"' | head -n 1)"
        if [ -n "$line" ]; then
            # The registered command is either `python …/….pyz` (zipapp) or an
            # absolute `…/tcl-mcp` (native binary) — match both.
            p="$(printf '%s\n' "$line" | grep -oE '/[A-Za-z0-9._/+~:-]+' \
                | grep -E '(\.pyz|/tcl-mcp)$' | head -n 1)"
            if [ -n "$p" ] && [ -r "$p" ]; then
                printf '%s\n' "$p"; return 0
            fi
        fi
    fi
    cfg="$HOME/.codex/config.toml"
    if [ -f "$cfg" ]; then
        p="$(awk '
            /^[[:space:]]*\[mcp_servers\.tcl_lsp\]/ { inside=1; next }
            inside && /^[[:space:]]*\[/             { inside=0 }
            # `command`/`args` may hold a `…/….pyz` (zipapp) or a native
            # `…/tcl-mcp` path — match either.
            inside && /"[^"]*(\.pyz|tcl-mcp)"/ {
                if (match($0, /"[^"]*(\.pyz|tcl-mcp)"/)) {
                    print substr($0, RSTART+1, RLENGTH-2); exit
                }
            }' "$cfg")"
        if [ -n "$p" ] && [ -r "$p" ]; then
            printf '%s\n' "$p"; return 0
        fi
    fi
    if p="$(find_on_path tcl-mcp)"; then
        printf '%s\n' "$p"; return 0
    fi
    if p="$(find_on_path tcl-lsp-mcp-server.pyz)"; then
        printf '%s\n' "$p"; return 0
    fi
    return 1
}

# The MCP artefact filename the install will write: the native `tcl-mcp`
# binary when a native build is available/selected, else the Python zipapp.
mcp_target_basename() {
    if [ "${TCL_LSP_MCP_PYZ:-0}" != "1" ] \
        && { [ -n "${TCL_LSP_MCP_BIN:-}" ] || host_triple >/dev/null 2>&1; }; then
        echo "tcl-mcp"
    else
        echo "tcl-lsp-mcp-server.pyz"
    fi
}

# Detect an existing MCP zipapp and offer to update it in place.
# Sets MCP_PREFIX_OVERRIDE.
looks_like_our_native_mcp() {
    # Our native MCP server is an executable named `tcl-mcp`.  Unlike the
    # zipapp (whose structure looks_like_our_zipapp inspects), a native binary
    # can't be introspected the same way, so match on the artefact name the
    # installer itself writes / registers.
    f="$1"
    [ -f "$f" ] && [ -x "$f" ] || return 1
    case "$(basename "$f")" in
        tcl-mcp | tcl-mcp.exe) return 0 ;;
        *) return 1 ;;
    esac
}

looks_like_our_native_cli() {
    # Our `tcl` and `f5` CLIs ship as native binaries, not
    # Python zipapps, so looks_like_our_zipapp can't recognise a prior
    # install of them. As with looks_like_our_native_mcp, a native binary
    # can't be introspected structurally, so match on the artefact names the
    # installer itself writes — `tcl`/`f5`, honouring any TCL_LSP_SUFFIX
    # (INSTALL_SUFFIX) and a Windows `.exe`.
    f="$1"
    [ -f "$f" ] && [ -x "$f" ] || return 1
    case "$(basename "$f")" in
        "tcl${INSTALL_SUFFIX}" | "tcl${INSTALL_SUFFIX}.exe") return 0 ;;
        "f5${INSTALL_SUFFIX}" | "f5${INSTALL_SUFFIX}.exe") return 0 ;;
        *) return 1 ;;
    esac
}

# True when $1 is an artefact this installer produces — a Python zipapp, the
# native MCP binary, or a native `tcl`/`f5` CLI. Used by the overwrite paths
# to decide whether an existing file at a target path is our own artefact
# (safe to replace in place) rather than an unrelated file.
looks_like_ours() {
    looks_like_our_zipapp "$1" \
        || looks_like_our_native_mcp "$1" \
        || looks_like_our_native_cli "$1"
}

propose_update_mcp() {
    [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && return 0
    p="$(find_existing_mcp)" || return 0
    if ! looks_like_our_zipapp "$p" && ! looks_like_our_native_mcp "$p"; then
        warn "found '$(basename "$p")' at $p but it doesn't look like our MCP server"
        return 0
    fi
    # Remember the prior install so cleanup_stale_mcp can remove it if this run
    # installs the other flavour (native ⇄ zipapp) or a copy elsewhere.
    PRIOR_MCP_PATH="$p"
    log "found existing MCP server at $p"
    if ask_optout "Update existing MCP server at $p (in place)? [Y/n]"; then
        MCP_PREFIX_OVERRIDE="$(dirname "$p")"
    fi
    return 0
}

# After a successful MCP install + registration, remove leftover copies so a
# machine doesn't accumulate stale MCP servers when switching flavours or
# locations:
#   * the OTHER flavour's artefact at the install prefix (old Python zipapp when
#     we just installed native, or an old native binary when we installed the
#     zipapp);
#   * a prior install detected elsewhere (PRIOR_MCP_PATH), when it's ours and
#     isn't the copy we just wrote.
# Only ever removes files we positively recognise as ours.
cleanup_stale_mcp() {
    [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && return 0
    [ -n "${MCP_PATH:-}" ] || return 0
    prefix="$(prefix_for mcp)"
    if [ "$MCP_NATIVE" = "1" ]; then
        other="$prefix/tcl-lsp-mcp-server.pyz"
    else
        other="$prefix/tcl-mcp"
    fi
    # The superseded other-flavour artefact at our prefix.
    if [ -e "$other" ] && [ "$other" != "$MCP_PATH" ]; then
        if looks_like_our_zipapp "$other" || looks_like_our_native_mcp "$other"; then
            rm -f "$other" && log "removed superseded MCP server $other"
        fi
    fi
    # A prior install elsewhere that we've now replaced.
    if [ -n "${PRIOR_MCP_PATH:-}" ] && [ "$PRIOR_MCP_PATH" != "$MCP_PATH" ] \
        && [ -e "$PRIOR_MCP_PATH" ]; then
        if looks_like_our_zipapp "$PRIOR_MCP_PATH" || looks_like_our_native_mcp "$PRIOR_MCP_PATH"; then
            rm -f "$PRIOR_MCP_PATH" && log "removed prior MCP server $PRIOR_MCP_PATH"
        fi
    fi
}

propose_update_clis() {
    # Offer to update each existing tcl/f5 install in place;
    # recorded per-binary in *_PREFIX_OVERRIDE for split-directory
    # layouts.
    [ "$PREFIX_EXPLICIT" = "1" ] && return 0

    # `|| return 0` because propose_update_one's non-zero is an internal
    # "fall back to picker" signal, not a failure to propagate.
    case "$ONLY" in
        tcl)  propose_update_one tcl  || return 0 ;;
        f5)   propose_update_one f5   || return 0 ;;
        both) propose_update_one tcl  || return 0
              propose_update_one f5   || return 0 ;;
    esac

    # Pick a PATH/completion anchor from whichever overrides got
    # populated, preferring tcl (the headline binary) then f5.
    # When the user is updating multiple binaries that live in
    # different dirs, the helper above already set per-binary
    # overrides; this just picks the one we'll log + use as $PREFIX.
    anchor=""
    [ -n "$TCL_PREFIX_OVERRIDE" ]          && anchor="$TCL_PREFIX_OVERRIDE"
    [ -z "$anchor" ] && [ -n "$F5_PREFIX_OVERRIDE" ]           && anchor="$F5_PREFIX_OVERRIDE"
    [ -z "$anchor" ] && return

    # Report split-directory updates when any pair of overrides
    # disagrees with the anchor.
    split=0
    for o in "$TCL_PREFIX_OVERRIDE" "$F5_PREFIX_OVERRIDE"; do
        if [ -n "$o" ] && [ "$o" != "$anchor" ]; then split=1; fi
    done
    if [ "$split" = 1 ]; then
        log "split-directory update:"
        [ -n "$TCL_PREFIX_OVERRIDE" ]          && log "  tcl              -> $TCL_PREFIX_OVERRIDE/tcl"
        [ -n "$F5_PREFIX_OVERRIDE" ]           && log "  f5               -> $F5_PREFIX_OVERRIDE/f5"
        log "(PATH / completion anchor on $anchor)"
    else
        log "updating in place: $anchor"
    fi
    set_prefix "$anchor"
    PREFIX_EXPLICIT=1
    PREFIX_FROM_UPDATE=1
}

propose_update_one() {
    # Returns non-zero to signal the caller should bail to the picker.
    n="$1"
    path="$(find_on_path "$n")" || return 0
    if looks_like_our_zipapp "$path"; then
        dir="$(dirname "$path")"
        log "found existing $n at $path"
        if ! ask_optout "Update existing $n at $path (in place)? [Y/n]"; then
            log "leaving $path alone; will pick a fresh install location"
            return 1
        fi
        case "$n" in
            tcl)              TCL_PREFIX_OVERRIDE="$dir" ;;
            f5)               F5_PREFIX_OVERRIDE="$dir" ;;
        esac
        return 0
    fi
    warn "found '$n' at $path but it doesn't look like our zipapp"
    warn "(missing python shebang or ZIP signature) — picker will run as usual"
    return 1
}


# Suffix applied to our binaries (e.g. "-lsp"). Set by detect_conflicts
# when the user picks "rename" or via the TCL_LSP_SUFFIX env override.
INSTALL_SUFFIX="${TCL_LSP_SUFFIX:-}"

alias_in_rc() {
    # Cheap, best-effort grep for an alias/abbr that would shadow $1.
    # Catches the common forms; doesn't follow source/sourced rc fragments.
    n="$1"
    for rc in "$RC" "$HOME/.profile" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"; do
        [ -f "$rc" ] || continue
        if grep -qE "^[[:space:]]*(alias[[:space:]]+|abbr([[:space:]]+--add)?[[:space:]]+)${n}[[:space:]=]" "$rc" 2>/dev/null; then
            printf '%s\n' "$rc"
            return 0
        fi
    done
    return 1
}

count_conflicts_for() {
    # Increment parent's $conflicts when $1 would be shadowed on PATH or by an alias.
    n="$1"
    own_dir="$(prefix_for "$n")"
    if other="$(find_on_path "$n")" && [ "$other" != "$own_dir/$n" ]; then
        if looks_like_our_zipapp "$other"; then
            warn "another tcl-lsp '$n' is already at $other (will shadow $own_dir/$n)"
        else
            warn "an unrelated '$n' exists at $other (will shadow $own_dir/$n)"
        fi
        conflicts=$((conflicts + 1))
    fi
    if rc_with_alias="$(alias_in_rc "$n")"; then
        warn "shell alias for '$n' in $rc_with_alias may shadow our install"
        conflicts=$((conflicts + 1))
    fi
}

detect_conflicts() {
    # Warn on PATH/alias shadows and offer keep / rename / abort.
    [ "${PREFIX_FROM_UPDATE:-0}" = "1" ] && return

    conflicts=0
    case "$ONLY" in
        tcl)  count_conflicts_for tcl ;;
        f5)   count_conflicts_for f5  ;;
        both) count_conflicts_for tcl; count_conflicts_for f5 ;;
    esac
    [ "$conflicts" = 0 ] && return

    if [ -n "$INSTALL_SUFFIX" ]; then
        log "applying TCL_LSP_SUFFIX: binaries will install as <name>${INSTALL_SUFFIX}"
        return
    fi
    if ! tty_available; then
        warn "(set TCL_LSP_SUFFIX=-lsp to install as <name>-lsp and avoid the shadow)"
        return
    fi

    print_prompt "$(printf '\nHow to proceed?\n  1) keep (install with original name; existing shadow remains)\n  2) rename (install as <name>-lsp)\n  3) abort\n  selection [1]: ')"
    read_user_line || reply=1
    case "${reply:-1}" in
        1|keep)   sel=keep ;;
        2|rename) sel=rename ;;
        3|abort)  sel=abort ;;
        *) die "invalid selection: $reply (expected 1, 2, or 3)" ;;
    esac
    case "$sel" in
        rename)
            INSTALL_SUFFIX="-lsp"
            log "binaries will install as <name>-lsp"
            ;;
        abort)
            die "aborted: naming conflict not resolved"
            ;;
        keep|*)
            log "keeping original names — be aware existing shadows remain in effect"
            ;;
    esac
}

choose_prefix() {
    if [ "$PREFIX_EXPLICIT" = "1" ]; then
        if [ "${PREFIX_FROM_UPDATE:-0}" != "1" ]; then
            log "install location (TCL_LSP_PREFIX): $PREFIX"
        fi
        return
    fi
    if ! tty_available; then
        log "install location (default): $PREFIX"
        return
    fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        log "install location (default): $PREFIX"
        return
    fi

    cands="$(collect_install_candidates)"

    print_line ""
    print_line "${BOLD}Choose install location:${RESET}"
    i=0
    OLD_IFS="$IFS"; IFS='
'
    for c in $cands; do
        i=$((i + 1))
        annot="$(annotate_candidate "$c")"
        marker=" "
        [ "$c" = "$PREFIX" ] && marker="*"
        print_line "$(printf '  %s %d) %-32s %s' "$marker" "$i" "$c" "$annot")"
    done
    IFS="$OLD_IFS"
    other_idx=$((i + 1))
    print_line "$(printf '    %d) Other (enter a path)' "$other_idx")"
    print_prompt "$(printf '\nSelection [%s]:' "$PREFIX")"
    read_user_line || reply=""
    ans="$reply"

    if [ -z "$ans" ]; then
        log "install location: $PREFIX"
        return
    fi
    case "$ans" in
        *[!0-9]*)
            set_prefix "$ans"
            ;;
        *)
            if [ "$ans" = "$other_idx" ]; then
                print_prompt 'Path:'
                read_user_line || reply=""
                custom="$reply"
                if [ -n "$custom" ]; then set_prefix "$custom"; fi
            else
                i=0
                IFS='
'
                for c in $cands; do
                    i=$((i + 1))
                    if [ "$i" = "$ans" ]; then set_prefix "$c"; break; fi
                done
                IFS="$OLD_IFS"
            fi
            ;;
    esac
    log "install location: $PREFIX"
}

choose_mcp_prefix() {
    # Pick the directory where the MCP server zipapp lands. Defaults to
    # the existing install dir (set by propose_update_mcp) when one was
    # detected; otherwise to $PREFIX (the CLI dir, or ~/.local/bin when
    # no CLI was installed).
    if [ -n "$MCP_PREFIX_OVERRIDE" ]; then
        log "MCP install location (existing): $MCP_PREFIX_OVERRIDE"
        return
    fi
    default="$PREFIX"
    if ! tty_available \
       || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        MCP_PREFIX_OVERRIDE="$default"
        log "MCP install location (default): $MCP_PREFIX_OVERRIDE"
        return
    fi

    print_line ""
    print_line "MCP server install directory:"
    print_prompt "$(printf '  path [%s]: ' "$default")"
    read_user_line || reply=""
    MCP_PREFIX_OVERRIDE="${reply:-$default}"
    # Validate via set_prefix-style charset check.
    case "$MCP_PREFIX_OVERRIDE" in
        '')                          die "MCP install location cannot be empty" ;;
        [!/]*)                       die "MCP install location must be absolute: $MCP_PREFIX_OVERRIDE" ;;
        *[!A-Za-z0-9._/+~-]*)        die "MCP install location contains unsupported characters: $MCP_PREFIX_OVERRIDE" ;;
    esac
    log "MCP install location: $MCP_PREFIX_OVERRIDE"
}

# Planned destination paths the user already approved overwriting. Used
# by write_target() to skip its existence check.
OVERWRITE_OK=""

plan_overwrite_check() {
    # Front-loaded: for each binary we plan to write, if a non-tcl-lsp
    # file already exists there, ask now whether to overwrite. Record
    # the decision so phase 3 doesn't have to ask again.
    dst="$1"
    [ -e "$dst" ] || return 0
    # Recognise every flavour of artefact we produce — zipapp, native MCP
    # binary, or native `tcl`/`f5` CLI — so an in-place update doesn't trip
    # the "unrelated file" prompt (which aborts non-interactive upgrades).
    looks_like_ours "$dst" && return 0
    warn "$dst already exists and is not a recognised tcl-lsp artefact"
    warn "(not our zipapp, native MCP binary, or native CLI)"
    if ! ask_default_no "Overwrite $dst anyway? [y/N]"; then
        die "aborted: existing $dst is unrelated and overwrite declined.
Re-run with TCL_LSP_PREFIX=/other/dir, or remove $dst first."
    fi
    OVERWRITE_OK="${OVERWRITE_OK} ${dst}"
}

write_target() {
    # Atomic install of SRC to DST. Phase 2 already settled writability
    # and overwrite consent (OVERWRITE_OK lists paths the user explicitly
    # approved overwriting); phase 3 catches anything that appeared
    # between phases.
    src="$1"; dst="$2"
    if [ -e "$dst" ] && ! looks_like_ours "$dst"; then
        case " $OVERWRITE_OK " in
            *" $dst "*) : ;;
            *) die "internal: $dst exists and was not pre-approved for overwrite (created between phase 2 and 3?)" ;;
        esac
    fi
    target_dir="$(dirname "$dst")"
    if dir_writable "$target_dir"; then
        mkdir -p "$target_dir"
        install -m 0755 "$src" "$dst"
    else
        run_as_root mkdir -p "$target_dir"
        run_as_root install -m 0755 "$src" "$dst"
    fi
}


install_cli() {
    # Install CLI $1 (tcl or f5) to its prefix_for() dir.  The CLIs ship as
    # native per-triple binaries (`tcl-<triple>` / `f5-query-<triple>`, per the
    # publish-native-binaries CI job); the Python zipapp builds are retired.
    name="$1"
    final_name="${name}${INSTALL_SUFFIX}"
    dir="$(prefix_for "$name")"
    ensure_tag
    # The published asset's base name: the `f5` CLI binary is `f5-query`.
    case "$name" in
        f5) asset_base="f5-query" ;;
        *)  asset_base="$name" ;;
    esac
    triple="$(host_triple)" || die "no native $name binary for \
$(uname -sm 2>/dev/null) — this platform has no published CLI build."
    asset="${asset_base}-${triple}"
    url="$(asset_url "$asset")"
    log "resolved $name -> $asset (tag $RESOLVED_TAG)"

    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    # Verify against SHA256SUMS when the asset is listed (fatal on mismatch);
    # otherwise install unverified with a warning — mirrors install_mcp_native.
    if ensure_sums && awk -v a="$asset" '$2==a || $2=="*"a {f=1} END{exit !f}' "$SUMS_PATH" 2>/dev/null; then
        verify_artefact "$asset" "$tmpfile"
    else
        warn "no checksum entry for $asset — installing unverified"
    fi
    write_target "$tmpfile" "$dir/$final_name"
    log "installed $name -> $dir/$final_name"
}


PATH_MARKER='# Added by tcl-lsp installer'
WANT_PATH_UPDATE=0

plan_path_update() {
    if path_contains "$PREFIX"; then return; fi
    if [ "${TCL_LSP_NO_PATH:-0}" = "1" ]; then
        warn "$PREFIX is not on PATH — TCL_LSP_NO_PATH=1, skipping rc update."
        return
    fi
    if [ -f "$RC" ] && grep -qF "$PATH_MARKER" "$RC" 2>/dev/null; then
        log "PATH entry already present in $RC"
        return
    fi
    if ask "Add $PREFIX to PATH in $RC? [Y/n]"; then
        WANT_PATH_UPDATE=1
    else
        warn "Skipped PATH update. Add this to $RC manually:"
        warn "  export PATH=\"$PREFIX:\$PATH\""
    fi
}

apply_path_update() {
    [ "$WANT_PATH_UPDATE" = 1 ] || return 0
    mkdir -p "$(dirname "$RC")"
    case "$SHELL_NAME" in
        fish)
            printf '\n%s\nfish_add_path %s\n' "$PATH_MARKER" "$PREFIX" >> "$RC"
            ;;
        *)
            # shellcheck disable=SC2016  # $PATH must be written verbatim
            printf '\n%s\nexport PATH="%s:$PATH"\n' "$PATH_MARKER" "$PREFIX" >> "$RC"
            ;;
    esac
    log "appended PATH entry to $RC"
}


WANT_COMP_TCL=0
WANT_COMP_F5=0
plan_completion() {
    # Front-loaded decision: install shell completion for each chosen CLI.
    [ "${TCL_LSP_NO_COMP:-0}" = "1" ] && return
    [ -n "$INSTALL_SUFFIX" ] && {
        log "shell completion: skipped (binaries install as <name>${INSTALL_SUFFIX})"
        return
    }
    case "$ONLY" in
        tcl|both)
            if ask "Install tcl shell completion for $SHELL_NAME? [Y/n]"; then
                WANT_COMP_TCL=1
            fi
            ;;
    esac
    case "$ONLY" in
        f5|both)
            if ask "Install f5 shell completion for $SHELL_NAME? [Y/n]"; then
                WANT_COMP_F5=1
            fi
            ;;
    esac
}

install_completion() {
    name="$1"
    case "$name" in
        tcl) [ "$WANT_COMP_TCL" = 1 ] || return 0 ;;
        f5)  [ "$WANT_COMP_F5"  = 1 ] || return 0 ;;
    esac
    bin="$(prefix_for "$name")/$name"
    case "$SHELL_NAME" in
        bash)
            dir="$HOME/.local/share/bash-completion/completions"
            comp_install "$bin" "$dir/$name" bash
            log "bash completion -> $dir/$name"
            ;;
        zsh)
            dir="${ZDOTDIR:-$HOME}/.zsh/completions"
            comp_install "$bin" "$dir/_$name" zsh
            log "zsh completion -> $dir/_$name"
            log "ensure your .zshrc has: fpath=($dir \$fpath) && autoload -Uz compinit && compinit"
            ;;
        fish)
            dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
            comp_install "$bin" "$dir/$name.fish" fish
            log "fish completion -> $dir/$name.fish"
            ;;
        *)
            warn "no completion template for $SHELL_NAME — skipping"
            ;;
    esac
}

comp_install() {
    cbin="$1"; cout="$2"; cshell="$3"
    mkdir -p "$(dirname "$cout")"
    "$cbin" completion "$cshell" > "$cout" \
        || warn "$(basename "$cbin") completion failed"
}


have_claude_cli() { have claude; }
have_codex_cli()  { have codex; }
has_claude_dir()  { [ -d "$HOME/.claude" ]; }
has_codex_dir()   { [ -d "$HOME/.codex" ]; }

AI_DETECTED=0
detect_ai_clients() {
    # Idempotent — choose_clis() and install_ai_integrations()
    # may both call it; only log the first time.
    [ "$AI_DETECTED" = "1" ] && return 0
    HAS_CLAUDE=0
    HAS_CODEX=0
    if [ "${TCL_LSP_NO_CLAUDE:-0}" != "1" ]; then
        if have_claude_cli || has_claude_dir; then HAS_CLAUDE=1; fi
    fi
    if [ "${TCL_LSP_NO_CODEX:-0}" != "1" ]; then
        if have_codex_cli || has_codex_dir; then HAS_CODEX=1; fi
    fi
    if [ "$HAS_CLAUDE" = "1" ] || [ "$HAS_CODEX" = "1" ]; then
        msg=""
        [ "$HAS_CLAUDE" = "1" ] && msg="${msg}Claude Code "
        [ "$HAS_CODEX"  = "1" ] && msg="${msg}Codex "
        log "detected AI client(s): ${msg}"
    fi
    AI_DETECTED=1
}

UNZIP_INSTALL_PLANNED=0
plan_unzip_if_needed() {
    # Called in phase 2 if skills install was requested. Records root need
    # without doing anything yet.
    [ "$WANT_SKILLS" = "1" ] || return 0
    have unzip && return 0
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        warn "unzip missing and TCL_LSP_NO_DEPS=1 — skills install will be skipped"
        return 0
    fi
    case "$OS" in
        macos)  warn "unzip missing on macOS — install Xcode Command Line Tools or skills will be skipped"; return 0 ;;
        debian|rhel|fedora|arch|alpine)
            need_root "install unzip via $(pkg_manager_label)" \
                      "install unzip manually, set TCL_LSP_NO_DEPS=1, or skip the skills install"
            UNZIP_INSTALL_PLANNED=1
            ;;
        *) warn "unzip not found and cannot auto-install on $OS — skills install will be skipped" ;;
    esac
}

ensure_unzip() {
    if have unzip; then return 0; fi
    [ "$UNZIP_INSTALL_PLANNED" = 1 ] || { warn "unzip unavailable — skipping skills install"; return 1; }
    case "$OS" in
        debian)        apt_get_update_once
                       run_as_root apt-get install -y unzip ;;
        rhel|fedora)   if have dnf; then run_as_root dnf install -y unzip
                       else run_as_root yum install -y unzip; fi ;;
        arch)          run_as_root pacman -Sy --noconfirm unzip ;;
        alpine)        run_as_root apk add --no-cache unzip ;;
    esac
    have unzip
}

MCP_PATH=""
# When set to 1, MCP_PATH is a native `tcl-mcp` binary (launched directly, no
# Python interpreter). Left 0 for the default Python zipapp.
MCP_NATIVE=0
# A prior MCP install detected during planning (propose_update_mcp), removed by
# cleanup_stale_mcp if this run replaces it.
PRIOR_MCP_PATH=""

# Map the host OS/arch to the Rust target triple used in the per-triple native
# release asset names (`tcl-mcp-<triple>`, `tcl-<triple>`, `f5-query-<triple>`;
# see the publish-native-binaries CI job). Prints the triple, or returns 1 for a
# platform without a published native binary.
host_triple() {
    _os="$(uname -s 2>/dev/null || echo unknown)"
    _arch="$(uname -m 2>/dev/null || echo unknown)"
    case "$_os" in
        Linux)
            case "$_arch" in
                x86_64 | amd64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
                riscv64)         echo "riscv64gc-unknown-linux-gnu" ;;
                *) return 1 ;;
            esac ;;
        Darwin)
            case "$_arch" in
                arm64 | aarch64) echo "aarch64-apple-darwin" ;;
                x86_64)          echo "x86_64-apple-darwin" ;;
                *) return 1 ;;
            esac ;;
        *) return 1 ;;
    esac
}

# Try to fetch + install the native `tcl-mcp` binary for the host platform.
# Returns 0 (and sets MCP_PATH + MCP_NATIVE) on success, 1 to fall back to the
# Python zipapp. Set TCL_LSP_MCP_PYZ=1 to force the zipapp.
install_mcp_native() {
    [ "${TCL_LSP_MCP_PYZ:-0}" = "1" ] && return 1
    triple="$(host_triple)" || {
        warn "no native tcl-mcp binary for $(uname -sm 2>/dev/null) — using Python zipapp"
        return 1
    }
    ensure_tag
    asset="tcl-mcp-${triple}"
    tmpfile="$WORKDIR/$asset"
    log "trying native MCP binary $asset"
    if ! try_download "$(asset_url "$asset")" "$tmpfile"; then
        warn "native MCP binary $asset not published for this release — using Python zipapp"
        return 1
    fi
    # Verify against SHA256SUMS when the asset is listed (fatal on mismatch);
    # otherwise install unverified with a warning.
    if ensure_sums && awk -v a="$asset" '$2==a || $2=="*"a {f=1} END{exit !f}' "$SUMS_PATH" 2>/dev/null; then
        verify_artefact "$asset" "$tmpfile"
    else
        warn "no checksum entry for $asset — installing unverified"
    fi
    MCP_PATH="$(prefix_for mcp)/tcl-mcp"
    write_target "$tmpfile" "$MCP_PATH"
    chmod +x "$MCP_PATH" 2>/dev/null || true
    MCP_NATIVE=1
    log "installed native MCP server -> $MCP_PATH"
    return 0
}

install_mcp_zipapp() {
    if [ -n "$MCP_PATH" ] && [ -x "$MCP_PATH" ]; then return 0; fi
    # Opt-in: register a prebuilt native `tcl-mcp` binary instead of the Python
    # zipapp (e.g. `TCL_LSP_MCP_BIN=$(pwd)/target/release/tcl-mcp ./install.sh`,
    # or after `make rust-mcp`). Skips the download entirely.
    if [ -n "${TCL_LSP_MCP_BIN:-}" ] && [ -x "${TCL_LSP_MCP_BIN}" ]; then
        MCP_PATH="${TCL_LSP_MCP_BIN}"
        MCP_NATIVE=1
        log "using native MCP server -> $MCP_PATH"
        return 0
    fi
    # Default: fetch the native binary for this platform; fall back to the
    # Python zipapp when unavailable (unsupported arch / older release).
    if install_mcp_native; then return 0; fi
    ensure_tag
    asset="tcl-lsp-mcp-server-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "downloading $asset"
    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    verify_artefact "$asset" "$tmpfile"
    MCP_PATH="$(prefix_for mcp)/tcl-lsp-mcp-server.pyz"
    write_target "$tmpfile" "$MCP_PATH"
    log "installed MCP server -> $MCP_PATH"
}

register_mcp_claude() {
    # Native binary launches directly; the Python zipapp needs the interpreter.
    if [ "$MCP_NATIVE" = "1" ]; then
        set -- "$MCP_PATH"
    else
        set -- "$PYTHON" "$MCP_PATH"
    fi
    # Capture prior entry so a failed add can be restored.
    if ! have_claude_cli; then
        warn "claude CLI not on PATH — add the MCP server manually:"
        warn "  claude mcp add tcl-lsp -- $*"
        return
    fi
    prior=""
    if claude mcp list 2>/dev/null | awk '{print $1}' | grep -qx 'tcl-lsp'; then
        # The exact `mcp list` format isn't a stable contract; capture
        # the full record so we can echo it back as restore guidance.
        prior="$(claude mcp list 2>/dev/null | awk '$1=="tcl-lsp" {sub(/^tcl-lsp[[:space:]]+/,""); print; exit}')"
        claude mcp remove tcl-lsp >/dev/null 2>&1 || true
    fi
    if claude mcp add tcl-lsp -- "$@" >/dev/null 2>&1; then
        log "registered MCP server with Claude Code (tcl-lsp)"
        return
    fi
    warn "claude mcp add failed"
    if [ -n "$prior" ]; then
        warn "prior registration was: $prior"
        warn "restore it manually if needed"
    fi
    warn "or register fresh with: claude mcp add tcl-lsp -- $*"
}

register_mcp_codex() {
    cfg="$HOME/.codex/config.toml"
    mkdir -p "$HOME/.codex"
    touch "$cfg"
    # Allow leading whitespace — TOML accepts indented section headers
    # and a hand-edited config may use them.
    if grep -qE '^[[:space:]]*\[mcp_servers\.tcl_lsp\]' "$cfg" 2>/dev/null; then
        log "Codex already has [mcp_servers.tcl_lsp] in $cfg — leaving as-is"
        return
    fi
    cp "$cfg" "${cfg}.bak.$(date +%Y%m%d%H%M%S)" 2>/dev/null || true
    # Escape \ and " so a path with TOML-meaningful chars can't break the file.
    mcp_escaped="$(toml_basic_escape "$MCP_PATH")"
    {
        printf '\n[mcp_servers.tcl_lsp]\n'
        if [ "$MCP_NATIVE" = "1" ]; then
            # Native binary: run it directly, no interpreter, no args.
            printf 'command = "%s"\n' "$mcp_escaped"
            printf 'args = []\n'
        else
            printf 'command = "%s"\n' "$(toml_basic_escape "$PYTHON")"
            printf 'args = ["%s"]\n'  "$mcp_escaped"
        fi
    } >> "$cfg"
    log "registered MCP server with Codex in $cfg"
}

toml_basic_escape() {
    # Escape \ and " for a TOML basic string.
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

install_claude_skills() {
    ensure_unzip || { warn "unzip unavailable — skipping skills install"; return 1; }
    ensure_tag
    asset="tcl-lsp-claude-skills-${VER_NO_V}.zip"
    url="$(asset_url "$asset")"
    if [ -d "$HOME/.claude/skills" ]; then
        existing="$(find "$HOME/.claude/skills" -mindepth 1 -maxdepth 1 -type d \
                    \( -name 'irule-*' -o -name 'tcl-*' -o -name 'tk-*' \) 2>/dev/null \
                    | wc -l | tr -d ' ')"
        if [ "$existing" -gt 0 ]; then
            log "found $existing existing tcl-lsp skill(s) in $HOME/.claude/skills/ (will update in place)"
        fi
    fi
    log "downloading $asset"
    tmpzip="$WORKDIR/$asset"
    download "$url" "$tmpzip"
    verify_artefact "$asset" "$tmpzip"
    extract_dir="$WORKDIR/claude-skills"
    mkdir -p "$extract_dir"
    unzip -q -o "$tmpzip" -d "$extract_dir"
    inner="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d | head -n1)"
    if [ -z "$inner" ]; then
        warn "could not locate extracted skill payload in $extract_dir"
        return 1
    fi
    mkdir -p "$HOME/.claude"

    # Snapshot existing ~/.claude/{skills,prompts,tcl-ai.pyz} before overwrite.
    if [ -d "$HOME/.claude/skills" ] || [ -d "$HOME/.claude/prompts" ]; then
        bak_stamp="$(date +%Y%m%d%H%M%S)"
        bak_dir="$HOME/.claude/.tcl-lsp-bak-${bak_stamp}"
        mkdir -p "$bak_dir"
        [ -d "$HOME/.claude/skills"  ] && cp -R "$HOME/.claude/skills"  "$bak_dir/" 2>/dev/null
        [ -d "$HOME/.claude/prompts" ] && cp -R "$HOME/.claude/prompts" "$bak_dir/" 2>/dev/null
        [ -f "$HOME/.claude/tcl-ai.pyz" ] && cp "$HOME/.claude/tcl-ai.pyz" "$bak_dir/" 2>/dev/null
        # Drop the backup dir entirely if nothing was actually staged.
        if [ -z "$(ls -A "$bak_dir" 2>/dev/null)" ]; then
            rmdir "$bak_dir" 2>/dev/null
        else
            log "backed up prior ~/.claude/{skills,prompts,tcl-ai.pyz} → $bak_dir"
        fi
    fi

    # Native skills bundle: just the skills tree (domain-knowledge prompts live
    # in skills/_prompts/; no Python tcl-ai.pyz — the skills drive the native
    # tcl-mcp MCP server + tcl/f5-query CLIs).
    [ -d "$inner/skills" ] && cp -R "$inner/skills" "$HOME/.claude/"
    # Remove a stale Python AI CLI left by an older installer.
    rm -f "$HOME/.claude/tcl-ai.pyz" 2>/dev/null || true
    n="$(find "$HOME/.claude/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')"
    log "installed Claude Code skills -> $HOME/.claude/skills/ ($n skills)"
}

install_ai_integrations() {
    # No further questions asked here — choose_ai_components already
    # set WANT_MCP / WANT_SKILLS, and choose_mcp_prefix set the path.
    if [ "$WANT_MCP" = "1" ]; then
        install_mcp_zipapp
        [ "$HAS_CLAUDE" = "1" ] && register_mcp_claude
        [ "$HAS_CODEX"  = "1" ] && register_mcp_codex
        cleanup_stale_mcp
    fi
    if [ "$WANT_SKILLS" = "1" ]; then
        install_claude_skills
    fi
}


main() {
    init_workdir
    log "tcl-lsp installer $INSTALLER_VERSION"

    # === PHASE 1: detect environment (no prompts, no installs) ===
    detect_os
    detect_shell
    detect_proxy
    plan_downloader              # record curl-install need if missing
    plan_python_if_needed        # record python-install need if missing
    detect_ai_clients

    # === PHASE 2: gather every decision the user needs to make ===
    choose_clis

    if needs_prefix; then
        propose_update_clis
        choose_prefix
        detect_conflicts
        plan_cli_runtime_dependencies
        plan_path_update
        plan_completion
        if ! dir_writable "$PREFIX"; then
            need_root "write to ${PREFIX}" \
                      "pick a writable directory (e.g. TCL_LSP_PREFIX=\$HOME/.local/bin)"
        fi
        # Check for non-tcl-lsp files at each planned target path.
        case "$ONLY" in
            tcl)  plan_overwrite_check "$(prefix_for tcl)/tcl${INSTALL_SUFFIX}" ;;
            f5)   plan_overwrite_check "$(prefix_for f5)/f5${INSTALL_SUFFIX}" ;;
            both) plan_overwrite_check "$(prefix_for tcl)/tcl${INSTALL_SUFFIX}"
                  plan_overwrite_check "$(prefix_for f5)/f5${INSTALL_SUFFIX}" ;;
        esac
    fi

    choose_ai_components
    guard_install_plan
    if [ "$WANT_MCP" = "1" ]; then
        propose_update_mcp
        choose_mcp_prefix
        mcp_target_dir="$(prefix_for mcp)"
        if ! dir_writable "$mcp_target_dir"; then
            need_root "write the MCP zipapp to ${mcp_target_dir}" \
                      "pick a writable MCP directory"
        fi
        plan_overwrite_check "${mcp_target_dir}/$(mcp_target_basename)"
    fi
    plan_unzip_if_needed

    # === PHASE 2.5: pre-flight — abort with consolidated root message ===
    require_root_or_die

    # === PHASE 3: execute the plan (no more questions) ===
    install_python
    install_downloader
    log "using Python: $PYTHON"

    if needs_prefix; then
        case "$ONLY" in
            tcl)  install_cli tcl ;;
            f5)   install_cli f5 ;;
            both) install_cli tcl; install_cli f5 ;;
            none) : ;;
            *) die "invalid ONLY=$ONLY" ;;
        esac
        install_cli_runtime_dependencies
        apply_path_update
    fi

    case "$ONLY" in
        tcl)  install_completion tcl ;;
        f5)   install_completion f5 ;;
        both) install_completion tcl; install_completion f5 ;;
    esac

    install_ai_integrations

    printf '\n%sInstall complete.%s\n' "$BOLD" "$RESET"
    if needs_prefix && ! path_contains "$PREFIX"; then
        # shellcheck disable=SC2016  # instruction text shown to user
        printf 'Open a new shell, or run:  %sexport PATH="%s:$PATH"%s\n' \
               "$BOLD" "$PREFIX" "$RESET"
    fi
    s="$INSTALL_SUFFIX"
    verify_parts=""
    case "$ONLY" in
        tcl)  verify_parts="tcl${s} --help" ;;
        f5)   verify_parts="f5${s} --help" ;;
        both) verify_parts="tcl${s} --help && f5${s} --help" ;;
    esac
    if [ -n "$verify_parts" ]; then
        printf 'Verify:  %s%s%s\n' "$BOLD" "$verify_parts" "$RESET"
    fi
    if [ "$WANT_MCP" = "1" ] || [ "$WANT_SKILLS" = "1" ]; then
        printf 'AI integrations: '
        sep=""
        [ "$WANT_MCP"    = "1" ] && { printf '%sMCP server' "$sep"; sep=", "; }
        [ "$WANT_SKILLS" = "1" ] && { printf '%sClaude skills' "$sep"; }
        printf '\n'
    fi
}

main "$@"
