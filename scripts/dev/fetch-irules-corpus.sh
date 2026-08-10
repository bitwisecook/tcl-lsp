#!/usr/bin/env bash
# Fetch a third-party iRules corpus into tmp/ for the false-positive audit
# sweep (issue #1316; docs/design/compiler/fp-audit-todo.md).
#
# The IRULE1002-5007 rows of that checklist sat open for want of a real
# corpus: the in-repo `samples/irules/` set is curated, small, and written to
# demonstrate one diagnostic apiece, so it exercises almost none of the
# family. These nine public repositories are real, production-shaped iRules.
#
# Idempotent: a repo already present is left alone (delete its directory to
# refresh). Nothing here is vendored into the repository — the corpus is
# third-party code under its own licences and lives only in tmp/, exactly as
# `.claude/skills/fetch-tcl-source` handles the Tcl source trees.
#
# Deviation from that skill's convention, deliberately: it prefers codeload
# tarballs (CDN-cached, no .git, easier on upstream), but this sandbox's
# egress policy answers codeload.github.com with 403 while allowing the git
# protocol, so this uses `git clone --depth 1`. Prefer tarballs if you are
# somewhere codeload is reachable.
set -euo pipefail

DEST="${1:-tmp/irules-corpus}"

REPOS=(
  f5devcentral/f5-agility-labs-irules
  f5devcentral/irules-toolbox
  e-XpertSolutions/f5
  landro/TesTcl
  megamattzilla/iRules
  JuergenMang/f5-irules-json
  pwhitef5/simple-sideband
  0xHiteshPatel/HTTP_Debug_iRule
  erkac/f5-irules
)

mkdir -p "$DEST"

for repo in "${REPOS[@]}"; do
  name="${repo//\//_}"
  target="$DEST/$name"
  if [ -d "$target" ]; then
    echo "==> $repo already present — skipping"
    continue
  fi
  echo "==> cloning $repo"
  if ! git clone --depth 1 -q "https://github.com/$repo.git" "$target"; then
    echo "    WARNING: $repo failed to clone — continuing without it" >&2
    rm -rf "$target"
  fi
done

echo
echo "==> corpus at $DEST"

# `xtask fp-sweep` also recognises direct `.txt` iRules and iRules-bearing
# reStructuredText `code` / `code-block` directives. The extension count below
# remains useful as the native-source subset; the event-signature count covers
# every publication format.
sweepable=$(find "$DEST" -type f \( -name '*.tcl' -o -name '*.irule' -o -name '*.irul' \) \
  -not -path '*/.git/*' 2>/dev/null | wc -l | tr -d ' ')
witheventsig=$(grep -rlE '^[[:space:]]*when[[:space:]]+[A-Z][A-Z0-9_]*' "$DEST" \
  --exclude-dir=.git 2>/dev/null | wc -l | tr -d ' ')
echo "    $sweepable native source file(s) (.tcl/.irule/.irul)"
echo "    $witheventsig file(s) carrying a 'when EVENT' signature (any extension)"
echo
echo "Sweep the iRules diagnostic family with, e.g.:"
echo "  cargo xtask fp-sweep --code IRULE1002 --code IRULE1003 \\"
echo "    --code IRULE1004 --code IRULE1005 \\"
echo "    \$(for d in $DEST/*/; do echo --corpus \"\$d\"; done)"
