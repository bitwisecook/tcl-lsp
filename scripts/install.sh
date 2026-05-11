#!/usr/bin/env sh
# tcl-lsp installer for the released `tcl` / `f5` CLIs, the MCP server,
# and the Claude Code skills bundle.
#
# Detects the host OS (macOS, Debian/Ubuntu, RHEL/CentOS/Fedora, Arch,
# Alpine), checks for a usable Python 3.10+, installs dependencies
# through the native package manager when missing (after confirming),
# prompts for an install directory (scanning $PATH for writable
# candidates), and optionally writes shell completion.
#
# Before prompting, looks for an existing `tcl` / `f5` on $PATH and
# offers to update in place. When the chosen install directory is not
# user-writable, asks explicitly (default NO) before escalating to sudo.
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
#   curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh | sh
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
#   TCL_LSP_NO_TUI          - 1 to force text prompts even when whiptail/dialog is present
#   TCL_LSP_QUIET           - 1 to suppress the per-prompt stderr breadcrumb
#                             (only the underlying TUI / text prompt is shown)
#   TCL_LSP_TRACE           - 1 to print extra trace lines for downloads etc.
#   TCL_LSP_SUFFIX          - suffix for installed binaries (e.g. "-lsp"; default: empty)
#
# Question + answer breadcrumb: every interactive prompt prints a
# single-line `ask: [kind] question -> answer` to stderr.  This survives
# whiptail/dialog clearing the screen, so an aborted run leaves a
# visible record in the user's scrollback (no logfile needed).  Set
# TCL_LSP_QUIET=1 to suppress.

set -eu
# IFS=\n only — defence against a poisoned caller IFS. Loops that need
# whitespace splitting save and restore locally.
IFS='
'

# Stamped at release time by the publish-checksums CI job. Unstamped
# copies (e.g. running from main) report as "dev".
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
# pinned via env so choose_install_plan() skips the prompt.
if [ -n "${TCL_LSP_ONLY:-}" ]; then
    ONLY="$TCL_LSP_ONLY"
    ONLY_EXPLICIT=1
else
    ONLY=both
    ONLY_EXPLICIT=0
fi

# Plan flags set by choose_install_plan().  All "" (unset) until then,
# at which point each becomes "1" or "0".  install_cli / install_ai_*
# read these instead of asking again later.
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
# pair survives whiptail/dialog clearing the screen.  TCL_LSP_QUIET=1
# silences these (e.g. for CI runs that re-display the answer themselves).
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

UI=text                 # text | whiptail | dialog (set by ensure_ui)
TTY_IN=""               # stdin | /dev/tty | none (set by detect_tty)
TTY_OUT=""              # stderr | /dev/tty | none
TTY_PROBED=0
TUI_TITLE='tcl-lsp installer'

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

read_user_line() {
    tty_available || return 1
    if [ "$TTY_IN" = stdin ]; then IFS= read -r reply
    else IFS= read -r reply </dev/tty
    fi
}

# Probe whiptail/dialog when we have a path to the user. TCL_LSP_NO_TUI=1
# forces text. Always returns 0; no-TTY is a normal mode.
ensure_ui() {
    [ "${TCL_LSP_NO_TUI:-0}" = "1" ] && return 0
    tty_available || return 0
    if command -v whiptail >/dev/null 2>&1; then UI=whiptail
    elif command -v dialog >/dev/null 2>&1; then UI=dialog
    fi
    return 0
}

# whiptail/dialog need a TTY for the curses UI; redirect stdin from
# /dev/tty when our stdin is piped.
tui_run() {
    if [ "$TTY_IN" = stdin ]; then
        "$UI" --title "$TUI_TITLE" "$@"
    else
        "$UI" --title "$TUI_TITLE" "$@" </dev/tty
    fi
}

# yes/no — default-yes highlight on the dialog backend.
tui_yesno() {
    tty_available || { prompt_record yesno "$1" "no-tty"; return 1; }
    case "$UI" in
        whiptail|dialog)
            if tui_run --yesno "$1" 10 70; then
                prompt_record yesno "$1" yes
                return 0
            fi
            prompt_record yesno "$1" no
            return 1
            ;;
        *)
            print_prompt "$1 [Y/n]"
            read_user_line || { prompt_record yesno "$1" "read-failed"; return 1; }
            case "$reply" in
                ''|y|Y|yes|YES|Yes)
                    prompt_record yesno "$1" "yes (${reply:-default})"
                    return 0 ;;
                *)
                    prompt_record yesno "$1" "no ($reply)"
                    return 1 ;;
            esac
            ;;
    esac
}

tui_menu() {
    # tui_menu "prompt" tag1 desc1 tag2 desc2 ...   (must come in pairs)
    # Prints the chosen tag to stdout; non-zero on cancel.
    tty_available || { prompt_record menu "$1" "no-tty"; return 1; }
    prompt="$1"; shift
    case "$UI" in
        whiptail|dialog)
            count=$(($# / 2))
            if chosen="$(tui_run --menu "$prompt" $((count + 8)) 70 "$count" "$@" 3>&1 1>&2 2>&3)"; then
                prompt_record menu "$prompt" "$chosen"
                printf '%s\n' "$chosen"
                return 0
            fi
            prompt_record menu "$prompt" "cancel"
            return 1
            ;;
        *)
            print_prompt "$(printf '\n%s\n' "$prompt")"
            i=0
            while [ $# -gt 0 ]; do
                i=$((i + 1))
                tag="$1"; desc="$2"; shift 2
                eval "tag_$i=\"\$tag\""
                print_prompt "$(printf '  %d) %-32s %s\n' "$i" "$tag" "$desc")"
            done
            print_prompt 'Selection [1]:'
            read_user_line || reply=1
            : "${reply:=1}"
            case "$reply" in
                *[!0-9]*) prompt_record menu "$prompt" "invalid:$reply"; return 1 ;;
            esac
            eval "chosen=\"\${tag_$reply:-}\""
            prompt_record menu "$prompt" "$chosen"
            printf '%s\n' "$chosen"
            ;;
    esac
}

