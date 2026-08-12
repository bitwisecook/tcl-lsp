# tcl-lsp tooling/editor duplication audit

Branch: `claude/legacy-code-duplication-audit-edm0bo`. Scope: `rust/tcl-cli*`,
`rust/f5-cli`, `rust/tcl-mcp`, `rust/tcl-explorer*`, `rust/tcl-debugger`,
`rust/tcl-pkg`, `rust/tcl-diagram`, `rust/tcl-spec-studio`, `rust/xtask`,
`editors/`, `docs/generated/`, `scripts/`.

---

## F1: The native LSP's `diagramData` command is a private, shallower reimplementation of the shared `tcl-diagram` crate

**Confidence:** high
**Category:** duplicated-tooling

**Authoritative source:** `rust/tcl-diagram/src/data.rs:444-524` (`diagram_data` /
`diagram_data_for_dialect`) — projects a parsed script's IR into
`{events: [{name, priority, multiplicity, flow}], procedures: [{name, params,
flow}]}` (decision/action/assign/return/switch/loop nodes). Its module doc
(`rust/tcl-diagram/src/lib.rs:18-32`) states it exists precisely so "any
tool" shares one implementation, and names "the CLI, and the MCP" as
consumers.

- `rust/tcl-cli/src/commands/diagram.rs:32,42` — `tcl diagram` calls
  `tcl_diagram::diagram_data_for_dialect` directly.
- `rust/tcl-mcp/src/tools.rs:305-309` — the MCP `diagram` tool calls the same
  function: `tcl_diagram::diagram_data_for_dialect(source, registry(&dialect), &dialect)`.

**The duplicate / stale copy:** `rust/tcl-lsp-server/src/lib.rs:10600-10610`
— the `tcl-lsp.diagramData` custom-command handler:

```rust
fn diagram_data_command(args: &[serde_json::Value]) -> Option<serde_json::Value> {
    let source = args.first().and_then(serde_json::Value::as_str)?;
    let events: Vec<serde_json::Value> =
        tcl_lsp_core::irules_context::scan_file_events(source, "f5-irules")
            .into_iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect();
    Some(serde_json::json!({ "events": events }))
}
```

This calls `scan_file_events` (`rust/tcl-lsp-core/src/irules_context.rs:67-73`),
a helper that only regex/brace-scans `when EVENT { … }` headers for event
*names*. It returns `{"events": [{"name": "..."}]}` — no `procedures` key at
all, and each event carries only a bare name: no `priority`, no
`multiplicity`, no `flow`. `tcl-lsp-server`/`tcl-lsp-core`'s `Cargo.toml`
files have no dependency on `tcl-diagram` at all (`grep` for
`tcl-diagram`/`tcl_diagram` in both returns nothing).

**Is it gated?** No gate — this is hand-written Rust logic, not a generated
artefact, so no xtask drift check applies. The only test is
`rust/tcl-lsp-server/tests/e2e/commands.rs:354-368`
(`diagram_extracts_irule_events`), which asserts only that `events[].name`
contains the expected event name — it never asserts `procedures` or `flow`
exist, so it cannot catch the shape mismatch.

**Drift evidence:** Both consumer-side clients that call this command expect
the full `tcl-diagram` shape, not what the server actually returns:

- `editors/vscode/src/chat/commands/diagram.ts:25-35` declares
  `interface DiagramData { events: Array<{name, priority, multiplicity,
  flow}>; procedures: Array<{name, params, flow}>; error?}` and at line
  118-119 invokes `command: "tcl-lsp.diagramData"` expecting that shape; line
  134 checks `diagramData.events.length === 0 && diagramData.procedures.length
  === 0` — `procedures` is always `undefined` in the real response, so that
  branch can never distinguish "no procedures" from "field doesn't exist,"
  and the LLM prompt built from `structuredDataStr` (the raw JSON) is missing
  all flow/procedure data it is instructed to diagram.
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/actions/TclLspActions.kt:86-93`
  (`DiagramDataAction`) sets `resultExtension(result) = "mmd"` — i.e. it
  saves the command's raw JSON reply as a `.mmd` (Mermaid) scratch file
  (via `TclLspActionBase.stringifyResult`/`presentResult`,
  `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/actions/TclLspActionBase.kt:152-181`,
  which just does `Gson().toJson(result)`). The server's actual reply
  (`{"events":[{"name":"HTTP_REQUEST"}]}`) is not Mermaid syntax, so the
  resulting `diagram-data.mmd` scratch file cannot render as a diagram —
  this action does not produce a usable diagram today.
- By contrast, `editors/zed/README.md:206` advertises a `diagram` tool
  through the `tcl-lsp-mcp` context server — that one, being the real MCP
  `diagram` tool (`rust/tcl-mcp/src/tools.rs:305-309`), returns the correct
  full structure. **Same underlying question ("what is this iRule's
  structure?"), three different answers depending on which client path you
  go through** — exactly the two-source-of-truth shape this audit is hunting
  for.

**Why it matters:** VS Code's `/diagram` chat command and JetBrains' "Diagram
Data" action are both user-facing, advertised features
(`rust/tcl-lsp-server/tests/e2e/commands.rs:388` lists `tcl-lsp.diagramData`
as a `CORE_COMMANDS` every backend must expose) that silently produce
degraded or broken output: VS Code's LLM diagram prompt only ever sees event
names (no branches, no procedure bodies), and JetBrains' action produces a
non-Mermaid `.mmd` file. A user going through the MCP tool (Zed, or `tcl-mcp`
directly) gets the real, richly-structured diagram; a user going through the
native LSP command in VS Code/JetBrains does not — with no error, so the gap
is invisible until someone inspects the output.

**What cleanup looks like:** Replace `diagram_data_command`'s body with a
call into `tcl_diagram::diagram_data_for_dialect` (parse the source, resolve
its dialect/registry the same way the compiler-explorer/diagnostics command
does elsewhere in `tcl-lsp-server`, add the `tcl-diagram` crate dependency to
`tcl-lsp-server`/`tcl-lsp-core`), then extend the e2e test to assert
`procedures`, `flow`, `priority`, and `multiplicity` are present and
non-trivial for a small fixture with both an event and a procedure — so this
class of shape regression fails a test instead of shipping silently.

**Scale:** ~10-line function replaced by a call to an existing crate; one
`Cargo.toml` dependency line; one e2e test extended. Small, contained fix
with an outsized correctness payoff (two editors' diagram feature currently
under-delivers or is broken).

---

## F2: The canonical dialect list is hand-copied into 8+ editor files with no drift gate, and has already gone stale in the Sublime README

**Confidence:** medium-high
**Category:** ungated-duplication

**Authoritative source:** `rust/tcl-dialect/src/profile.rs:259-816` —
`static CATALOG: [DialectProfile; 16]`, one entry per dialect (`name: "..."`
at lines 266 `bpf`, 286 `cadence-eda-tcl`, 346 `expect`, 376 `f5-bigip`, 403
`f5-iapps`, 437 `f5-irules`, 467 `f5-tmsh`, 491 `intel-quartus-eda-tcl`, 552
`mentor-eda-tcl`, 596 `synopsys-eda-tcl`, 654 `tcl8.4`, 682 `tcl8.5`, 706
`tcl8.6`, 730 `tcl9.0`, 756 `tcl9.1`, 780 `xilinx-eda-tcl`).

> **Correction (review of PR #1401).** This report originally claimed only 15
> of the 16 were LSP-selectable, treating `bpf` as CLI-only on the strength of
> its KCS page's `## Applies to: tcl-lsp CLI` line. That was wrong, and the
> server says so directly: `is_known_dialect_name`
> (`rust/tcl-lsp-server/src/lib.rs:20059-20061`) accepts **any** catalogue
> profile for `initializationOptions.folderDialects` and the per-folder
> `tclLsp.dialect`, and its doc comment names the case explicitly — *"including
> the config-only f5-tmsh / f5-bigip / **bpf**, which a bare `DialectSet::parse`
> check wrongly rejects"*. `dialect_from_language_id`
> (`lib.rs:5879`) maps `"tcl-bpf" | "bpf" => "bpf"`. The `## Applies to` line
> scopes the *KCS note*, not `tclLsp.dialect`.
>
> All **16** catalogue entries are therefore LSP-selectable, and the finding is
> larger than first written: `bpf` appears nowhere in
> `editors/vscode/package.json` at all, so it is a supported dialect missing
> from every editor enum — a second live drift alongside the Sublime README
> one below. A generator that excludes "CLI-only" entries would preserve that
> omission rather than detect it; it should emit all 16.

None
of the xtask generators (`gen-editor-settings`, `gen-vscode-package`,
`gen-jetbrains-catalog`, `gen-editor-catalogs`, `gen-tmlanguage-keywords`)
touch a dialect enum — they are all scoped to `DiagCode`/`OptCategory`
tables, event/command catalogues, and TextMate keyword lists (confirmed by
grepping `dialect` across `rust/xtask/src/gen_*.rs`: the only hits are
`TCL_SOURCE_EXTENSIONS`/glob helpers, not the dialect *name* list).

**The duplicate / stale copy:** the dialect list is hand-typed (as 15
entries, omitting `bpf` — see the correction above),
independently, in at least these places:

- `editors/vscode/package.json:3193-3207` (`tclLsp.dialect` setting `enum`)
  **and again** at `editors/vscode/package.json:7935-7949` (a chat-tool
  argument schema's `dialects` enum) — two separate hand-typed copies in the
  same file.
- `editors/jetbrains/.../settings/TclLspSettings.kt:573-590`
  (`DIALECT_OPTIONS`) — outside any `@generated:*` marker block (the file's
  generated regions are `@generated:diagnostic-vars`,
  `@generated:optimiser-vars`, `@generated:diagnostic-map`,
  `@generated:optimiser-map` only, per `gen_jetbrains.rs:215-218,397-399`).
- `editors/sublime-text/plugin.py:52-69` (`DIALECTS`, functional — feeds the
  "Select Dialect" quick panel).
- `editors/sublime-text/README.md:150-163` ("Supported dialects" table).
- `editors/neovim/tcl_lsp.lua:19-21` and `editors/neovim/README.md:122`.
- `editors/helix/README.md:83-86`.
- `editors/emacs/README.md:73`.
- `editors/zed/README.md:151-153`.
- `README.md:865-881,992-1005` (two more copies at the repo root, outside
  the audited `editors/` tree but the same knowledge again).

**Is it gated?** No gate — none of the `xtask-check` generators, and no CI
step, verifies any of these lists against `tcl-dialect`'s `CATALOG`. A new
dialect profile can be added to `profile.rs` and every one of the ~10 copies
above will silently continue advertising the old set.

**Drift evidence:** `editors/sublime-text/README.md:150-163`'s "Supported
dialects" table has already drifted from the canonical set: it lists 11
entries (`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `f5-irules`, `f5-iapps`,
`synopsys-eda-tcl`, `cadence-eda-tcl`, `xilinx-eda-tcl`,
`intel-quartus-eda-tcl`, `mentor-eda-tcl`) and is missing **`tcl9.1`,
`f5-tmsh`, `f5-bigip`, and `expect`** — four dialects the extension actually
supports (confirmed functional in `plugin.py:52-69`'s `DIALECTS`, which *is*
complete). The doc undersells the extension to a Sublime user reading it.

**Why it matters:** a Sublime user reading the README has no way to learn
that `tcl9.1`, `f5-tmsh`, `f5-bigip`, or `expect` dialect selection exists —
a documentation gap that actively misleads, not a typo. More broadly, every
other copy is a maintenance trap: the next new dialect (there is prior art —
`bpf` was added as dialect #16 without any editor file needing to change,
because it's deliberately CLI-only) will need the same manual edit in up to
10 files with nothing to catch a miss, unlike every diagnostic/optimisation/
keyword catalogue in the same files, which the `xtask-check` gate protects.

**What cleanup looks like:** Add a `dialect names` (or fold into
`gen-editor-catalogs`) xtask generator that emits the canonical LSP-facing
dialect list (all `CATALOG` entries except ones marked CLI-only, if such a
flag is added) and patches the `enum` arrays in `package.json` (both
occurrences), the JetBrains `DIALECT_OPTIONS` list (inside a new
`@generated:dialect-options` marker pair), and the Sublime `DIALECTS`
list/README table; wire its `--check` into `xtask-check`. The plain-Markdown
READMEs (Neovim/Helix/Emacs/Zed/root `README.md`) that are prose, not
settings schemas, are lower priority but could at minimum gain a `<!--
dialects:begin/end -->` marker patched the same way, or a
`kcs-index-links`-style check that greps them for the same set.

**Scale:** one new small xtask generator (~50-100 lines, similar shape to
`gen_tmlanguage_keywords.rs`'s multi-file patching) plus a Makefile/CI wire-
up; fixing the immediate Sublime README drift is a 4-line documentation
edit.

---

## Summary

Two findings, both verified on both sides of the duplication:

- **F1 (high confidence, duplicated-tooling):** `tcl-lsp-server`'s
  `tcl-lsp.diagramData` command (`rust/tcl-lsp-server/src/lib.rs:10600`) is a
  private, much shallower reimplementation of `tcl_diagram::diagram_data_for_dialect`
  (`rust/tcl-diagram/src/data.rs:444`), which `tcl-cli` and `tcl-mcp` both
  correctly share. It returns bare event names with no `procedures`/`flow`/
  `priority`/`multiplicity`, breaking VS Code's `/diagram` chat prompt (which
  expects that shape, `editors/vscode/src/chat/commands/diagram.ts:25`) and
  producing a non-Mermaid `.mmd` file from JetBrains' `DiagramDataAction`
  (`editors/jetbrains/.../TclLspActions.kt:86`) — while Zed's MCP-routed
  `diagram` tool gets the correct answer for the identical input. Fix is a
  ~10-line change plus a dependency add.
- **F2 (medium-high confidence, ungated-duplication):** the 16-entry
  LSP-facing dialect list, canonically `rust/tcl-dialect/src/profile.rs`'s
  `CATALOG`, is hand-copied into 10+ files across VS Code, JetBrains, Sublime,
  Neovim, Helix, Emacs, Zed, and the root README, with zero drift-gate
  coverage (unlike every diagnostic/optimisation/keyword catalogue nearby,
  which the `xtask-check` gate protects). It has already drifted: the
  Sublime README's "Supported dialects" table is missing `tcl9.1`,
  `f5-tmsh`, `f5-bigip`, and `expect`, even though the extension's own
  `plugin.py` `DIALECTS` list supports all 15 of the ones it lists. Per the
  correction at the top of F2, the canonical set is 16 — `bpf` is
  LSP-selectable and missing from every editor enum.
