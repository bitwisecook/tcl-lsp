#!/usr/bin/env bash
# Automated screenshot capture for tcl-lsp feature demonstration.
#
# Architecture:
#   - This script runs from Terminal.
#   - It launches VS Code via @vscode/test-electron, resolves the exact
#     VS Code window ID for this run (via a small Swift helper when
#     available), and captures screenshots with screencapture -o -l when
#     the extension signals readiness.
#   - Finally it composites the PNGs into an animated GIF.
#
# Prerequisites:
#   - macOS (uses screencapture and osascript)
#   - Terminal must have Accessibility permission (System Events)
#   - Terminal must have Screen Recording permission (screencapture)
#   - ImageMagick (brew install imagemagick)
#   - Optional: Swift toolchain (for faster/more robust window probing)
#
# Usage:
#   make screenshots             # from repo root
#   bash scripts/screenshots.sh  # directly

# Ensure we run under bash even if invoked via sh.
if [ -z "${BASH_VERSION:-}" ]; then
  exec /usr/bin/env bash "$0" "$@"
fi

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
EXT_DIR="$ROOT_DIR/editors/vscode"
OUTPUT_DIR="$ROOT_DIR/docs/screenshots"
WINDOW_PROBE_SRC="$ROOT_DIR/scripts/macos/vscode_window_probe.swift"
WINDOW_PROBE_BIN="$ROOT_DIR/build/vscode-window-probe"
AUTO_BREW_INSTALL="${TCL_LSP_SCREENSHOT_AUTO_BREW:-0}"

# Prerequisite checks