# checklist — one tag per line on stdout for ticked entries.
tui_checklist() {
    tty_available || { prompt_record checklist "$1" "no-tty"; return 1; }
    prompt="$1"; shift
    case "$UI" in
        whiptail|dialog)
            count=$(($# / 3))
            if out="$(tui_run --separate-output --checklist "$prompt" \
                        $((count + 8)) 70 "$count" "$@" 3>&1 1>&2 2>&3)"; then
                # Render answer as space-separated tags for the transcript.
                ans="$(printf '%s' "$out" | tr '\n' ' ' | sed 's/ *$//')"
                prompt_record checklist "$prompt" "${ans:-(none)}"
                printf '%s\n' "$out"
                return 0
            fi
            prompt_record checklist "$prompt" "cancel"
            return 1
            ;;
        *)
            defaults=""
            args=""
            while [ $# -gt 0 ]; do
                tag="$1"; desc="$2"; state="$3"; shift 3
                args="${args}${tag}|${desc}
"
                [ "$state" = ON ] && defaults="${defaults}${defaults:+,}${tag}"
            done
            print_prompt "$(printf '\n%s\n' "$prompt")"
            print_prompt "$(printf '%s' "$args" | awk -F'|' '{ printf "  %-24s %s\n", $1, $2 }')"
            print_prompt "Enable [$defaults]:"
            read_user_line || reply=""
            : "${reply:=$defaults}"
            prompt_record checklist "$prompt" "${reply:-(none)}"
            printf '%s\n' "$reply" | tr ',' '\n' | sed 's/^ *//; s/ *$//; /^$/d'
            ;;
    esac
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

# Opt-in prompt — default no when headless.
ask() {
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}"  = "1" ]; then prompt_record ask "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    tty_available || { prompt_record ask "$1" "no (no-tty, default-no)"; return 1; }
    tui_yesno "$1"
}

# Opt-out prompt — default yes when headless (upstream signal was the opt-in).
ask_optout() {
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then prompt_record ask-optout "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask-optout "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    tty_available || { prompt_record ask-optout "$1" "yes (no-tty, default-yes)"; return 0; }
    tui_yesno "$1"
}

# Strict default-NO — for privilege escalation and destructive choices.
# Empty input, headless run, dialog cancel all count as no.
ask_default_no() {
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then prompt_record ask-no "$1" "yes (TCL_LSP_ASSUME_YES)"; return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then prompt_record ask-no "$1" "no (TCL_LSP_ASSUME_NO)"; return 1; fi
    tty_available || { prompt_record ask-no "$1" "no (no-tty, default-no)"; return 1; }
    case "$UI" in
        whiptail|dialog)
            if tui_run --defaultno --yesno "$1" 10 70; then
                prompt_record ask-no "$1" yes
                return 0
            fi
            prompt_record ask-no "$1" no
            return 1
            ;;
        *)
            print_prompt "$1"
            read_user_line || { prompt_record ask-no "$1" "read-failed"; return 1; }
            case "$reply" in
                y|Y|yes|YES|Yes) prompt_record ask-no "$1" "yes ($reply)"; return 0 ;;
                *) prompt_record ask-no "$1" "no (${reply:-default})"; return 1 ;;
            esac
            ;;
    esac
}

# Surface every sudo/doas escalation.
confirm_root_action() {
    if [ "$(id -u)" = 0 ] || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then
        return 0
    fi
    warn "Next step needs root: $1"
    ask "Proceed? [Y/n]" || die "aborted: root step declined"
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
        *[!A-Za-z0-9._/+~:-]*)
            die "install location contains unsupported characters: $p
Allowed: A-Z a-z 0-9 . _ / + ~ : - (no quotes, spaces, dollars, newlines)" ;;
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
        tcl) printf '%s' "${TCL_PREFIX_OVERRIDE:-$PREFIX}" ;;
        f5)  printf '%s' "${F5_PREFIX_OVERRIDE:-$PREFIX}" ;;
        mcp) printf '%s' "${MCP_PREFIX_OVERRIDE:-$PREFIX}" ;;
        *)   printf '%s' "$PREFIX" ;;
    esac
}

