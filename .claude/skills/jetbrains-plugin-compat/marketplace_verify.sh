#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# marketplace_verify.sh — read (and optionally schedule) the JetBrains
# Marketplace's own IntelliJ-Plugin-Verifier results for our plugin, the same
# verdicts shown on the plugin's "…/edit/versions/{stable,eap}" admin page.
#
# The Marketplace runs the verifier against a matrix of released IDE builds
# after every upload and exposes the results over its REST API. Reading them
# requires the maintainer token (READ_VERIFICATION_RESULTS permission); the
# token is resolved via scripts/release/jetbrains_token.sh (env var, macOS
# Keychain, or libsecret) and is NEVER printed or written to disk here.
#
# This is the *post-publish* half of the compatibility story. It tells you how
# a build that is already on the Marketplace fares against IDE versions newer
# than anything in build.gradle.kts's local `pluginVerification.ides` list —
# which is exactly how the LspServer.sendRequestSync$default breakage first
# surfaced (a 2.1.x build flagged CRITICAL on 2026.1/2026.2 that our local
# verifier targets never covered). Use `make verify-editor-jetbrains` for the
# pre-publish gate; use this to read what the Marketplace found afterward.
#
# Usage:
#   marketplace_verify.sh [results] [--channel eap|stable] [--version X.Y.Z]
#                         [--update <id>] [--full]
#   marketplace_verify.sh products  [--channel eap|stable] [--version X.Y.Z]
#                         [--update <id>]
#   marketplace_verify.sh schedule  --ide <ideVersion> [--update <id>]
#                         [--channel eap|stable] [--version X.Y.Z]
#
#   results   (default)  Per-IDE verdict table. Exits non-zero if any verdict
#                        is CRITICAL / PROBLEMS / INVALID_PLUGIN, so it can gate
#                        a release. With no --channel, checks BOTH channels'
#                        latest builds. --full also prints the exact problem
#                        messages from each failing build's report.
#   products             List the IDE builds the Marketplace can verify against
#                        (the input to `schedule --ide`).
#   schedule             Enqueue a verification run for one IDE build (the
#                        "Schedule Verification" button). Side-effecting.
#
# Env overrides:
#   JB_PLUGIN_ID   numeric Marketplace plugin id (default: 31801)
#   JETBRAINS_TOKEN  token (else resolved via jetbrains_token.sh)

set -euo pipefail

BASE="https://plugins.jetbrains.com"
PLUGIN_ID="${JB_PLUGIN_ID:-31801}"
VTYPE="INTELLIJ_COMPATIBILITY"   # the plugin-verifier compatibility results

die() { echo "error: $*" >&2; exit 2; }
need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

repo_root() {
  git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel 2>/dev/null || echo "$PWD"
}

resolve_token() {
  if [ -n "${JETBRAINS_TOKEN:-}" ]; then printf '%s' "$JETBRAINS_TOKEN"; return 0; fi
  local script; script="$(repo_root)/scripts/release/jetbrains_token.sh"
  [ -x "$script" ] && "$script" 2>/dev/null && return 0
  die "no Marketplace token: set \$JETBRAINS_TOKEN or store it for jetbrains_token.sh"
}

# GET/POST with bearer auth. Token stays in the header only; never echoed.
api() { # api <method> <url-path> [outfile]
  local method="$1" path="$2" out="${3:-/dev/stdout}"
  curl -sSL -X "$method" -H "Authorization: Bearer $TOKEN" \
       -o "$out" -w '%{http_code}' "$BASE$path"
}

# Resolve an update id from --update, or --version within --channel, or the
# latest build in the channel. Echoes the update id.
resolve_update() {
  local channel="$1" version="$2" update="$3"
  [ -n "$update" ] && { echo "$update"; return; }
  local tmp; tmp="$(mktemp)"
  local q="/api/plugins/${PLUGIN_ID}/updates?size=50"
  # The public Stable channel is the *default/empty* Marketplace channel (this
  # repo publishes stable as `channels = listOf("default")`). Querying
  # `&channel=stable` (or `default`) targets a nonexistent named channel and
  # returns nothing, so normalise those to no channel filter. Only a real named
  # channel like `eap` gets the query parameter.
  case "$channel" in
    ""|stable|default) : ;;
    *) q="${q}&channel=${channel}" ;;
  esac
  local code; code="$(api GET "$q" "$tmp")"
  [ "$code" = "200" ] || { rm -f "$tmp"; die "updates list failed (HTTP $code)"; }
  python3 - "$tmp" "$version" <<'PY'
import sys,json
ups=json.load(open(sys.argv[1])); want=sys.argv[2]
ups=ups if isinstance(ups,list) else ups.get("updates",[])
if not ups: sys.exit("no updates in channel")
if want:
    m=[u for u in ups if u.get("version")==want]
    if not m: sys.exit(f"version {want} not found in channel")
    print(m[0]["id"])
else:
    print(ups[0]["id"])
PY
  rm -f "$tmp"
}

fetch_results() { # fetch_results <update_id> -> json on stdout
  local update="$1" tmp; tmp="$(mktemp)"
  local code; code="$(api GET "/api/verifications/update/${update}?verificationType=${VTYPE}" "$tmp")"
  [ "$code" = "200" ] || { cat "$tmp" >&2; rm -f "$tmp"; die "results fetch failed (HTTP $code)"; }
  cat "$tmp"; rm -f "$tmp"
}

