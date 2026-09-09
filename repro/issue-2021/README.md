# Issue #2021 reproduction artefacts (temporary — remove before the PR)

Harness and evidence for the orphaned-server report. Nothing here ships.

- `orphan_repro.py` — stdio JSON-RPC driver: starts the server on a workspace,
  opens a document, tears the client down (`--scenario clean|eof|shutdown-eof|exit-stdout-open`,
  `--at midscan|settled`), then samples CPU / RSS / threads and takes a gdb
  stack sample of any survivor.
- `run_all.sh` — the full sweep.
- `RESULTS.md` — the results table and stack excerpts.
- `*.gdb.txt` — thread stacks of orphaned servers; `runs*.jsonl` — raw samples.

Finding: every teardown style orphans the server while the `initialized`
handler's workspace scan is still running; nothing cancels the scan on
`shutdown`, `exit`, stdin EOF or EPIPE, and `Server::serve` cannot return
until that handler future completes.

```bash
python3 repro/issue-2021/orphan_repro.py --binary target/release/tcl-lsp-server \
  --workspace tmp/tcllib-2.0 --scenario eof --at midscan --observe 45
```