# Read /etc/os-release (uid-0, not world-writable) and set $ID/$ID_LIKE.
read_os_release_safe() {
    f=/etc/os-release
    [ -e "$f" ] || return 1
    # All four predicates are POSIX or universally supported (BusyBox/
    # BSD/GNU). If find itself is unusable, the user has TCL_LSP_OS as
    # an explicit escape hatch.
    safe="$(find -L "$f" -maxdepth 0 -uid 0 ! -perm -002 -print 2>/dev/null)"
    if [ -z "$safe" ]; then
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

run_root() {
    if [ "$(id -u)" = 0 ]; then
        "$@"
    elif have sudo; then
        sudo "$@"
    elif have doas; then
        doas "$@"
    else
        die "Need root for: $*  (install sudo, or run this script as root)"
    fi
}

install_python() {
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Python 3.10+ not found and TCL_LSP_NO_DEPS=1 — install Python manually and retry."
    fi
    log "Python 3.10+ not found — attempting install via package manager"
    case "$OS" in
        macos)
            if ! have brew; then
                die "Homebrew not found. Install from https://brew.sh and re-run."
            fi
            brew install python@3.14 || brew install python@3.13 || brew install python3
            ;;
        debian)
            confirm_root_action "apt-get update && apt-get install -y python3 ca-certificates curl"
            run_root apt-get update
            run_root apt-get install -y python3 ca-certificates curl
            ;;
        rhel)
            # RHEL 9 / Rocky 9 / Alma 9 ship Python 3.9 as `python3`;
            # try a versioned 3.10+ interpreter from AppStream first.
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y ca-certificates curl"
            run_root "$PM" install -y ca-certificates curl
            installed=0
            for v in python3.12 python3.11; do
                # Probe availability quietly, then install loudly.
                if "$PM" list --available "$v" >/dev/null 2>&1 \
                   || "$PM" info "$v" >/dev/null 2>&1; then
                    confirm_root_action "$PM install -y $v"
                    if run_root "$PM" install -y "$v"; then installed=1; break; fi
                fi
            done
            if [ "$installed" = 0 ]; then
                confirm_root_action "$PM install -y python3"
                run_root "$PM" install -y python3
            fi
            ;;
        fedora)
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y python3 ca-certificates curl"
            run_root "$PM" install -y python3 ca-certificates curl
            ;;
        arch)
            confirm_root_action "pacman -Sy --noconfirm python ca-certificates curl"
            run_root pacman -Sy --noconfirm python ca-certificates curl
            ;;
        alpine)
            confirm_root_action "apk add --no-cache python3 ca-certificates curl"
            run_root apk add --no-cache python3 ca-certificates curl
            ;;
        *)
            die "Cannot auto-install Python on this OS. Install Python 3.10+ manually and retry." ;;
    esac

    find_python || die "Python install completed but no python3.10+ found on PATH."
}

ensure_curl() {
    # Prefer curl (--proto-redir-pins) over wget; install curl if neither present.
    if have curl; then DOWNLOADER="curl"; return; fi
    if have wget; then DOWNLOADER="wget"; return; fi
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Neither curl nor wget found and TCL_LSP_NO_DEPS=1 — install one and retry."
    fi
    case "$OS" in
        macos)  die "curl missing on macOS — install Xcode Command Line Tools and retry." ;;
        debian) confirm_root_action "apt-get install -y curl"; run_root apt-get install -y curl ;;
        rhel|fedora)
            if have dnf; then
                confirm_root_action "dnf install -y curl"; run_root dnf install -y curl
            else
                confirm_root_action "yum install -y curl"; run_root yum install -y curl
            fi ;;
        arch)   confirm_root_action "pacman -Sy --noconfirm curl"; run_root pacman -Sy --noconfirm curl ;;
        alpine) confirm_root_action "apk add --no-cache curl";     run_root apk add --no-cache curl ;;
        *)      die "Need curl or wget to download release artefacts." ;;
    esac
    have curl || die "curl install failed."
    DOWNLOADER="curl"
}


pkg_name_for() {
    # OS-package name for CMD on OS. Empty = no equivalent on this OS.
    os="$1"; cmd="$2"
    case "$os:$cmd" in
        # whiptail (newt on RHEL/Fedora, libnewt on Arch, none on macOS)
        rhel:whiptail|fedora:whiptail) printf 'newt' ;;
        arch:whiptail)                 printf 'libnewt' ;;
        macos:whiptail)                printf '' ;;

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
            if brew install "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via brew"
            else
                warn "brew install $pkg failed"
            fi
            ;;
        debian)
            confirm_root_action "apt-get install -y $pkg"
            if run_root apt-get install -y "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via apt-get"
            else
                warn "apt install $pkg failed"
            fi
            ;;
        rhel|fedora)
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y $pkg"
            if run_root "$PM" install -y "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via $PM"
            else
                warn "$PM install $pkg failed"
            fi
            ;;
        arch)
            confirm_root_action "pacman -Sy --noconfirm $pkg"
            if run_root pacman -Sy --noconfirm "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via pacman"
            else
                warn "pacman install $pkg failed"
            fi
            ;;
        alpine)
            confirm_root_action "apk add --no-cache $pkg"
            if run_root apk add --no-cache "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via apk"
            else
                warn "apk install $pkg failed"
            fi
            ;;
        *)
            warn "no package manager known for $OS — install $pkg manually" ;;
    esac
}

