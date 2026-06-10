#!/usr/bin/env python3
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
