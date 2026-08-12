<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# Capabilities — how to leverage tcl-lsp

tcl-lsp is three capabilities behind one toolchain. Each is reachable from a CLI,
a library/binding, a WebAssembly app, and (for analysis) the LSP + an MCP server.
Start here, then follow the reference links.

Live, in-browser demos (nothing installed, nothing uploaded):
**[compiler explorer](https://bitwisecook.github.io/tcl-lsp/compiler-explorer/)** ·
**[BIG-IP report generator](https://bitwisecook.github.io/tcl-lsp/bigip-report-generator/)** ·
**[example BIG-IP report](https://bitwisecook.github.io/tcl-lsp/bigip-report-demo/)**

---

## 1. f5q — the F5 BIG-IP query engine

A jq-flavoured query DSL over BIG-IP configs (UCS / SCF / `bigip.conf`), plus the
report generator built on it.

| Surface | How |
|---------|-----|
| **CLI** | `f5 query '.ltm.virtual[].name' device.ucs` (alias `f5 q`). Renderers: `f5 q --render mermaid …`. Help: `f5 query --help-manual \| --help-dsl \| --help-builtins`. |
| **Python** | `import f5report; html = f5report.build_report(f5report.load_paths(["dev.ucs"]))`; ad-hoc `f5report.query('.ltm.pool[]', sources)`. |
| **Web** | the in-browser report generator (above); every generated report embeds a live f5-query **console** and the **full manual**. |
| **Server** | `f5-report-web` — upload a config, get a report server-side (`rust/bigip-report-gen/python`). |

**Enrichment / side-inputs** — a query (and the report) can bind external tables to
`$name`: `--input csv nat=nat.csv`, `--input zone dns=example.com.zone` (a new
RFC 1035 DNS zone-file format), `--input json`/`jsonl`/`f5log`. The report's
**topology + enrichment DSL** (the architecture manifest) declares devices, tiers,
links, network **zones**, device **interfaces**, **DNS zones**, and CIDR/service/NAT
maps in one place — see `rust/tcl-bigip-query/src/architecture.rs`.

Reference: **[docs/references/f5_query/](references/f5_query/)** (`manual.md`, `dsl.md`,
`builtins.md`), how-tos in **[docs/kcs/](kcs/)** (`kcs-howto-*query*`), internals in
`docs/design/f5-query-engine-internals.md`. Crate: `rust/tcl-bigip-query`.

---

## 2. The Tcl compiler / analyser

A real Tcl front-end (lexer → green tree → IR → CFG → SSA → optimiser → codegen) that
powers iRule analysis, diagnostics, and the compiler explorer.

| Surface | How |
|---------|-----|
| **LSP** | `tcl-lsp-server`, used by the editors under `editors/` (VS Code, Neovim, Zed, Emacs, Helix, JetBrains, Sublime): diagnostics, semantic tokens, hover, completion, refactors. |
| **CLI** | `tcl compile \| diag \| diff \| highlight \| explore …`. Compiler-explorer views: `tcl explore ir\|cfg\|ssa\|opt\|asm\|wasm …`, web GUI `tcl explore --serve`. |
| **Web** | the compiler explorer (above), the analysis core compiled to WASM, client-side. |
| **MCP** | `tcl-mcp` exposes ~40 analysis tools (`analyze`, `validate`, `review`, `optimize`, `call_graph`, `dataflow_graph`, `goto_definition`, refactors, …). |

Reference: `docs/design/compiler-architecture.md` (+ `docs/design/compiler/`), how-tos in
`docs/kcs/`, generated tables in
`docs/generated/`. Crates: `rust/tcl-lexer`, `rust/tcl-syntax`, `rust/tcl-compiler`,
`rust/tcl-lsp-core`, `rust/tcl-explorer`.

---

## 3. The BIG-IP registries

Command + event registries and the `f5-irules` dialect: what backs hover/completion,
event-validity diagnostics, and iRule event ordering.

| Surface | How |
|---------|-----|
| **CLI** | `f5 irule event-order some.irule`; `f5 irule event-info HTTP_REQUEST --json` (multiplicity, side, transport, implied profiles, valid commands); `tcl registry` / `tcl lookup`. |
| **MCP** | `event_info` and `command_info` tools (`tcl-mcp`). |
| **LSP** | drives hover, completion, signature help, and event-validity diagnostics in every editor. |

Reference: `docs/design/bigip-registry-architecture.md`, contracts in
`docs/design/contracts/command-registry-event-model.md`, features in
`docs/kcs/features/kcs-feature-bigip-registry.md` / `kcs-feature-event-registry.md`.
Crates: `rust/tcl-registry`, `rust/tcl-irules`.