check_cli_runtime_dependencies() {
    # Survey external tools the tcl/f5 CLIs may shell out to.
    [ "${TCL_LSP_NO_DEPS:-0}" = "1" ] && return

    needs_tclsh=0      # tcl pkg / tcl venv / `tcl explore` against real tclsh
    needs_ssh=0        # f5 fetch (required for that verb's SSH transport)
    needs_sshpass=0    # f5 fetch password auth (optional fallback)
    needs_tshark=0     # f5 explain-flow --tshark, enrich-pcapng, pcap-remap

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

    log "$ONLY CLI runtime dependencies missing ($total):"
    [ "$needs_tclsh"   = 1 ] && log "  tclsh    — tcl pkg / tcl venv / tcl explore against the real interpreter"
    [ "$needs_ssh"     = 1 ] && log "  openssh  — f5 fetch over SSH (required for that verb)"
    [ "$needs_sshpass" = 1 ] && log "  sshpass  — f5 fetch with password auth (optional fallback to keys)"
    [ "$needs_tshark"  = 1 ] && log "  tshark   — f5 explain-flow / enrich-pcapng / pcap-remap libpcap support"

    if ! ask_default_no "Install missing CLI runtime dependencies via the package manager? [y/N]"; then
        log "skipping CLI runtime dependency install — features above will be unavailable"
        return
    fi

    [ "$needs_tclsh"   = 1 ] && install_pkg tclsh
    [ "$needs_ssh"     = 1 ] && install_pkg ssh
    [ "$needs_sshpass" = 1 ] && install_pkg sshpass
    [ "$needs_tshark"  = 1 ] && install_pkg tshark
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

curl_invoke() {
    # curl with the current transport-strictness, plus the caller's args.
    if [ "$TLS_FALLBACK" = 1 ]; then
        curl --tlsv1.2 --http1.1 --proto '=https' --proto-redir '=https' "$@"
    else
        curl --tlsv1.3 --http2  --proto '=https' --proto-redir '=https' "$@"
    fi
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
        final_url="$(curl_invoke -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url" 2>/dev/null)"
        if [ -z "$final_url" ] && maybe_fallback_tls; then
            final_url="$(curl_invoke -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url" 2>/dev/null)"
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
    trap 'rm -rf -- "$WORKDIR"' EXIT INT TERM HUP
}

