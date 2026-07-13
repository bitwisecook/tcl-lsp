#!/usr/bin/env bash
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

# render_logo.sh — rasterise the canonical project logo SVGs to the PNG
# sizes the project ships.
#
# The SVGs in docs/ are the source of truth:
#   docs/tcl-lsp-logo.svg        (light squircle)
#   docs/tcl-lsp-logo-dark.svg   (dark squircle, for dark UI themes)
#   docs/f5q-logo.svg            (f5-query — dark squircle, one variant)
#
# Every id in those SVGs is namespaced (`tcl-`, `tcld-`, `f5q-`) because the
# BIG-IP report inlines them side by side as <svg> elements in one document,
# where ids share a single namespace — unprefixed ids (the minifier emits `a`,
# `b`, `c`, …) would collide and make one logo render with another's gradients.
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
    "docs/f5q-logo.svg|docs/f5q-logo-8bit-{size}"
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

optimise() {  # optimise <png> — squeeze a rendered PNG as small as is reasonable.
    local png="$1"
    if [ "$HAVE_PNGQUANT" -eq 1 ]; then
        # RGBA -> 8-bit palette (the big win).  --force overwrite in place;
        # --strip drops metadata; tolerate exit 98/99 ("could not reduce" /
        # "skipped, larger") by keeping the RGBA original.
        pngquant --force --strip --output "$png" 256 -- "$png" || true
    fi
    if [ "$HAVE_ZOPFLI" -eq 1 ]; then
        # -m: more iterations; --lossy_transparent: drop colour behind fully
        # transparent pixels (no visual change); --filters=0me: try a few
        # filter strategies and keep the smallest.  Slow but these are small
        # one-off assets.
        zopflipng -y -m --lossy_transparent --filters=0me "$png" "$png" >/dev/null 2>&1 \
            || zopflipng -y "$png" "$png" >/dev/null 2>&1 || true
    fi
}

report() {  # report <png> — dimensions + bytes, without depending on ImageMagick.
    local png="$1" info
    info="$(file -b "$png" 2>/dev/null | grep -oE '[0-9]+ x [0-9]+' | head -1 || true)"
    printf '  %-44s %s (%s bytes)\n' "$png" "${info:-png}" "$(wc -c <"$png" | tr -d ' ')"
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
        optimise "$out"
        report "$out"
    done
done

# --- downstream integration ------------------------------------------------
# Keep docs/*.svg the single source of truth, then propagate copies + raster
# favicons to the places that consume a logo directly:
#   * the Sphinx docs theme (furo light/dark logo + favicon)
#   * the compiler-explorer web GUI (browser-tab favicon)
#   * the BIG-IP report generator, which inlines them into every report
SPHINX_STATIC="docs/sphinx/_static"
# The explorer GUI moved out of the retired tooling/explorer/static tree into
# the tcl-cli crate, where build.rs embeds it into the `tcl` binary.
EXPLORER_STATIC="rust/tcl-cli/gui"
# The report generator's vendored assets: render.rs `include_str!`s these, and
# frontend/build.mjs syncs them into the Python `f5report` package, so both
# backends inline the same marks into the report they emit.
REPORT_ASSETS="rust/bigip-report-gen/assets"

if [ -d "docs/sphinx" ]; then
    mkdir -p "$SPHINX_STATIC"
    cp docs/tcl-lsp-logo.svg docs/tcl-lsp-logo-dark.svg "$SPHINX_STATIC/"
    rasterise docs/tcl-lsp-logo.svg 64 "$SPHINX_STATIC/favicon.png"
    optimise "$SPHINX_STATIC/favicon.png"
    report "$SPHINX_STATIC/favicon.png"
fi

if [ -d "$EXPLORER_STATIC" ]; then
    cp docs/tcl-lsp-logo.svg "$EXPLORER_STATIC/tcl-lsp-logo.svg"
    rasterise docs/tcl-lsp-logo.svg 48 "$EXPLORER_STATIC/favicon.png"
    optimise "$EXPLORER_STATIC/favicon.png"
    report "$EXPLORER_STATIC/favicon.png"
fi

if [ -d "$REPORT_ASSETS" ]; then
    cp docs/f5q-logo.svg          "$REPORT_ASSETS/logo-f5q.svg"
    cp docs/tcl-lsp-logo.svg      "$REPORT_ASSETS/logo-tcl-lsp.svg"
    cp docs/tcl-lsp-logo-dark.svg "$REPORT_ASSETS/logo-tcl-lsp-dark.svg"
    for f in logo-f5q logo-tcl-lsp logo-tcl-lsp-dark; do
        svg="$REPORT_ASSETS/$f.svg"
        printf '  %-44s svg (%s bytes)\n' "$svg" "$(wc -c <"$svg" | tr -d ' ')"
    done
fi

echo "logo: rendered ${#SIZES[@]} sizes × ${#TARGETS[@]} variants from docs/*.svg (+ sphinx/explorer/report assets)"
