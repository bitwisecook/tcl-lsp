#!/usr/bin/env bash
# Rule-only creation/cleanup. No virtual-server changes and no config save.
set -euo pipefail
umask 077
usage() { echo 'usage: appliance-rules.sh load|cleanup FIXTURE_DIR JOURNAL_DIR [CASE ...]' >&2; exit 2; }
[[ $# -ge 3 ]] || usage
mode=$1
fixtures=$2
journal=$3
shift 3
[[ $mode == load || $mode == cleanup ]] || usage
[[ -f $fixtures/rules.tsv ]] || { echo 'missing rules.tsv' >&2; exit 2; }
[[ -d $journal ]] || mkdir -m 700 "$journal"
# An interrupted previous load is deliberately not overwritten.
if [[ $mode == load ]]; then
  [[ ! -e $journal/owned.tsv ]] || { echo 'journal already exists; clean it first' >&2; exit 2; }
  : > "$journal/owned.tsv"
fi
exists() {
  local object=$1 output rc
  if output=$(tmsh list ltm rule "$object" 2>&1); then
    printf '%s\n' "$output"; return 0
  else
    rc=$?
    printf '%s\n' "$output" >&2
    if [[ $output == *'was not found'* || $output == *'does not exist'* ]]; then return 1; fi
    echo "Cannot prove absence: tmsh status $rc" >&2
    exit 3
  fi
}
if [[ $mode == cleanup ]]; then
  [[ -f $journal/owned.tsv ]] || { echo 'missing ownership journal' >&2; exit 2; }
  while IFS=$'\t' read -r object checksum; do
    [[ $object =~ ^/Common/__tcl_lsp_probe_2286_[A-Za-z0-9_]+$ ]] || exit 3
    if exists "$object" > "$journal/current.conf"; then
      actual=$(sha256sum "$journal/current.conf"); actual=${actual%% *}
      [[ $actual == "$checksum" ]] || { echo "Changed object: refuse deletion $object" >&2; exit 3; }
      tmsh delete ltm rule "$object" > "$journal/delete-${object##*/}.log" 2>&1
      if exists "$object" > /dev/null; then echo "Deletion failed $object" >&2; exit 3; fi
    fi
    printf 'ABSENT %s\n' "$object"
  done < "$journal/owned.tsv"
  mv "$journal/owned.tsv" "$journal/cleaned.tsv"
  exit 0
fi
[[ $# -gt 0 ]] || { echo 'Explicit case selection required; no load-all mode' >&2; exit 2; }
for case in "$@"; do
  [[ $case =~ ^[A-Za-z0-9_]+$ ]] || exit 2
  object=$(awk -F '\t' -v file="$case.conf" '$1==file {print $2}' "$fixtures/rules.tsv")
  [[ $object =~ ^/Common/__tcl_lsp_probe_2286_[A-Za-z0-9_]+$ ]] || { echo "unknown case $case" >&2; exit 2; }
  if exists "$object" > /dev/null; then echo "Collision: refuse modification $object" >&2; exit 3; fi
  # Preserve ALL original loader output, including warning text and exit status.
  set +e
  tmsh load sys config merge file "$fixtures/$case.conf" > "$journal/load-$case.log" 2>&1
  rc=$?
  set -e
  printf '%s\t%s\n' "$case" "$rc" >> "$journal/load-status.tsv"
  if exists "$object" > "$journal/created-$case.conf"; then
    hash=$(sha256sum "$journal/created-$case.conf"); hash=${hash%% *}
    printf '%s\t%s\n' "$object" "$hash" >> "$journal/owned.tsv"
    echo "PRESENT $case load-status=$rc (inspect full load log for warnings)"
  else
    echo "ABSENT $case load-status=$rc (inspect full load log for rejection)"
  fi
  if [[ $rc -ne 0 ]]; then echo 'Stopping at load failure; cleanup successful creations using this journal.'; exit "$rc"; fi
done