SUMS_PATH=""
SUMS_STATE=""   # "present" | "absent" | ""
ensure_sums() {
    # Download SHA256SUMS once. Missing SUMS aborts the install by
    # default; set TCL_LSP_NO_VERIFY=1 to install without integrity
    # checks (e.g. for older releases that predate the SUMS publish
    # pipeline — see scripts/backfill-sums.sh for how to retro-publish).
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
and TCL_LSP_REQUIRE_COSIGN=1. Refusing to proceed with hash-only verification.
Older releases that predate the publish-checksums job may need the bundle
backfilled — see scripts/backfill-sums.sh --sign."
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
        perms="needs sudo"
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

choose_install_plan() {
    # Single front-loaded question that picks every component up front:
    # tcl CLI, f5 CLI, MCP server (when an AI client is detected), and
    # the Claude Code skills (when Claude is detected).  All later
    # phases read WANT_TCL / WANT_F5 / WANT_MCP / WANT_SKILLS and skip
    # any further prompting.
    #
    # Honoured env opt-outs (no prompt fired when set):
    #   TCL_LSP_ONLY=tcl|f5|both, TCL_LSP_NO_MCP=1, TCL_LSP_NO_SKILLS=1,
    #   TCL_LSP_NO_CLAUDE=1, TCL_LSP_NO_CODEX=1.
    detect_ai_clients

    # Compute defaults from env / detection.
    case "$ONLY" in
        tcl)  d_tcl=ON; d_f5=OFF ;;
        f5)   d_tcl=OFF; d_f5=ON ;;
        both|*) d_tcl=ON; d_f5=ON ;;
    esac
    d_mcp=OFF
    d_skills=OFF
    if [ "${TCL_LSP_NO_MCP:-0}" != "1" ] && { [ "$HAS_CLAUDE" = "1" ] || [ "$HAS_CODEX" = "1" ]; }; then
        d_mcp=ON
    fi
    if [ "${TCL_LSP_NO_SKILLS:-0}" != "1" ] && [ "$HAS_CLAUDE" = "1" ]; then
        d_skills=ON
    fi

    # Non-interactive paths take the defaults.  We still record them in
    # the transcript so a head-less run leaves a clear paper trail.
    if [ "$ONLY_EXPLICIT" = "1" ] || ! tty_available \
       || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        case "$ONLY" in
            both) WANT_TCL=1; WANT_F5=1 ;;
            tcl)  WANT_TCL=1; WANT_F5=0 ;;
            f5)   WANT_TCL=0; WANT_F5=1 ;;
            *) die "invalid TCL_LSP_ONLY: $ONLY (expected tcl|f5|both)" ;;
        esac
        [ "$d_mcp"    = ON ] && WANT_MCP=1    || WANT_MCP=0
        [ "$d_skills" = ON ] && WANT_SKILLS=1 || WANT_SKILLS=0
        log "install plan (non-interactive): tcl=$WANT_TCL f5=$WANT_F5 mcp=$WANT_MCP skills=$WANT_SKILLS"
        return
    fi

    case "$UI" in
        whiptail|dialog)
            # Build a single checklist; entries are skipped when the
            # underlying integration isn't applicable (no AI client).
            set -- tcl "Unified Tcl tools (format, lint, opt, ...)" "$d_tcl" \
                   f5  "F5 BIG-IP tools (cleanup, irule, redact, ...)" "$d_f5"
            if [ "$HAS_CLAUDE" = "1" ] || [ "$HAS_CODEX" = "1" ]; then
                if [ "$HAS_CLAUDE" = "1" ] && [ "$HAS_CODEX" = "1" ]; then
                    ai_label="MCP server (Claude + Codex)"
                elif [ "$HAS_CLAUDE" = "1" ]; then
                    ai_label="MCP server (Claude)"
                else
                    ai_label="MCP server (Codex)"
                fi
                set -- "$@" mcp "$ai_label" "$d_mcp"
            fi
            if [ "$HAS_CLAUDE" = "1" ]; then
                set -- "$@" skills "Claude Code skills (irule-*, tcl-*, tk-*)" "$d_skills"
            fi
            sel="$(tui_checklist "Choose what to install:" "$@")" \
                || die "aborted at install-plan selection"
            WANT_TCL=0; WANT_F5=0; WANT_MCP=0; WANT_SKILLS=0
            for t in $sel; do
                case "$t" in
                    tcl)    WANT_TCL=1 ;;
                    f5)     WANT_F5=1 ;;
                    mcp)    WANT_MCP=1 ;;
                    skills) WANT_SKILLS=1 ;;
                esac
            done
            ;;
        *)
            # Text mode: a numbered menu. Entries are dynamic based on
            # which AI clients (if any) were detected. With both clients
            # detected the full list is:
            #   1) tcl   2) f5   3) mcp   4) skills
            #   5) both CLIs   6) both AI   7) all
            WANT_TCL=0; WANT_F5=0; WANT_MCP=0; WANT_SKILLS=0
            has_ai=0; has_skills=0
            if [ "$HAS_CLAUDE" = "1" ] || [ "$HAS_CODEX" = "1" ]; then has_ai=1; fi
            if [ "$HAS_CLAUDE" = "1" ]; then has_skills=1; fi

            i=0
            menu_labels=""
            add_menu() {
                i=$((i + 1))
                menu_labels="${menu_labels}${menu_labels:+
}  $i) $2"
                eval "menu_key_$i=\"$1\""
            }
            add_menu tcl "tcl"
            add_menu f5  "f5"
            [ "$has_ai" = 1 ]     && add_menu mcp    "mcp"
            [ "$has_skills" = 1 ] && add_menu skills "skills"
            add_menu cli "both CLIs (tcl + f5)"
            if [ "$has_ai" = 1 ] && [ "$has_skills" = 1 ]; then
                add_menu ai "both AI (mcp + skills)"
            fi
            [ "$has_ai" = 1 ] && add_menu all "all"
            menu_default="$i"   # most comprehensive entry

            print_prompt "$(printf '\nChoose what to install:\n%s\n  selection [%s]: ' "$menu_labels" "$menu_default")"
            read_user_line || reply=""
            ans="${reply:-$menu_default}"
            case "$ans" in
                ''|*[!0-9]*) die "invalid selection: $ans (expected a number 1..$i)" ;;
            esac
            if [ "$ans" -lt 1 ] || [ "$ans" -gt "$i" ]; then
                die "invalid selection: $ans (out of range 1..$i)"
            fi
            sel_key="$(eval "echo \"\$menu_key_$ans\"")"
            case "$sel_key" in
                tcl)     WANT_TCL=1 ;;
                f5)      WANT_F5=1 ;;
                mcp)     WANT_MCP=1 ;;
                skills)  WANT_SKILLS=1 ;;
                cli)     WANT_TCL=1; WANT_F5=1 ;;
                ai)      WANT_MCP=1; WANT_SKILLS=1 ;;
                all)
                    WANT_TCL=1; WANT_F5=1
                    [ "$has_ai" = 1 ]     && WANT_MCP=1
                    [ "$has_skills" = 1 ] && WANT_SKILLS=1
                    ;;
            esac
            prompt_record install-plan "menu pick (1..$i)" "$ans ($sel_key)"
            ;;
    esac

    # Nothing selected at all is the only invalid plan.
    if [ "$WANT_TCL" != "1" ] && [ "$WANT_F5" != "1" ] \
       && [ "$WANT_MCP" != "1" ] && [ "$WANT_SKILLS" != "1" ]; then
        die "nothing selected — at least one component must be picked"
    fi
    # Skills/MCP without their AI client is harmless but pointless.
    if [ "$WANT_MCP" = "1" ] && [ "$HAS_CLAUDE" != "1" ] && [ "$HAS_CODEX" != "1" ]; then
        warn "MCP requested but no AI client detected — installing anyway"
    fi
    if [ "$WANT_SKILLS" = "1" ] && [ "$HAS_CLAUDE" != "1" ]; then
        warn "skills requested but Claude Code not detected — installing anyway"
    fi

    # Reflect CLI plan back into ONLY. `none` means "AI-only install".
    if   [ "$WANT_TCL" = "1" ] && [ "$WANT_F5" = "1" ]; then ONLY=both
    elif [ "$WANT_TCL" = "1" ]; then ONLY=tcl
    elif [ "$WANT_F5"  = "1" ]; then ONLY=f5
    else ONLY=none
    fi
    log "install plan: tcl=$WANT_TCL f5=$WANT_F5 mcp=$WANT_MCP skills=$WANT_SKILLS"
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

    # Tier 2a — unzip
    if have unzip; then
        if unzip -l -- "$f" 2>/dev/null | grep -q 'lsp/_build_info.py'; then
            return 0
        fi
        if unzip -p -- "$f" __main__.py 2>/dev/null \
            | grep -qE 'explorer\.(tcl|f5|wasm)_cli|ai\.mcp\.tcl_mcp_server|lsp\._build_info|lsp\.server'; then
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
        if 'lsp/_build_info.py' in set(z.namelist()):
            sys.exit(0)
        try:
            main = z.read('__main__.py').decode('utf-8', errors='replace')
        except KeyError:
            sys.exit(1)
        for m in (
            'explorer.tcl_cli', 'explorer.f5_cli', 'explorer.wasm_cli',
            'ai.mcp.tcl_mcp_server', 'lsp._build_info', 'lsp.server',
        ):
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
            p="$(printf '%s\n' "$line" | grep -oE '/[A-Za-z0-9._/+~:-]+\.pyz' | head -n 1)"
            if [ -n "$p" ] && [ -r "$p" ]; then
                printf '%s\n' "$p"; return 0
            fi
        fi
    fi
    cfg="$HOME/.codex/config.toml"
    if [ -f "$cfg" ]; then
        p="$(awk '
            /^[[:space:]]*\[mcp_servers\.tcl_lsp\]/ { in=1; next }
            in && /^[[:space:]]*\[/                 { in=0 }
            in && /\.pyz/ {
                if (match($0, /"[^"]+\.pyz"/)) {
                    print substr($0, RSTART+1, RLENGTH-2); exit
                }
            }' "$cfg")"
        if [ -n "$p" ] && [ -r "$p" ]; then
            printf '%s\n' "$p"; return 0
        fi
    fi
    if p="$(find_on_path tcl-lsp-mcp-server.pyz)"; then
        printf '%s\n' "$p"; return 0
    fi
    return 1
}

