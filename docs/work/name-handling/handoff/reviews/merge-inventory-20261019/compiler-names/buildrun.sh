#!/usr/bin/env bash
# usage: buildrun.sh TAG [early|signature|bindings]
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
source /tmp/tcl-lsp-naming-env.sh
D="$PWD/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-names"
EVIDENCE=$D
D=/tmp/compiler-names-g4
mkdir -p "$D"
TAG=${1:?tag required}
STAGE=${2:-early}
run() {
    local label=$1 filter=$2
    (
        flock -x 9
        echo "LOCK_CLAIMED $TAG-$label $(date -u +%FT%TZ)"
        available=$(df --output=avail -B1 "$PWD" | awk 'NR==2 {print $1}')
        if (( available < 2000000000 )); then echo DISK_BELOW_2GB; exit 90; fi
        cargo test --locked -p tcl-compiler --lib "$filter" -- --nocapture --test-threads=1 2>&1 | python3 "$EVIDENCE/capped-log.py" "$D/$TAG-$label.log"
        statuses=("${PIPESTATUS[@]}")
        printf '%s\n' "cargo=${statuses[0]} filter=${statuses[1]}" > "$D/$TAG-$label.exit"
        if rg -q '^error.*failed to get|^error.*could not compile' "$D/$TAG-$label.log"; then exit 91; fi
        rg '^test result:|^error' "$D/$TAG-$label.log" || true
        # Fresh shared executables stay in place for at least sixty minutes.
        echo "LOCK_RELEASED $TAG-$label $(date -u +%FT%TZ)"
    ) 9>/tmp/tcl-lsp-naming-build.lock
}
case "$STAGE" in
    early)
        run main completed_world_follows
        ZZ_PROBE_ENSEMBLE=1 ZZ_PROBE_OPAQUE=1 run zz zz_probe_import_and_crlf
        for filter in original_readonly_worker_formal crlf_class_method substituted_heads_and_consumed; do run "$filter" "$filter"; done
        ZZ_PROBE_LOWER=1 run w210 w210_pkgindex
        while IFS= read -r filter; do run "regress-${filter##*::}" "$filter"; done < "$EVIDENCE/g1-command_binding-pass2fail.txt"
        ;;
    signature) run signature_scan signature_scan:: ;;
    bindings) run command_binding command_binding:: ;;
    *) echo "unknown stage: $STAGE" >&2; exit 2 ;;
esac
