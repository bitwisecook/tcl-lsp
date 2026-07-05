#!/usr/bin/env python3
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

"""Print the project version that hatch-vcs would produce for the current tree.

Used by the Makefile so ``WHEEL_FILENAME`` matches the wheel that
``uv build`` will actually emit (e.g. ``1.10.5`` on a tagged commit,
``1.10.5.dev9`` nine commits past the last tag).

Falls back to ``0.0.0+unknown`` if hatch-vcs is unavailable or the tree
has no tags — so the Makefile parse never fails on a fresh clone.
"""

from __future__ import annotations

import sys


def main() -> int:
    try:
        from hatch_vcs.version_source import VCSVersionSource
    except ImportError:
        # hatch-vcs is a build-time dep; if it's missing in the host
        # environment we cannot compute the version — emit a fallback
        # that's still PEP 440-valid so wheel filenames stay legal.
        print("0.0.0+unknown")
        return 0

    config = {
        "source": "vcs",
        "raw-options": {"local_scheme": "no-local-version"},
    }
    try:
        version = VCSVersionSource(".", config).get_version_data()["version"]
    except LookupError:
        # No tag in history yet (or shallow clone with depth 1 and no
        # tags); fall back to a stable dev string.
        print("0.0.0.dev0")
        return 0

    print(version)
    return 0


if __name__ == "__main__":
    sys.exit(main())