# Detect an existing MCP zipapp and offer to update it in place.
# Sets MCP_PREFIX_OVERRIDE.
propose_update_mcp() {
    [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && return 0
    p="$(find_existing_mcp)" || return 0
    if ! looks_like_our_zipapp "$p"; then
        warn "found '$(basename "$p")' at $p but it doesn't look like our zipapp"
        return 0
    fi
    log "found existing MCP server at $p"
    if ask_optout "Update existing MCP server at $p (in place)? [Y/n]"; then
        MCP_PREFIX_OVERRIDE="$(dirname "$p")"
    fi
    return 0
}

propose_update_install() {
    # Offer to update each existing tcl/f5/MCP install in place;
    # recorded per-artefact in *_PREFIX_OVERRIDE for split-directory layouts.
    # MCP detection runs regardless of CLI-PREFIX state because it has
    # its own override.
    propose_update_mcp

    [ "$PREFIX_EXPLICIT" = "1" ] && return 0

    # `|| return 0` because propose_update_one's non-zero is an internal
    # "fall back to picker" signal, not a failure to propagate.
    case "$ONLY" in
        tcl)  propose_update_one tcl  || return 0 ;;
        f5)   propose_update_one f5   || return 0 ;;
        both) propose_update_one tcl  || return 0
              propose_update_one f5   || return 0 ;;
        *)    return 0 ;;
    esac

    # No CLI was found on PATH — choose_prefix handles the picker.
    if [ -z "$TCL_PREFIX_OVERRIDE" ] && [ -z "$F5_PREFIX_OVERRIDE" ]; then
        return
    fi

    # $PREFIX anchors PATH/completion/MCP; per-CLI overrides still apply per binary.
    if [ -n "$TCL_PREFIX_OVERRIDE" ] && [ -n "$F5_PREFIX_OVERRIDE" ] \
       && [ "$TCL_PREFIX_OVERRIDE" != "$F5_PREFIX_OVERRIDE" ]; then
        log "split-directory update:"
        log "  tcl will update at $TCL_PREFIX_OVERRIDE/tcl"
        log "  f5  will update at $F5_PREFIX_OVERRIDE/f5"
        log "(PATH / completion / MCP install will anchor on $TCL_PREFIX_OVERRIDE)"
        set_prefix "$TCL_PREFIX_OVERRIDE"
    elif [ -n "$TCL_PREFIX_OVERRIDE" ]; then
        set_prefix "$TCL_PREFIX_OVERRIDE"
        log "updating in place: $PREFIX"
    else
        set_prefix "$F5_PREFIX_OVERRIDE"
        log "updating in place: $PREFIX"
    fi
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
            tcl) TCL_PREFIX_OVERRIDE="$dir" ;;
            f5)  F5_PREFIX_OVERRIDE="$dir" ;;
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
        *)    return ;;
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

    case "$UI" in
        whiptail|dialog)
            sel="$(tui_menu "Resolve naming conflict" \
                "keep"   "Install with original name (may be shadowed)" \
                "rename" "Install as <name>-lsp instead" \
                "abort"  "Cancel install")" \
                || die "aborted: naming conflict not resolved"
            ;;
        *)
            print_prompt "$(printf '\nHow to proceed?\n  1) keep (install with original name; existing shadow remains)\n  2) rename (install as <name>-lsp)\n  3) abort\n  selection [1]: ')"
            read_user_line || reply=1
            case "${reply:-1}" in
                1|keep)   sel=keep ;;
                2|rename) sel=rename ;;
                3|abort)  sel=abort ;;
                *) die "invalid selection: $reply (expected 1, 2, or 3)" ;;
            esac
            ;;
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

    case "$UI" in
        whiptail|dialog)
            set --
            OLD_IFS="$IFS"; IFS='
'
            for c in $cands; do
                set -- "$@" "$c" "$(annotate_candidate "$c")"
            done
            IFS="$OLD_IFS"
            set -- "$@" "Other" "Enter a path manually"

            chosen="$(tui_menu "Choose install location" "$@")" \
                || { log "install location: $PREFIX"; return; }
            if [ "$chosen" = "Other" ]; then
                custom="$(tui_run --inputbox "Path:" 8 70 "$PREFIX" 3>&1 1>&2 2>&3)" \
                    || { log "install location: $PREFIX"; return; }
                [ -n "$custom" ] && set_prefix "$custom"
            else
                set_prefix "$chosen"
            fi
            ;;
        *)
            print_prompt "$(printf '\n%sChoose install location:%s' "$BOLD" "$RESET")"
            i=0
            OLD_IFS="$IFS"; IFS='
