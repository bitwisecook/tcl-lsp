# f5report — BIG-IP reports powered by the query engine (via PyO3)

`f5report` turns one or more F5 BIG-IP configs — plain `bigip.conf` / SCF, or a
`.ucs` archive (plain **or** OpenPGP-encrypted) — into a single, self-contained,
interactive HTML report: virtual servers, pools and members, nodes, monitors,
iRules, data groups and SSL profiles, plus a **reference/orphan analysis** that
flags every object nothing points at.

It also embeds four interactive views, all client-side in the one HTML file:

- **Topology** — a Mermaid object graph (vendored Mermaid, inlined). Every
  object is clickable → a detail drawer with its neighbourhood diagram; for a
  virtual, its processing-flow diagram and its static **plus iRule-driven**
  profiles. Pools referenced *inside* an iRule appear as dashed edges. Clicking
  a connector line lights up the whole connected component.
- **Listener Matcher / simulator** — enter a client flow (source/dest/port/
  protocol/VLAN/route-domain, IPv4 **and** IPv6); the most-specific listener is
  highlighted and the fall-through order shown. Click a listener to load the
  exact flow that reaches it and simulate its processing: client SSL
  (cert/key/ciphers, SNI), the applied profiles, LTM policy rules evaluated
  against the request, iRule actions (pool selection, header rewrites, redirects,
  persistence), the resulting HTTP request, and load-balancing / member
  selection.
- **Query Console** — the real `f5-query` DSL compiled to **WebAssembly** and
  embedded with the config, so you can run live queries against the device
  entirely in the browser (nothing leaves the page).

It exists to demonstrate the power of the tcl-lsp **BIG-IP query engine**
(`tcl-bigip-query`, the jq-flavoured `f5-query` DSL). Every fact in the report
is pulled from that engine in-process through a PyO3 binding — **no subprocess,
no shelling out** to the `f5-query` binary, no re-implementation of the config
parser in Python.

```
┌───────────────┐   PyO3    ┌────────────────────┐ MiniJinja ┌─────────────┐
│  f5report     │ ───────►  │  _engine (Rust)    │           │ report.html │
│ (Python 3.9+) │           │  tcl-bigip-query   │           │ (1 file)    │
│  report.py    │ ◄───────  │  tcl-bigip-io (UCS)│  ───────► │ dark/light  │
└───────────────┘  native   └────────────────────┘           └─────────────┘
     objects
```

## Layout

| Path | What |
|------|------|
| `src/lib.rs` | The PyO3 extension module `f5report._engine`: `query()`, `load_paths()`, `ucs_to_scf()`, `sys_file_ssl_certs()` / `sys_file_ssl_keys()` (cert inventory), `decrypt_secrets()` (`f5mku` master-key secret decryption). Converts engine `Value`s to native Python objects (no JSON round-trip). |
| `python/f5report/report.py` | Runs the engine queries and shapes the report model, incl. the `referenced_by` graph → orphan detection. |
| `python/f5report/certs.py` | The SSL-certificate + private-key expiry inventory (answers "which certs are expiring, and what do they front?"). |
| `python/f5report/render.py` + `templates/` | MiniJinja rendering to one standalone HTML file (embedded CSS/JS, no external assets). |
| `python/f5report/__main__.py` | The `f5-report` CLI (`--f5mku` / `--f5mku-file` reveal `$M$` secrets). |
| `tests/` | pytest suite + real-world config fixtures (see `tests/data/PROVENANCE.md`). |

> This Python package is deliberately kept as the demonstration of using the
> BIG-IP query engine as a **library** (via PyO3). The same generator, ported to
> Rust and compiled to WebAssembly so it runs entirely in the browser, lives in
> [`../rust`](../rust) + [`../wasm`](../wasm);
> the two are kept at feature parity.

This crate is **excluded** from the Cargo workspace (like `editors/zed`): PyO3's
generated glue trips the workspace `unsafe_code = "forbid"` lint, and the cdylib
links libpython. It is built with **maturin**, not `cargo` directly.

## Building

Requires Python **3.9+** and Rust ≥ 1.96. The extension is built against the
CPython **stable ABI** (`abi3`, 3.9 floor), so one wheel loads on every CPython
from 3.9 up — the interpreter you build with need not match the one that runs it.

```bash
python -m venv .venv && . .venv/bin/activate
pip install maturin
cd rust/bigip-report-gen/python
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
