#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# inspect_sdk.sh — disassemble an IntelliJ Platform API class across IDE
# versions WITHOUT downloading a multi-GB IDE.
#
# The JetBrains "intellij-repository" publishes each platform module as its
# own small jar (the `lsp` module is ~200-450 KB), so we can fetch just the
# module that declares the API we care about and run `javap` over the exact
# class. This is how you prove *where* a method is declared, whether it moved
# to a super-interface, and whether its signature changed between the version
# we compile against (the sinceBuild floor) and the version that fails
# verification.
#
# Usage:
#   inspect_sdk.sh <version> <fully.qualified.ClassName> [module] [-c]
#
#   <version>      IDE platform version, e.g. 241.14494.240, 2024.1,
#                  262.8665.176. Release builds resolve from the `releases`
#                  repo; EAP/RC builds (…-EAP-SNAPSHOT or fresh RC build
#                  numbers) resolve from `snapshots`. Both are tried.
#   <ClassName>    e.g. com.intellij.platform.lsp.api.LspServer
#   [module]       platform module jar name (default: lsp). Other useful
#                  ones: platform-impl, analysis-api, core-api, lang-impl.
#   [-c]           also print method bodies (javap -c), so you can see
#                  invokestatic/invokeinterface at call sites.
#
# Examples:
#   # Where does sendRequestSync live in the floor build vs a failing build?
#   inspect_sdk.sh 241.14494.240 com.intellij.platform.lsp.api.LspServer
#   inspect_sdk.sh 262.8665.176  com.intellij.platform.lsp.api.LspServer
#   inspect_sdk.sh 262.8665.176  com.intellij.platform.lsp.api.LspClient
#
# List available versions of a module (when you need an exact build number):
#   inspect_sdk.sh --list [module]
#
# Env:
#   SDK_CACHE   where to cache downloaded jars (default: <repo>/tmp/idea-sdk-jars)

set -euo pipefail

REPO_BASE="https://www.jetbrains.com/intellij-repository"
GROUP_PATH="com/jetbrains/intellij/platform"

repo_root() {
  git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel 2>/dev/null \
    || echo "${PWD}"
}
CACHE="${SDK_CACHE:-$(repo_root)/tmp/idea-sdk-jars}"

die() { echo "error: $*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

list_versions() {
  local module="${1:-lsp}"
  echo "== available '${module}' versions (releases) =="
  curl -sSL "${REPO_BASE}/releases/${GROUP_PATH}/${module}/maven-metadata.xml" \
    | grep -oE '<version>[^<]+</version>' | sed -E 's|</?version>||g' | tail -25
  echo
  echo "== available '${module}' versions (snapshots: EAP/RC) =="
  curl -sSL "${REPO_BASE}/snapshots/${GROUP_PATH}/${module}/maven-metadata.xml" \
    | grep -oE '<version>[^<]+</version>' | sed -E 's|</?version>||g' | tail -25
}

# Fetch <module>-<version>.jar, trying releases then snapshots. Echoes the
# local jar path on success.
fetch_jar() {
  local version="$1" module="$2"
  local jar="${CACHE}/${module}-${version}.jar"
  if [ -s "$jar" ] && unzip -l "$jar" >/dev/null 2>&1; then
    echo "$jar"; return 0
  fi
  mkdir -p "$CACHE"
  local repo url code
  for repo in releases snapshots; do
    url="${REPO_BASE}/${repo}/${GROUP_PATH}/${module}/${version}/${module}-${version}.jar"
    code=$(curl -sSL -o "${jar}.part" -w '%{http_code}' "$url" 2>/dev/null || true)
    if [ "$code" = "200" ] && unzip -l "${jar}.part" >/dev/null 2>&1; then
      mv "${jar}.part" "$jar"
      echo "fetched ${module}-${version}.jar from ${repo}" >&2
      echo "$jar"; return 0
    fi
    rm -f "${jar}.part"
  done
  die "could not fetch ${module}-${version}.jar from releases or snapshots.
Hint: run '$(basename "$0") --list ${module}' to see valid version strings,
and confirm the class really lives in the '${module}' module (try another
module name as the 3rd argument)."
}

main() {
  need curl; need unzip; need javap

  if [ "${1:-}" = "--list" ]; then
    list_versions "${2:-lsp}"; exit 0
  fi

  local version="${1:?usage: inspect_sdk.sh <version> <class> [module] [-c]}"
  local class="${2:?usage: inspect_sdk.sh <version> <class> [module] [-c]}"
  local module="lsp" cflag=""
  shift 2
  for arg in "$@"; do
    case "$arg" in
      -c) cflag="-c" ;;
      *)  module="$arg" ;;
    esac
  done

  local jar; jar="$(fetch_jar "$version" "$module")"

  echo "########## ${class}  @  ${module}-${version} ##########"
  # javap reads classes straight out of a jar on the classpath. -p shows
  # private/synthetic members (the $default bridges), which is the whole point.
  javap -p ${cflag} -classpath "$jar" "$class" \
    || die "class '${class}' not found in ${module}-${version}.jar.
It may live in a different platform module — try a different [module] arg,
or grep the jar: unzip -l '${jar}' | grep -i '${class##*.}'"
}

main "$@"