print_results() { # print_results <update_id> <label> <full?>
  local update="$1" label="$2" full="$3"
  local jf; jf="$(mktemp)"; fetch_results "$update" > "$jf"
  echo "== update $update ($label) — INTELLIJ_COMPATIBILITY =="
  local rc
  # NB: `python3 -` reads the program from the heredoc, which exhausts stdin,
  # so the results JSON is passed by file path (argv[1]) rather than piped in.
  rc="$(TOKEN="$TOKEN" BASE="$BASE" FULL="$full" python3 - "$jf" <<'PY'
import sys,json,os,urllib.request
d=json.load(open(sys.argv[1]))
items=d if isinstance(d,list) else d.get("data") or [d]
bad={"CRITICAL","PROBLEMS","INVALID_PLUGIN"}
print(f"  {'VERDICT':10} {'PRODUCT':14} {'IDE':22} SUMMARY")
fails=0
for r in sorted(items,key=lambda r:(r.get('resultType',''), (r.get('availableProduct') or {}).get('ideVersion',''))):
    rt=r.get("resultType","?")
    ap=r.get("availableProduct") or {}
    prod=ap.get("productName") or ap.get("product") or "?"
    ide=ap.get("ideVersion") or "?"
    vv=(r.get("verificationVerdict") or "").split(". ")[0][:52]
    print(f"  {rt:10} {prod:14} {ide:22} {vv}")
    if rt in bad:
        fails+=1
        if os.environ.get("FULL")=="1":
            url=r.get("fullVerificationResultUrl")
            if url:
                req=urllib.request.Request(os.environ['BASE']+url,
                    headers={'Authorization':'Bearer '+os.environ['TOKEN']})
                try:
                    rep=json.load(urllib.request.urlopen(req))
                except Exception as e:
                    print(f"      (could not fetch report: {e})"); continue
                seen=set()
                def walk(o):
                    if isinstance(o,dict):
                        for v in o.values(): yield from walk(v)
                    elif isinstance(o,list):
                        for v in o: yield from walk(v)
                    elif isinstance(o,str): yield o
                for s in walk(rep):
                    if ("unresolved method" in s or "NoSuchMethodError" in s
                            or "Invocation of" in s) and s not in seen:
                        seen.add(s); print("      ! "+s[:200])
print(f"__FAILS__{fails}")
PY
)"
  rm -f "$jf"
  echo "$rc" | grep -v '^__FAILS__'
  local n; n="$(printf '%s' "$rc" | sed -n 's/^__FAILS__//p')"
  return "$(( n > 0 ? 1 : 0 ))"
}

main() {
  need curl; need python3
  local cmd="results" channel="" version="" update="" full="0" ide=""
  [ $# -gt 0 ] && case "$1" in results|products|schedule) cmd="$1"; shift;; esac
  while [ $# -gt 0 ]; do
    case "$1" in
      --channel) channel="$2"; shift 2;;
      --version) version="$2"; shift 2;;
      --update)  update="$2"; shift 2;;
      --ide)     ide="$2"; shift 2;;
      --full)    full="1"; shift;;
      *) die "unknown arg: $1";;
    esac
  done
  TOKEN="$(resolve_token)"; [ -n "$TOKEN" ] || die "empty token"

  case "$cmd" in
    products)
      local u; u="$(resolve_update "$channel" "$version" "$update")"
      api GET "/api/verifications/update/${u}/products" /tmp/_p.json >/dev/null
      python3 -c 'import json;[print(p["ideVersion"],"\t",p.get("productName")) for p in json.load(open("/tmp/_p.json"))]'
      ;;
    schedule)
      [ -n "$ide" ] || die "schedule needs --ide <ideVersion> (see: $0 products)"
      local u; u="$(resolve_update "$channel" "$version" "$update")"
      echo "scheduling INTELLIJ_COMPATIBILITY verification of update $u against $ide …"
      local code; code="$(api POST "/api/verifications/update/${u}?ideVersion=${ide}&verificationType=${VTYPE}" /tmp/_s.json)"
      echo "HTTP $code"; cat /tmp/_s.json; echo
      [ "$code" = "200" ] || [ "$code" = "201" ] || die "schedule failed"
      ;;
    results)
      local rc=0
      if [ -n "$channel" ] || [ -n "$version" ] || [ -n "$update" ]; then
        local u; u="$(resolve_update "$channel" "$version" "$update")"
        print_results "$u" "${channel:-${version:-explicit}}" "$full" || rc=1
      else
        # No selector: health-check both channels' latest builds. A channel we
        # cannot resolve (bad token, API error, no updates) must FAIL the gate,
        # not be silently skipped — otherwise the command could print OK having
        # verified nothing.
        for ch in stable eap; do
          local u
          if ! u="$(resolve_update "$ch" "" "")"; then
            echo "!! could not resolve latest '$ch' update — failing the gate" >&2
            rc=1; continue
          fi
          print_results "$u" "$ch latest" "$full" || rc=1
          echo
        done
      fi
      [ "$rc" = 0 ] && echo "OK: no CRITICAL/PROBLEMS/INVALID_PLUGIN verdicts." \
                    || echo "FAIL: at least one build has compatibility problems."
      return "$rc"
      ;;
  esac
}

main "$@"
