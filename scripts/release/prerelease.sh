#!/usr/bin/env bash
# prerelease.sh — decide whether a version belongs to the pre-release channel.
#
# Usage:  scripts/release/prerelease.sh X.Y.Z   ->  prints "true" or "false"
#
# Convention (the VS Code Marketplace odd/even-minor scheme):
#
#   * From major version 2 onward an ODD minor is a PRE-RELEASE
#     (2.1.x, 2.3.x, …) and an EVEN minor is a STABLE release
#     (2.0.x, 2.2.x, …).  The 2.x line ships its alphas on 2.1.x and
#     promotes to the stable 2.2.0 when it is ready.
#   * The 1.x line predates this convention and is ALWAYS stable — it
#     stays the default "latest" release / Marketplace channel for
#     everyone who has not opted into pre-releases.
#
# This single decision drives every pre-release switch in the release
# pipeline so CI, the Makefile, and tag.sh never disagree:
#
#   * `gh release create --prerelease`  (GitHub Release — keeps it off "latest")
#   * `vsce package/publish --pre-release`  (VS Code Marketplace pre-release channel)
#   * the branch tag.sh expects the tag to be cut from (rust vs main)
#
# Keeping the rule here honours the release-and-publish contract: CI and
# the Makefile *invoke* this script rather than re-deriving the parity.
set -euo pipefail

v="${1:?usage: prerelease.sh X.Y.Z}"
v="${v#v}"

# Split on '.' and strip any non-digit suffix (e.g. a stray "-dev") so a
# malformed or dev version degrades to "false" (stable) rather than
# erroring — the pre-release switches all fail safe toward the default
# stable channel.
IFS='.' read -r major minor _rest <<EOF
$v
EOF
major="${major%%[!0-9]*}"
minor="${minor%%[!0-9]*}"

if [ "${major:-0}" -ge 2 ] && [ "$(( ${minor:-0} % 2 ))" -eq 1 ]; then
    echo true
else
    echo false
fi