'
            for c in $cands; do
                i=$((i + 1))
                annot="$(annotate_candidate "$c")"
                marker=" "
                [ "$c" = "$PREFIX" ] && marker="*"
                print_prompt "$(printf '  %s %d) %-32s %s' "$marker" "$i" "$c" "$annot")"
            done
            IFS="$OLD_IFS"
            other_idx=$((i + 1))
            print_prompt "$(printf '    %d) Other (enter a path)' "$other_idx")"
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
            ;;
    esac
    log "install location: $PREFIX"
}


write_target() {
    # Atomic install of SRC to DST. Prompts before clobbering a non-zipapp
    # or escalating to sudo for a non-writable target dir.
    src="$1"; dst="$2"
    if [ -e "$dst" ] && ! looks_like_our_zipapp "$dst"; then
        warn "$dst already exists and is not a tcl-lsp zipapp"
        warn "(no Python shebang or ZIP signature in first 2KB)"
        if ! ask_default_no "Overwrite $dst anyway? [y/N]"; then
            die "aborted: existing $dst is unrelated and overwrite declined.
Re-run with TCL_LSP_PREFIX=/other/dir, or remove $dst first."
        fi
    fi
    target_dir="$(dirname "$dst")"
    if dir_writable "$target_dir"; then
        mkdir -p "$target_dir"
        install -m 0755 "$src" "$dst"
        return
    fi
    warn "Target directory not writable by $(id -un): $target_dir"
    if ! ask_default_no "Use sudo to install $(basename "$dst") into $target_dir? [y/N]"; then
        die "aborted: $target_dir not writable and sudo declined.
Re-run with TCL_LSP_PREFIX=/path/you/can/write/to, or accept the sudo prompt."
    fi
    run_root mkdir -p "$target_dir"
    run_root install -m 0755 "$src" "$dst"
}


install_cli() {
    # Install CLI $1 (tcl or f5) to its prefix_for() dir.
    name="$1"
    final_name="${name}${INSTALL_SUFFIX}"
    dir="$(prefix_for "$name")"
    ensure_tag
    asset="${name}-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "resolved $name -> $asset (tag $RESOLVED_TAG)"

    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    verify_artefact "$asset" "$tmpfile"
    write_target "$tmpfile" "$dir/$final_name"
    log "installed $name -> $dir/$final_name"
}


PATH_MARKER='# Added by tcl-lsp installer'

ensure_path() {
    if path_contains "$PREFIX"; then return; fi
    if [ "${TCL_LSP_NO_PATH:-0}" = "1" ]; then
        warn "$PREFIX is not on PATH — TCL_LSP_NO_PATH=1, skipping rc update."
        return
    fi
    if [ -f "$RC" ] && grep -qF "$PATH_MARKER" "$RC" 2>/dev/null; then
        log "PATH entry already present in $RC (skipping append)"
        return
    fi
    if ! ask "Add $PREFIX to PATH in $RC? [Y/n]"; then
        warn "Skipped PATH update. Add this to $RC manually:"
        warn "  export PATH=\"$PREFIX:\$PATH\""
        return
    fi
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


install_completion() {
    name="$1"
    final_name="${name}${INSTALL_SUFFIX}"
    if [ "${TCL_LSP_NO_COMP:-0}" = "1" ]; then return; fi
    if [ -n "$INSTALL_SUFFIX" ]; then
        # The bundled completion script registers for $name, not $name-lsp.
        log "skipping $name completion ($final_name has no matching completion script)"
        return
    fi
    if ! ask "Install $name shell completion for $SHELL_NAME? [Y/n]"; then
        if ! tty_available && [ "${TCL_LSP_ASSUME_YES:-0}" != "1" ]; then
            log "skipped $name completion (non-interactive). Install later with:"
            log "  $name completion $SHELL_NAME  # see INSTALL-cli.md for paths"
        fi
        return
    fi

    # Resolve the actual binary path — may be a per-CLI override under
    # split-directory update.
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
    # comp_install BIN OUT_PATH SHELL — write completion script, prompting
    # before overwrite when run interactively.
    cbin="$1"; cout="$2"; cshell="$3"
    if [ -e "$cout" ] && tty_available && [ "${TCL_LSP_ASSUME_YES:-0}" != "1" ]; then
        if ! ask "$cout already exists — overwrite? [Y/n]"; then
            log "kept existing $cout"
            return
        fi
    fi
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
    # Idempotent — choose_install_plan() and install_ai_integrations()
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

ensure_unzip() {
    if have unzip; then return 0; fi
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        warn "unzip missing and TCL_LSP_NO_DEPS=1 — skipping skills install"
        return 1
    fi
    case "$OS" in
        macos)  warn "unzip missing on macOS — install Xcode Command Line Tools"; return 1 ;;
        debian) confirm_root_action "apt-get install -y unzip"; run_root apt-get install -y unzip ;;
        rhel|fedora)
            if have dnf; then
                confirm_root_action "dnf install -y unzip"; run_root dnf install -y unzip
            else
                confirm_root_action "yum install -y unzip"; run_root yum install -y unzip
            fi ;;
        arch)   confirm_root_action "pacman -Sy --noconfirm unzip"; run_root pacman -Sy --noconfirm unzip ;;
        alpine) confirm_root_action "apk add --no-cache unzip";     run_root apk add --no-cache unzip ;;
        *) warn "unzip not found and cannot auto-install"; return 1 ;;
    esac
    have unzip
}