brew_install_hint() {
  if [[ $# -eq 0 ]]; then
    return 0
  fi
  if command -v brew &>/dev/null; then
    echo "Install with: brew install $*"
  else
    echo "Homebrew not found. Install Homebrew first, then run: brew install $*"
  fi
}

show_optional_tool_hints() {
  local missing=()
  command -v pngquant &>/dev/null || missing+=("pngquant")
  command -v oxipng &>/dev/null || missing+=("oxipng")
  command -v gifsicle &>/dev/null || missing+=("gifsicle")

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "==> Optional screenshot tools missing: ${missing[*]}"
    if [[ "$AUTO_BREW_INSTALL" == "1" ]]; then
      if command -v brew &>/dev/null; then
        echo "==> Auto-install enabled; running: brew install ${missing[*]}"
        if brew install "${missing[@]}"; then
          return 0
        fi
        echo "WARNING: Homebrew install failed; continuing with built-in fallbacks."
      else
        echo "WARNING: Auto-install enabled but Homebrew is not available."
      fi
    fi
    brew_install_hint "${missing[@]}"
    echo "    (continuing with built-in fallbacks where possible)"
  fi
}

check_prereqs() {
  local missing=0

  if ! command -v screencapture &>/dev/null; then
    echo "ERROR: screencapture not found (macOS only)"
    missing=1
  fi

  if ! command -v osascript &>/dev/null; then
    echo "ERROR: osascript not found (macOS only)"
    missing=1
  fi

  if ! command -v magick &>/dev/null && ! command -v convert &>/dev/null; then
    echo "ERROR: ImageMagick not found."
    if [[ "$AUTO_BREW_INSTALL" == "1" ]]; then
      if command -v brew &>/dev/null; then
        echo "==> Auto-install enabled; running: brew install imagemagick"
        if ! brew install imagemagick; then
          echo "WARNING: Homebrew install for ImageMagick failed."
        fi
      else
        echo "WARNING: Auto-install enabled but Homebrew is not available."
      fi
    fi
    if ! command -v magick &>/dev/null && ! command -v convert &>/dev/null; then
      brew_install_hint imagemagick
      missing=1
    fi
  fi

  if [[ $missing -ne 0 ]]; then
    exit 1
  fi
}

check_prereqs
show_optional_tool_hints
MAGICK_BIN=$(command -v magick || command -v convert)

# Setup

mkdir -p "$OUTPUT_DIR"
# Keep existing PNGs so optional/skip-able scenes (for example AI screenshots)
# are not deleted when they are intentionally not captured in this run.
rm -f "$OUTPUT_DIR"/.done "$OUTPUT_DIR"/.shell-ready "$OUTPUT_DIR"/.ai-started "$OUTPUT_DIR"/.ai-success "$OUTPUT_DIR"/.ai-done \
      "$OUTPUT_DIR"/.vscode-main-pid "$OUTPUT_DIR"/.vscode-ext-host-pid \
      "$OUTPUT_DIR"/*.ready "$OUTPUT_DIR"/*.captured
# Scene 06 was removed in favour of a single side-by-side formatting shot.
rm -f "$OUTPUT_DIR"/06-formatting-before.png

export SCREENSHOT_OUTPUT_DIR="$OUTPUT_DIR"
SCREENSHOT_HOME="${TCL_LSP_SCREENSHOT_HOME:-$HOME/.tcl-lsp-screenshots}"
REUSE_CODE_USER_DATA="${TCL_LSP_SCREENSHOT_REUSE_CODE_USER_DATA:-0}"
if [[ -n "${TCL_LSP_SCREENSHOT_USER_DATA_DIR:-}" ]]; then
  SCREENSHOT_USER_DATA_DIR="$TCL_LSP_SCREENSHOT_USER_DATA_DIR"
elif [[ "$REUSE_CODE_USER_DATA" == "1" && "$OSTYPE" == darwin* ]]; then
  SCREENSHOT_USER_DATA_DIR="$HOME/Library/Application Support/Code"
else
  SCREENSHOT_USER_DATA_DIR="$SCREENSHOT_HOME/user-data"
fi
SCREENSHOT_PROFILE="${TCL_LSP_SCREENSHOT_PROFILE:-Tcl LSP Screenshots}"
SCREENSHOT_EXTENSIONS_DIR="${TCL_LSP_SCREENSHOT_EXTENSIONS_DIR:-$SCREENSHOT_HOME/extensions}"
# Use "-" (not ":-") so callers can intentionally pass an empty string to
# disable external extension staging.
SCREENSHOT_ALLOWED_EXTENSIONS="${TCL_LSP_SCREENSHOT_ALLOWED_EXTENSIONS-github.copilot-chat}"
SCREENSHOT_CLEAN_EXTENSIONS_DIR="${TCL_LSP_SCREENSHOT_CLEAN_EXTENSIONS_DIR:-1}"
SCREENSHOT_VSCODE_VERSION="${TCL_LSP_SCREENSHOT_VSCODE_VERSION:-stable}"
SCREENSHOT_FORCE_DOWNLOADED_VSCODE="${TCL_LSP_SCREENSHOT_FORCE_DOWNLOADED_VSCODE:-1}"
mkdir -p "$SCREENSHOT_USER_DATA_DIR"
mkdir -p "$SCREENSHOT_EXTENSIONS_DIR"
export TCL_LSP_SCREENSHOT_HOME="$SCREENSHOT_HOME"
export TCL_LSP_SCREENSHOT_USER_DATA_DIR="$SCREENSHOT_USER_DATA_DIR"
export TCL_LSP_SCREENSHOT_PROFILE="$SCREENSHOT_PROFILE"
export VSCODE_EXTENSIONS_DIR="$SCREENSHOT_EXTENSIONS_DIR"
export TCL_LSP_SCREENSHOT_VSCODE_VERSION="$SCREENSHOT_VSCODE_VERSION"
export TCL_LSP_SCREENSHOT_FORCE_DOWNLOADED_VSCODE="$SCREENSHOT_FORCE_DOWNLOADED_VSCODE"

echo "==> Output dir: $OUTPUT_DIR"
echo "==> VS Code user-data-dir: $SCREENSHOT_USER_DATA_DIR"
echo "==> VS Code profile: $SCREENSHOT_PROFILE"
echo "==> VS Code extensions-dir: $SCREENSHOT_EXTENSIONS_DIR"
echo "==> VS Code download channel: $SCREENSHOT_VSCODE_VERSION"

reset_screenshot_extensions_dir() {
  if [[ "$SCREENSHOT_CLEAN_EXTENSIONS_DIR" != "1" ]]; then
    return 0
  fi
  find "$SCREENSHOT_EXTENSIONS_DIR" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
}

find_latest_user_extension() {
  local extension_id="$1"
  local source_root="$HOME/.vscode/extensions"
  if [[ ! -d "$source_root" ]]; then
    return 0
  fi
  find "$source_root" -maxdepth 1 -type d -name "${extension_id}-*" | sort -V | tail -n 1
}

stage_allowed_extensions() {
  local extension_ids_raw="${SCREENSHOT_ALLOWED_EXTENSIONS}"
  if [[ -z "$extension_ids_raw" ]]; then
    echo "==> No allowed external extensions configured (built-ins only)"
    return 0
  fi

  reset_screenshot_extensions_dir
  local old_ifs="$IFS"
  IFS=','
  read -r -a extension_ids <<< "$extension_ids_raw"
  IFS="$old_ifs"

  echo "==> Staging allowed extensions into isolated dir..."
  local staged=0
  for extension_id in "${extension_ids[@]}"; do
    extension_id="$(echo "$extension_id" | xargs)"
    if [[ -z "$extension_id" ]]; then
      continue
    fi
    local source_dir
    source_dir="$(find_latest_user_extension "$extension_id")"
    if [[ -z "$source_dir" ]]; then
      echo "  WARNING: extension '$extension_id' not found in ~/.vscode/extensions"
      continue
    fi
    local dest_path="$SCREENSHOT_EXTENSIONS_DIR/$(basename "$source_dir")"
    rm -rf "$dest_path"
    cp -R "$source_dir" "$dest_path"
    echo "  staged: $(basename "$source_dir")"
    staged=1
  done

  if [[ $staged -eq 0 ]]; then
    echo "  WARNING: no external extensions staged; AI scenes will be skipped."
  fi
}

stage_allowed_extensions

WINDOW_PROBE_AVAILABLE=0

build_window_probe() {
  if ! command -v swiftc &>/dev/null; then
    return 1
  fi
  if [[ ! -f "$WINDOW_PROBE_SRC" ]]; then
    return 1
  fi
  mkdir -p "$(dirname "$WINDOW_PROBE_BIN")"
  if [[ ! -x "$WINDOW_PROBE_BIN" || "$WINDOW_PROBE_SRC" -nt "$WINDOW_PROBE_BIN" ]]; then
    echo "==> Building Swift window probe..."
    if ! swiftc -O "$WINDOW_PROBE_SRC" -o "$WINDOW_PROBE_BIN"; then
      return 1
    fi
  fi
  return 0
}

if build_window_probe; then
  WINDOW_PROBE_AVAILABLE=1
  echo "==> Swift window probe: $WINDOW_PROBE_BIN"
else
  echo "==> Swift window probe unavailable; using osascript window detection"
fi

# Kill any leftover VS Code test instances from previous runs.
pkill -f "vscode-test.*Electron" 2>/dev/null || true
sleep 0.5

# Launch VS Code in screenshot mode

echo "==> Launching VS Code for screenshot capture..."

node "$EXT_DIR/out/test/runScreenshots.js" &
VSCODE_PID=$!

# Find the VS Code window
# Strategy 1 (preferred): resolve VS Code main PID for this run, then query
# CoreGraphics by PID for an exact CGWindowID.
#   Needs Screen Recording permission for Terminal.  Enables
#   `screencapture -o -l <id>` which captures only the window (no desktop
#   bleed, no shadow).
#
# Strategy 2 (fallback):  position+size via System Events (Accessibility).
#   Works without Screen Recording permission but uses
#   `screencapture -R x,y,w,h` which is a flat rectangle crop.

echo "==> Waiting for VS Code window..."

find_vscode_pid_by_userdata() {
  local user_data_arg="--user-data-dir=$SCREENSHOT_USER_DATA_DIR"
  ps -ax -o pid= -o command= | awk -v arg="$user_data_arg" '
    index($0, arg) > 0 &&
    $0 ~ /Visual Studio Code( - Insiders)?\.app\/Contents\/MacOS\/Electron/ &&
    $0 !~ /--type=/ {
      print $1
    }
  ' | tail -n 1
}

read_vscode_pid_marker() {
  local marker="$OUTPUT_DIR/.vscode-main-pid"
  local pid=""
  if [[ -f "$marker" ]]; then
    pid=$(tr -cd '0-9' < "$marker")
  fi
  if [[ -n "$pid" ]]; then
    echo "$pid"
  fi
}

get_window_id_for_pid() {
  local pid="$1"
  if [[ "$WINDOW_PROBE_AVAILABLE" -eq 1 ]]; then
    "$WINDOW_PROBE_BIN" id --pid "$pid" 2>/dev/null || true
    return 0
  fi
  TCL_LSP_SCREENSHOT_TARGET_PID="$pid" osascript -l JavaScript <<'ENDJXA' 2>/dev/null || true
ObjC.import("CoreGraphics");
ObjC.import("stdlib");

var pidText = ObjC.unwrap($.getenv("TCL_LSP_SCREENSHOT_TARGET_PID")) || "";
var targetPid = parseInt(pidText, 10);
if (!targetPid || targetPid <= 0) {
  "";
} else {
  var wins = ObjC.deepUnwrap(
    $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, $.kCGNullWindowID)
  );
  var best = null;
  var bestArea = -1;
  for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    if (w.kCGWindowOwnerPID !== targetPid || w.kCGWindowLayer !== 0) {
      continue;
    }
    var b = w.kCGWindowBounds || {};
    var width = Number(b.Width || 0);
    var height = Number(b.Height || 0);
    if (width < 600 || height < 400) {
      continue;
    }
    var area = width * height;
    if (area > bestArea) {
      bestArea = area;
      best = w;
    }
  }
  best ? String(best.kCGWindowNumber) : "";
}
ENDJXA
}

get_window_ids_for_pid() {
  local pid="$1"
  TCL_LSP_SCREENSHOT_TARGET_PID="$pid" osascript -l JavaScript <<'ENDJXA' 2>/dev/null || true
ObjC.import("CoreGraphics");
ObjC.import("stdlib");

var pidText = ObjC.unwrap($.getenv("TCL_LSP_SCREENSHOT_TARGET_PID")) || "";
var targetPid = parseInt(pidText, 10);
if (!targetPid || targetPid <= 0) {
  "";
} else {
  var wins = ObjC.deepUnwrap(
    $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, $.kCGNullWindowID)
  );
  var matches = [];
  for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    if (w.kCGWindowOwnerPID !== targetPid || w.kCGWindowLayer !== 0) {
      continue;
    }
    var b = w.kCGWindowBounds || {};
    var width = Number(b.Width || 0);
    var height = Number(b.Height || 0);
    if (width < 300 || height < 200) {
      continue;
    }
    matches.push({
      id: String(w.kCGWindowNumber),
      area: width * height,
    });
  }
  matches.sort(function (a, b) { return b.area - a.area; });
  matches.map(function (m) { return m.id; }).join("\n");
}
ENDJXA
}

VSCODE_MAIN_PID=""
WINDOW_ID=""
LAST_MARKER_PID=""
for _attempt in $(seq 1 120); do
  WINDOW_ID=""

  MARKER_PID=$(read_vscode_pid_marker)
  if [[ -n "$MARKER_PID" ]]; then
    if [[ "$MARKER_PID" != "$LAST_MARKER_PID" ]]; then
      echo "==> Found VS Code main pid marker from extension: $MARKER_PID"
      LAST_MARKER_PID="$MARKER_PID"
    fi
    WINDOW_ID=$(get_window_id_for_pid "$MARKER_PID")
    if [[ -n "$WINDOW_ID" && "$WINDOW_ID" != "0" ]]; then
      VSCODE_MAIN_PID="$MARKER_PID"
      break
    fi
  fi

  USERDATA_PID=$(find_vscode_pid_by_userdata)
  if [[ -n "$USERDATA_PID" ]]; then
    WINDOW_ID=$(get_window_id_for_pid "$USERDATA_PID")
    if [[ -n "$WINDOW_ID" && "$WINDOW_ID" != "0" ]]; then
      VSCODE_MAIN_PID="$USERDATA_PID"
      break
    fi
  fi
  if (( _attempt % 10 == 0 )); then
    echo "  ... still waiting for VS Code window id (attempt $_attempt/120)"
  fi
  sleep 0.5
done

if [[ -z "$WINDOW_ID" || "$WINDOW_ID" == "0" ]]; then
  WINDOW_ID=""
fi

# Strategy 2 (fallback): System Events position+size

get_window_rect_for_pid() {
  local pid="$1"
  if [[ "$WINDOW_PROBE_AVAILABLE" -eq 1 ]]; then
    "$WINDOW_PROBE_BIN" rect --pid "$pid" 2>/dev/null || true
    return 0
  fi
  osascript - "$pid" <<'ENDOSA' 2>/dev/null || true
on run argv
  set targetPid to (item 1 of argv) as integer
  tell application "System Events"
    repeat with proc in (every process whose background only is false)
      if unix id of proc is targetPid then
        if (count of windows of proc) is 0 then
          return ""
        end if
        set w to front window of proc
        set {x, y} to position of w
        set {width, height} to size of w
        return (x as text) & "," & (y as text) & "," & (width as text) & "," & (height as text)
      end if
    end repeat
  end tell
  return ""
end run
ENDOSA
}

get_window_rect_generic() {
  osascript -e '
    tell application "System Events"
      repeat with proc in (every process whose background only is false)
        set pName to name of proc
        if pName contains "Code" then
          if (count of windows of proc) > 0 then
            set w to front window of proc
            set {x, y} to position of w
            set {width, height} to size of w
            return (x as text) & "," & (y as text) & "," & (width as text) & "," & (height as text)
          end if
        end if
      end repeat
    end tell
  ' 2>/dev/null || true
}

get_window_rect() {
  local rect=""
  if [[ -n "$VSCODE_MAIN_PID" ]]; then
    rect=$(get_window_rect_for_pid "$VSCODE_MAIN_PID")
  fi
  if [[ -n "$rect" ]]; then
    echo "$rect"
    return 0
  fi
  get_window_rect_generic
}

position_window_for_pid() {
  local pid="$1"
  if [[ "$WINDOW_PROBE_AVAILABLE" -eq 1 ]]; then
    "$WINDOW_PROBE_BIN" move --pid "$pid" --x 50 --y 50 --w 1200 --h 900 >/dev/null 2>&1 || true
    return 0
  fi
  osascript - "$pid" <<'ENDOSA' 2>/dev/null || true
on run argv
  set targetPid to (item 1 of argv) as integer
  tell application "System Events"
    repeat with proc in (every process whose background only is false)
      if unix id of proc is targetPid then
        if (count of windows of proc) > 0 then
          set w to front window of proc
          set position of w to {50, 50}
          set size of w to {1200, 900}
          return
        end if
      end if
    end repeat
  end tell
end run
ENDOSA
}

position_window_generic() {
  osascript -e '
    tell application "System Events"
      repeat with proc in (every process whose background only is false)
        set pName to name of proc
        if pName contains "Code" then
          if (count of windows of proc) > 0 then
            set w to front window of proc
            set position of w to {50, 50}
            set size of w to {1200, 900}
            return
          end if
        end if
      end repeat
    end tell
  ' 2>/dev/null || true
}

WINDOW_RECT=""
if [[ -n "$WINDOW_ID" ]]; then
  CAPTURE_MODE="window"
  echo "==> Found VS Code PID: $VSCODE_MAIN_PID"
  echo "==> Found CGWindowID: $WINDOW_ID (screencapture -o -l - no background, no shadow)"
else
  echo "  CGWindowID not available (Screen Recording permission not granted to Terminal)."
  echo "  For best results: System Settings > Privacy & Security > Screen Recording > Terminal"
  echo "  Falling back to region capture..."

  for _attempt in $(seq 1 120); do
    WINDOW_RECT=$(get_window_rect)
    if [[ -n "$WINDOW_RECT" ]]; then
      break
    fi
    if (( _attempt % 10 == 0 )); then
      echo "  ... still waiting for VS Code window (attempt $_attempt/120)"
    fi
    sleep 0.5
  done

  if [[ -z "$WINDOW_RECT" ]]; then
    echo "ERROR: Could not find VS Code window"
    kill "$VSCODE_PID" 2>/dev/null || true
    exit 1
  fi

  CAPTURE_MODE="region"
  echo "==> Found window rect: $WINDOW_RECT (screencapture -R - region crop)"

  # Move the window away from screen edges and resize to 1200x900 so the
  # padded capture region fits comfortably and the bottom panel has room.
  echo "  Repositioning window to (50, 50) and resizing to 1200x900..."
  if [[ -n "$VSCODE_MAIN_PID" ]]; then
    position_window_for_pid "$VSCODE_MAIN_PID"
  else
    position_window_generic
  fi
  sleep 0.3  # let the window settle after repositioning

  # Re-read the rect after moving/resizing.
  WINDOW_RECT=$(get_window_rect)
  if [[ -z "$WINDOW_RECT" ]]; then
    echo "  (re-query returned empty, using 50,50,1200,900)"
    WINDOW_RECT="50,50,1200,900"
  fi
  echo "==> Window rect after move: $WINDOW_RECT"
fi

# Capture helper

capture_region_with_retry() {
  local initial_rect="$1"
  local output_path="$2"
  local current_rect="$initial_rect"
  local fallback_rects=(
    "50,50,1200,900"
    "100,100,1000,700"
    "20,40,800,600"
  )

  for _region_attempt in $(seq 1 4); do
    if [[ $_region_attempt -gt 1 ]]; then
      current_rect="${fallback_rects[$((_region_attempt - 2))]}"
    elif [[ -z "$current_rect" ]]; then
      current_rect="$(get_window_rect)"
      if [[ -z "$current_rect" ]]; then
        current_rect="$WINDOW_RECT"
      fi
      if [[ -z "$current_rect" ]]; then
        current_rect="${fallback_rects[0]}"
      fi
    fi

    IFS=',' read -r x y w h <<< "$current_rect"
    echo "    screencapture -R ${x},${y},${w},${h} (attempt ${_region_attempt}/4)"
    if screencapture -R "${x},${y},${w},${h}" -x -o "$output_path"; then
      return 0
    fi

    sleep 0.14
  done

  return 1
}

capture_fullscreen_cropped() {
  local output_path="$1"
  local rect_hint="$2"
  local tmp_png
  tmp_png="$(mktemp "${TMPDIR:-/tmp}/tcl-lsp-full-XXXXXX.png")"

  if ! screencapture -x "$tmp_png"; then
    rm -f "$tmp_png"
    return 1
  fi

  local rect="$rect_hint"
  if [[ -z "$rect" ]]; then
    rect="$(get_window_rect)"
  fi
  if [[ -z "$rect" ]]; then
    rect="$WINDOW_RECT"
  fi
  if [[ -z "$rect" ]]; then
    rect="50,50,1200,900"
  fi

  IFS=',' read -r x y w h <<< "$rect"
  local full_w full_h
  full_w="$(sips -g pixelWidth "$tmp_png" 2>/dev/null | awk '/pixelWidth:/{print $2}' | tail -n 1)"
  full_h="$(sips -g pixelHeight "$tmp_png" 2>/dev/null | awk '/pixelHeight:/{print $2}' | tail -n 1)"
  if [[ -z "$full_w" || -z "$full_h" ]]; then
    rm -f "$tmp_png"
    return 1
  fi

  local desktop_bounds
  desktop_bounds="$(osascript -e 'tell application "Finder" to get bounds of window of desktop' 2>/dev/null | tr -d ' ')"
  local desk_w="$full_w"
  local desk_h="$full_h"
  if [[ "$desktop_bounds" == *,*,*,* ]]; then
    IFS=',' read -r _ _ desk_right desk_bottom <<< "$desktop_bounds"
    if [[ -n "$desk_right" && -n "$desk_bottom" && "$desk_right" -gt 0 && "$desk_bottom" -gt 0 ]]; then
      desk_w="$desk_right"
      desk_h="$desk_bottom"
    fi
  fi

  local crop_x crop_y crop_w crop_h
  crop_x="$(awk -v x="$x" -v fw="$full_w" -v dw="$desk_w" 'BEGIN{printf "%d", (x*fw/dw)+0.5}')"
  crop_y="$(awk -v y="$y" -v fh="$full_h" -v dh="$desk_h" 'BEGIN{printf "%d", (y*fh/dh)+0.5}')"
  crop_w="$(awk -v w="$w" -v fw="$full_w" -v dw="$desk_w" 'BEGIN{printf "%d", (w*fw/dw)+0.5}')"
  crop_h="$(awk -v h="$h" -v fh="$full_h" -v dh="$desk_h" 'BEGIN{printf "%d", (h*fh/dh)+0.5}')"

  if [[ "$crop_x" -lt 0 ]]; then crop_x=0; fi
  if [[ "$crop_y" -lt 0 ]]; then crop_y=0; fi
  if [[ "$crop_w" -lt 10 ]]; then crop_w=10; fi
  if [[ "$crop_h" -lt 10 ]]; then crop_h=10; fi
  if [[ $((crop_x + crop_w)) -gt "$full_w" ]]; then crop_w=$((full_w - crop_x)); fi
  if [[ $((crop_y + crop_h)) -gt "$full_h" ]]; then crop_h=$((full_h - crop_y)); fi
  if [[ "$crop_w" -lt 10 || "$crop_h" -lt 10 ]]; then
    rm -f "$tmp_png"
    return 1
  fi

  if "$MAGICK_BIN" "$tmp_png" -crop "${crop_w}x${crop_h}+${crop_x}+${crop_y}" +repage "$output_path" >/dev/null 2>&1; then
    rm -f "$tmp_png"
    return 0
  fi

  rm -f "$tmp_png"
  return 1
}

capture_window() {
  local output_path="$1"

  if [[ "$CAPTURE_MODE" == "window" ]]; then
    # Resolve a fresh CGWindowID each frame (window IDs can rotate when the
    # app recreates windows) and retry before falling back to region capture.
    for _attempt in $(seq 1 4); do
      local candidate_id=""
      if [[ -n "$VSCODE_MAIN_PID" ]]; then
        candidate_id="$(get_window_id_for_pid "$VSCODE_MAIN_PID")"
      fi
      if [[ -z "$candidate_id" || "$candidate_id" == "0" ]]; then
        local fallback_pid
        fallback_pid="$(find_vscode_pid_by_userdata)"
        if [[ -n "$fallback_pid" ]]; then
          candidate_id="$(get_window_id_for_pid "$fallback_pid")"
          if [[ -n "$candidate_id" && "$candidate_id" != "0" ]]; then
            VSCODE_MAIN_PID="$fallback_pid"
          fi
        fi
      fi
      if [[ -n "$candidate_id" && "$candidate_id" != "0" ]]; then
        WINDOW_ID="$candidate_id"
      fi
      if [[ -n "$WINDOW_ID" && "$WINDOW_ID" != "0" ]]; then
        if screencapture -l "$WINDOW_ID" -x -o "$output_path"; then
          return 0
        fi
      fi
      if [[ -n "$VSCODE_MAIN_PID" ]]; then
        while IFS= read -r alt_id; do
          if [[ -z "$alt_id" || "$alt_id" == "$WINDOW_ID" || "$alt_id" == "0" ]]; then
            continue
          fi
          if screencapture -l "$alt_id" -x -o "$output_path"; then
            WINDOW_ID="$alt_id"
            return 0
          fi
        done < <(get_window_ids_for_pid "$VSCODE_MAIN_PID")
      fi
      sleep 0.12
    done

    echo "    WARNING: window capture failed; falling back to region capture for this frame"
    local current_rect
    current_rect="$(get_window_rect)"
    if [[ -z "$current_rect" ]]; then
      current_rect="$WINDOW_RECT"
    fi
    if ! capture_region_with_retry "$current_rect" "$output_path"; then
      echo "    WARNING: region fallback capture failed; trying full-screen crop"
      if ! capture_fullscreen_cropped "$output_path" "$current_rect"; then
        echo "    ERROR: full-screen crop fallback failed"
        return 1
      fi
    fi
  else
    # Re-query window rect each time in case the window moved/resized
    # (e.g. when a chat panel opens alongside the editor).
    local current_rect
    current_rect=$(get_window_rect)
    if [[ -z "$current_rect" ]]; then
      current_rect="$WINDOW_RECT"  # fallback to stored rect
    fi
    if ! capture_region_with_retry "$current_rect" "$output_path"; then
      echo "    WARNING: region capture failed; trying full-screen crop"
      if ! capture_fullscreen_cropped "$output_path" "$current_rect"; then
        echo "    ERROR: full-screen crop fallback failed"
        return 1
      fi
    fi
  fi
}

# Screenshot capture loop - watch for .ready files from the extension

# Signal the extension that we're ready to start capturing.
touch "$OUTPUT_DIR/.shell-ready"

echo "==> Watching for screenshot signals..."

TIMEOUT=$((SECONDS + 360))  # 6-minute overall timeout (AI scenes need extra time)

while [[ $SECONDS -lt $TIMEOUT ]]; do
  # Check if the extension is done.
  if [[ -f "$OUTPUT_DIR/.done" ]]; then
    echo "==> All screenshots captured."
    rm -f "$OUTPUT_DIR/.done"
    break
  fi

  # Check for .ready signals from the extension.
  for ready_file in "$OUTPUT_DIR"/*.ready; do
    [[ -f "$ready_file" ]] || continue

    scene_name="$(basename "$ready_file" .ready)"
    output_path="$OUTPUT_DIR/${scene_name}.png"

    echo "  Capturing: $scene_name"

    capture_window "$output_path"

    # Signal back to the extension.
    rm -f "$ready_file"
    touch "$OUTPUT_DIR/${scene_name}.captured"
  done

  sleep 0.05  # 50ms poll
done

# Wait for VS Code to exit.
wait "$VSCODE_PID" 2>/dev/null || true

# Optimise & composite

MAGICK=$(command -v magick || command -v convert)
PNG_FILES=()
while IFS= read -r f; do
  PNG_FILES+=("$f")
done < <(ls "$OUTPUT_DIR"/*.png 2>/dev/null | sort)

if [[ ${#PNG_FILES[@]} -eq 0 ]]; then
  echo "WARNING: No screenshots captured."
  exit 1
fi

# Add a uniform dark border (region captures include raw desktop behind
# the window - replacing that with a solid border looks much cleaner).

if [[ "$CAPTURE_MODE" == "region" ]]; then
  echo "==> Adding clean border to region captures..."
  for f in "${PNG_FILES[@]}"; do
    "$MAGICK" "$f" -bordercolor '#1e1e2e' -border 20 "$f"
  done
fi

# Optimise PNGs
#   1. pngquant  - lossy palettisation to 256 colours  (brew install pngquant)
#   2. oxipng    - lossless recompression               (brew install oxipng)
#   Fallback: ImageMagick PNG8 if neither is installed.

echo "==> Optimising ${#PNG_FILES[@]} PNGs..."

if command -v pngquant &>/dev/null; then
  echo "    pngquant: palettising to 256 colours (no dithering)..."
  for f in "${PNG_FILES[@]}"; do
    pngquant --force --quality=70-95 --speed 1 --nofs --strip --output "$f" -- "$f"
  done
else
  echo "    pngquant not found - using ImageMagick PNG8 fallback"
  echo "    $(brew_install_hint pngquant)"
  for f in "${PNG_FILES[@]}"; do
    "$MAGICK" "$f" -dither None -colors 256 -depth 8 -strip "PNG8:$f"
  done
fi

if command -v oxipng &>/dev/null; then
  echo "    oxipng: lossless recompression..."
  oxipng -o 4 --strip safe "${PNG_FILES[@]}"
else
  echo "    oxipng not found - skipping lossless recompression"
  echo "    $(brew_install_hint oxipng)"
fi

# Show before/after sizes.
echo "    PNG sizes after optimisation:"
ls -lh "$OUTPUT_DIR"/*.png | awk '{print "      " $5 "  " $NF}'

# Composite into animated GIF

echo "==> Compositing ${#PNG_FILES[@]} screenshots into animated GIF"

# Determine common dimensions from the first image, then normalise all frames.
REF_DIMS=$("$MAGICK" identify -format '%wx%h' "${PNG_FILES[0]}")
echo "    Reference dimensions: $REF_DIMS"
for f in "${PNG_FILES[@]}"; do
  DIMS=$("$MAGICK" identify -format '%wx%h' "$f")
  if [[ "$DIMS" != "$REF_DIMS" ]]; then
    echo "    Resizing $(basename "$f"): $DIMS -> $REF_DIMS"
    "$MAGICK" "$f" -resize "$REF_DIMS!" -extent "$REF_DIMS" -background '#1e1e2e' -gravity center "$f"
  fi
done

# Full-size GIF with frame-difference optimisation.
"$MAGICK" -delay 200 -loop 0 \
  "${PNG_FILES[@]}" \
  -dither None -colors 256 \
  -layers OptimizePlus \
  "$OUTPUT_DIR/tcl-lsp-demo.gif"
echo "==> GIF: $OUTPUT_DIR/tcl-lsp-demo.gif"

# Half-size version for README embedding.
"$MAGICK" -delay 200 -loop 0 \
  "${PNG_FILES[@]}" \
  -resize 50% \
  -dither None -colors 256 \
  -layers OptimizePlus \
  "$OUTPUT_DIR/tcl-lsp-demo-small.gif"
echo "==> Small GIF: $OUTPUT_DIR/tcl-lsp-demo-small.gif"

# Optional gifsicle pass for further GIF compression.

if command -v gifsicle &>/dev/null; then
  echo "==> Running gifsicle optimisation..."
  for gif in "$OUTPUT_DIR"/tcl-lsp-demo*.gif; do
    gifsicle -O3 --colors 256 -b "$gif"
  done
else
  echo "==> gifsicle not found - skipping optional GIF optimisation"
  echo "   $(brew_install_hint gifsicle)"
fi

echo ""
echo "==> Final output:"
ls -lh "$OUTPUT_DIR"/*.png "$OUTPUT_DIR"/*.gif
