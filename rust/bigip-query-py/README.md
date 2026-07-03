# f5report — BIG-IP reports powered by the query engine (via PyO3)

`f5report` turns one or more F5 BIG-IP configs — plain `bigip.conf` / SCF, or a
`.ucs` archive (plain **or** OpenPGP-encrypted) — into a single, self-contained,
interactive HTML report: virtual servers, pools and members, nodes, monitors,
iRules, data groups and SSL profiles, plus a **reference/orphan analysis** that
flags every object nothing points at.

It exists to demonstrate the power of the tcl-lsp **BIG-IP query engine**
(`tcl-bigip-query`, the jq-flavoured `f5-query` DSL). Every fact in the report
is pulled from that engine in-process through a PyO3 binding — **no subprocess,
no shelling out** to the `f5-query` binary, no re-implementation of the config
parser in Python.

```
┌───────────────┐   PyO3    ┌────────────────────┐   Jinja   ┌─────────────┐
│  f5report     │ ───────►  │  _engine (Rust)    │           │ report.html │
│  (Python 3.14)│           │  tcl-bigip-query   │           │ (1 file)    │
│  report.py    │ ◄───────  │  tcl-bigip-io (UCS)│  ───────► │ dark/light  │
└───────────────┘  native   └────────────────────┘           └─────────────┘
     objects
```

## Layout

| Path | What |
|------|------|
| `src/lib.rs` | The PyO3 extension module `f5report._engine`: `query()`, `load_paths()`, `ucs_to_scf()`. Converts engine `Value`s to native Python objects (no JSON round-trip). |
| `python/f5report/report.py` | Runs the engine queries and shapes the report model, incl. the `referenced_by` graph → orphan detection. |
| `python/f5report/render.py` + `templates/` | Jinja rendering to one standalone HTML file (embedded CSS/JS, no external assets). |
| `python/f5report/__main__.py` | The `f5-report` CLI. |
| `tests/` | pytest suite + real-world config fixtures (see `tests/data/PROVENANCE.md`). |

This crate is **excluded** from the Cargo workspace (like `editors/zed`): PyO3's
generated glue trips the workspace `unsafe_code = "forbid"` lint, and the cdylib
links libpython. It is built with **maturin**, not `cargo` directly.

## Building

Requires Python **3.14** and Rust ≥ 1.96.

```bash
python -m venv .venv && . .venv/bin/activate
pip install maturin
cd rust/bigip-query-py
maturin develop          # builds _engine and installs f5report editable
pytest tests/            # 20 tests
```

## Using it

Command line:

```bash
f5-report device.ucs -o report.html                 # one device
f5-report *.ucs -t "Production Estate" -o estate.html
f5-report backup.ucs --passphrase "$PW"             # encrypted UCS
f5-report device.ucs --json -o model.json           # raw report model
python -m f5report samples/bigip/bigip.conf -o out.html
```

As a library — the query engine is right there:

```python
import f5report

sources = f5report.load_paths(["device.ucs"])          # UCS → SCF, in-process

# Drive the DSL directly; results come back as native dict/list/str/…
orphan_pools = f5report.query(
    '.ltm.pool[] | select((referenced_by(.) | length) == 0) | .name',
    sources,
)

html = f5report.build_report(sources, title="Production LTM")
open("report.html", "w").write(html)
```

`f5report.query(expr, sources, *, merge=False, partitions=None, per_file=False)`
is the whole engine: pass any `f5-query` expression and it evaluates in-process
against the loaded configs. Mutating expressions are rejected (a report never
rewrites its input).