MCP_PATH=""
install_mcp_zipapp() {
    if [ -n "$MCP_PATH" ] && [ -x "$MCP_PATH" ]; then return 0; fi
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
    # Capture prior entry so a failed add can be restored.
    if ! have_claude_cli; then
        warn "claude CLI not on PATH — add the MCP server manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
        return
    fi
    prior=""
    if claude mcp list 2>/dev/null | awk '{print $1}' | grep -qx 'tcl-lsp'; then
        # The exact `mcp list` format isn't a stable contract; capture
        # the full record so we can echo it back as restore guidance.
        prior="$(claude mcp list 2>/dev/null | awk '$1=="tcl-lsp" {sub(/^tcl-lsp[[:space:]]+/,""); print; exit}')"
        claude mcp remove tcl-lsp >/dev/null 2>&1 || true
    fi
    if claude mcp add tcl-lsp -- "$PYTHON" "$MCP_PATH" >/dev/null 2>&1; then
        log "registered MCP server with Claude Code (tcl-lsp)"
        return
    fi
    warn "claude mcp add failed"
    if [ -n "$prior" ]; then
        warn "prior registration was: $prior"
        warn "restore it manually if needed"
    fi
    warn "or register fresh with: claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
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
    py_escaped="$(toml_basic_escape "$PYTHON")"
    mcp_escaped="$(toml_basic_escape "$MCP_PATH")"
    {
        printf '\n[mcp_servers.tcl_lsp]\n'
        printf 'command = "%s"\n' "$py_escaped"
        printf 'args = ["%s"]\n'  "$mcp_escaped"
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

    [ -d "$inner/skills" ]  && cp -R "$inner/skills"  "$HOME/.claude/"
    [ -d "$inner/prompts" ] && cp -R "$inner/prompts" "$HOME/.claude/"
    [ -f "$inner/tcl-ai.pyz" ] && cp "$inner/tcl-ai.pyz" "$HOME/.claude/tcl-ai.pyz"
    chmod 0755 "$HOME/.claude/tcl-ai.pyz" 2>/dev/null || true
    n="$(find "$HOME/.claude/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')"
    log "installed Claude Code skills -> $HOME/.claude/skills/ ($n skills)"
}

install_ai_integrations() {
    # No further questions asked here — choose_install_plan() already
    # captured the user's intent.  We just act on WANT_MCP / WANT_SKILLS.
    if [ "$WANT_MCP" = "1" ]; then
        install_mcp_zipapp
        [ "$HAS_CLAUDE" = "1" ] && register_mcp_claude
        [ "$HAS_CODEX"  = "1" ] && register_mcp_codex
    fi
    if [ "$WANT_SKILLS" = "1" ]; then
        install_claude_skills
    fi
}


main() {
    init_workdir
    ensure_ui
    log "tcl-lsp installer $INSTALLER_VERSION"
    detect_os
    detect_shell
    detect_proxy
    ensure_curl

    if ! find_python; then
        install_python
    fi
    log "using Python: $PYTHON"

    # Front-load every install-time decision into one combined prompt
    # before kicking off downloads, sudo, completion writes, etc.
    choose_install_plan
    propose_update_install
    choose_prefix
    detect_conflicts

    case "$ONLY" in
        tcl)  install_cli tcl ;;
        f5)   install_cli f5 ;;
        both) install_cli tcl; install_cli f5 ;;
        none) : ;;
        *) die "invalid TCL_LSP_ONLY: $ONLY (expected tcl|f5|both)" ;;
    esac

    if [ "$ONLY" != "none" ]; then
        check_cli_runtime_dependencies
        ensure_path
    fi

    case "$ONLY" in
        tcl)  install_completion tcl ;;
        f5)   install_completion f5 ;;
        both) install_completion tcl; install_completion f5 ;;
    esac

    install_ai_integrations

    printf '\n%sInstall complete.%s\n' "$BOLD" "$RESET"
    if [ "$ONLY" != "none" ] && ! path_contains "$PREFIX"; then
        # shellcheck disable=SC2016  # instruction text shown to user
        printf 'Open a new shell, or run:  %sexport PATH="%s:$PATH"%s\n' \
               "$BOLD" "$PREFIX" "$RESET"
    fi
    s="$INSTALL_SUFFIX"
    case "$ONLY" in
        tcl)  printf 'Verify:  %stcl%s --help%s\n' "$BOLD" "$s" "$RESET" ;;
        f5)   printf 'Verify:  %sf5%s --help%s\n'  "$BOLD" "$s" "$RESET" ;;
        both) printf 'Verify:  %stcl%s --help && f5%s --help%s\n' "$BOLD" "$s" "$s" "$RESET" ;;
    esac
    if [ "$WANT_MCP" = "1" ] || [ "$WANT_SKILLS" = "1" ]; then
        printf 'AI integrations: '
        sep=""
        [ "$WANT_MCP"    = "1" ] && { printf '%sMCP server' "$sep"; sep=", "; }
        [ "$WANT_SKILLS" = "1" ] && { printf '%sClaude skills' "$sep"; }
        printf '\n'
    fi
}

main "$@"
