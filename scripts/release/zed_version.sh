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

# Keep the committed Zed manifest aligned with a release tag. Zed's central
# registry builds this directory from the tagged commit, so unlike artefacts
# assembled by Make, the manifest version cannot be stamped after the tag.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$ROOT/editors/zed/extension.toml"

usage() {
    echo "Usage: $0 set|check X.Y.Z" >&2
    exit 2
}

MODE="${1:-}"
VERSION="${2:-}"
[ "$MODE" = set ] || [ "$MODE" = check ] || usage
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage

current="$({ sed -n 's/^version = "\([^"]*\)"/\1/p' "$MANIFEST"; } | head -n 1)"
[ -n "$current" ] || {
    echo "error: $MANIFEST has no top-level version" >&2
    exit 1
}

if [ "$MODE" = check ]; then
    if [ "$current" != "$VERSION" ]; then
        echo "error: editors/zed/extension.toml is $current, but the release is v$VERSION." >&2
        echo "       Run 'make release-zed-version V=$VERSION', commit the change, and retry." >&2
        exit 1
    fi
    echo "    Zed:      extension.toml is $VERSION"
    exit 0
fi

python3 - "$MANIFEST" "$VERSION" <<'PY'
from pathlib import Path
import re
import sys

path = Path(sys.argv[1])
version = sys.argv[2]
text = path.read_text(encoding="utf-8")
updated, count = re.subn(
    r'(?m)^version = "[^"]*"$', f'version = "{version}"', text, count=1
)
if count != 1:
    raise SystemExit(f"error: expected one top-level version in {path}")
path.write_text(updated, encoding="utf-8")
PY

echo "Set editors/zed/extension.toml to $VERSION"
