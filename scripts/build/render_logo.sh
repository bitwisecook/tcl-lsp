#!/usr/bin/env bash
# render_logo.sh — rasterise the canonical Tcl-LSP logo SVGs to the PNG
# sizes the project ships.
#
# The SVGs in docs/ are the source of truth:
#   docs/tcl-lsp-logo.svg        (light squircle)
#   docs/tcl-lsp-logo-dark.svg   (dark squircle, for dark UI themes)
#
# This regenerates the committed 8-bit PNGs that everything else consumes
# (the README, the VS Code extension icon copied from the 128px light PNG
# at package time, etc.).  Run it after editing either SVG:
#
#   make logo
#
# Pipeline per size:  rsvg-convert (vector → RGBA PNG)
#                   → pngquant     (RGBA → 8-bit palette, hence "8bit")
#                   → zopflipng    (lossless recompression)
#
# rsvg-convert (librsvg) is required; inkscape is used as a fallback.
# pngquant / zopflipng are optional — without them you still get correct
# (larger, 32-bit) PNGs and a warning.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SIZES=(64 128 256 512 1024)

# Each entry: "<source svg>|<output basename>" — {size} is substituted and
# ".png" appended.  The light names match the historical committed PNGs so
# every existing reference (README, the VS Code icon copy in the Makefile)
# keeps working untouched.
TARGETS=(
    "docs/tcl-lsp-logo.svg|docs/Tcl LSP Logo-8bit-{size}"
    "docs/tcl-lsp-logo-dark.svg|docs/Tcl LSP Logo-dark-8bit-{size}"
)

# --- tool detection ---------------------------------------------------------
RASTERISER=""
if command -v rsvg-convert >/dev/null 2>&1; then
    RASTERISER="rsvg"
elif command -v inkscape >/dev/null 2>&1; then
    RASTERISER="inkscape"
else
    echo "ERROR: need 'rsvg-convert' (librsvg) or 'inkscape' to rasterise the logo." >&2
    echo "       Debian/Ubuntu:  apt install librsvg2-bin" >&2
    exit 1
fi

HAVE_PNGQUANT=0; command -v pngquant >/dev/null 2>&1 && HAVE_PNGQUANT=1
HAVE_ZOPFLI=0;   command -v zopflipng >/dev/null 2>&1 && HAVE_ZOPFLI=1
[ "$HAVE_PNGQUANT" -eq 1 ] || echo "warning: pngquant not found — PNGs will be 32-bit RGBA (larger)." >&2
[ "$HAVE_ZOPFLI" -eq 1 ]   || echo "warning: zopflipng not found — PNGs will not be recompressed." >&2

rasterise() {  # rasterise <svg> <width> <out.png>
    local svg="$1" w="$2" out="$3"
    if [ "$RASTERISER" = "rsvg" ]; then
        rsvg-convert -w "$w" "$svg" -o "$out"
    else
        inkscape "$svg" --export-type=png --export-width="$w" --export-filename="$out" >/dev/null 2>&1
    fi
}

for entry in "${TARGETS[@]}"; do
    svg="${entry%%|*}"
    pattern="${entry#*|}"
    if [ ! -f "$svg" ]; then
        echo "ERROR: missing source SVG: $svg" >&2
        exit 1
    fi
    for size in "${SIZES[@]}"; do
        out="${pattern/\{size\}/$size}.png"
        rasterise "$svg" "$size" "$out"
        if [ "$HAVE_PNGQUANT" -eq 1 ]; then
            # --force overwrite in place; tolerate exit 98/99 ("could not
            # reduce" / "skipped, larger") by keeping the RGBA original.
            pngquant --force --strip --output "$out" 256 -- "$out" || true
        fi
        if [ "$HAVE_ZOPFLI" -eq 1 ]; then
            zopflipng -y "$out" "$out" >/dev/null 2>&1 || true
        fi
        # Report dimensions + size without depending on ImageMagick.
        info="$(file -b "$out" 2>/dev/null | grep -oE '[0-9]+ x [0-9]+' | head -1 || true)"
        printf '  %-44s %s (%s bytes)\n' "$out" "${info:-png}" "$(wc -c <"$out" | tr -d ' ')"
    done
done

echo "logo: rendered ${#SIZES[@]} sizes × ${#TARGETS[@]} variants from docs/*.svg"
