# Server, e2e and DB, 19 November 2026

## Stopped work

The resumed clean `it_5` attempt stopped at 14:19 UTC while the single rustc process was compiling `tcl-compiler`. It had never linked the DB test binary. The parent process group was terminated and the timing runner never started. An earlier three-target link had exited with SIGKILL before linking. Both terminal tails are in `measurement/db-it_5-build.log.txt` and `measurement/db-build.log.txt`. None of the three selected DB binaries has a fresh result, elapsed-time series or timeout. Historical long runs in the 18 November handoff retain their original dates and profiles.

## Restart

Take a clean checked-out `work` baseline first. The interrupted measurement head was `a6c66517c7`. Set `DB_HEAD` to the exact ten-digit SHA being measured and choose a new, head-specific output directory if the head changes. On this host the previous clean clone is `/tmp/bd-lsp`, its output directory is `/workspace/tcl-lsp/tmp/bd-db-timing-a6c66517c7`, its warm target is `/workspace/bd-build/lsp`, and the verified shell settings are in `/workspace/tcl-lsp/tmp/oracle-env.sh`. The two portable scripts in `measurement/` take paths via environment variables. Keep them outside the clean checkout when necessary.

```bash
export DB_CHECKOUT=/tmp/bd-lsp
export DB_HEAD=a6c66517c7
export DB_EVIDENCE=/workspace/tcl-lsp/tmp/bd-db-timing-a6c66517c7
export CARGO_TARGET_DIR=/workspace/bd-build/lsp
export ORACLE_ENV=/workspace/tcl-lsp/tmp/oracle-env.sh
source "$ORACLE_ENV"
# In DB_CHECKOUT rebuild Jim using bash scripts/dev/ensure-jim-oracle.sh.
# Set MEASUREMENT to this handoff's measurement directory.
for target in it_5 interned_gc compiler_check_corpus; do
    bash "$MEASUREMENT/db_link_one.sh.txt" "$target" || break
    python3 "$MEASUREMENT/db_timing.py.txt" "$target" || break
done
```

Reserve a single compile slot from the coordinator, then measure one DB test at a time. The runner verifies the clean SHA, oracle variables and Jim patchlevel, and consumes the actual cargo executable manifest. It records every exact test name in a per-target TSV, along with wall time, return code and binary path. It runs from the same package directory as Cargo, identifies ignored tests through libtest, stops each case after ten minutes and distinguishes failures from timeouts. Logs are capped at 1 MiB. Reusing completed rows is only appropriate with the same head, oracle versions, profile and executable. Send the named slow or nonreturning cases to analysis-performance. Retain exact joined e2e rename controls with diagnostics.

No server/DB source patch was applied to the clean measurement checkout. The optional #2543 debug instrumentation is parked separately in the analysis-performance handoff.
