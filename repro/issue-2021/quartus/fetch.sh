#!/bin/bash
BASE=/tmp/claude-0/-home-user-tcl-lsp/e2b9cc4f-a1cb-5ec8-b576-3c86047f6744/scratchpad/quartus
cd "$BASE" || exit 1
REPO=alterafpga/quartus-std
DEST="$BASE/layer-extract"
mkdir -p "$DEST"
LOG="$BASE/fetch.log"
: > "$LOG"

get_token() {
  curl -sS --cacert /root/.ccr/ca-bundle.crt \
    "https://auth.docker.io/token?service=registry.docker.io&scope=repository:${REPO}:pull" \
  | python3 -c "import json,sys;print(json.load(sys.stdin)['token'])"
}

mapfile -t LAYERS < <(python3 -c "
import json
m=json.load(open('$BASE/manifest.json'))
for l in m['layers']: print(l['digest'], l['size'])
")

prev=0
i=0
for entry in "${LAYERS[@]}"; do
  i=$((i+1))
  set -- $entry
  DIGEST=$1; SIZE=$2
  echo "=== layer $i digest=$DIGEST compressed=$SIZE ===" >> "$LOG"
  TOKEN=$(get_token)
  start=$(date +%s)
  curl -sSL --cacert /root/.ccr/ca-bundle.crt -H "Authorization: Bearer $TOKEN" \
    "https://registry-1.docker.io/v2/${REPO}/blobs/${DIGEST}" \
  | tar -xz -C "$DEST" --no-same-owner --wildcards --no-anchored --ignore-case \
      'ip/altera/*.tcl' 'ip/altera/*.qsf' 'ip/altera/*.qpf' 'ip/altera/*.qip' 'ip/altera/*.sdc' \
      2>> "$LOG"
  rc=${PIPESTATUS[1]}
  end=$(date +%s)
  now=$(find "$DEST" -type f 2>/dev/null | wc -l)
  echo "layer $i rc=$rc secs=$((end-start)) files_total=$now added=$((now-prev))" >> "$LOG"
  prev=$now
done
echo "DONE" >> "$LOG"
