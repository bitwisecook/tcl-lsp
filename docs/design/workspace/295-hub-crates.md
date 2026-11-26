# 295: the hub crates

Status: designed 26 November 2026. Plan item 295. Builds on
[289](289-lsp-core.md), [290](290-lsp-server.md), [293](293-mcp.md) and
[294](294-explorer.md), and on items 266 and 287 (`tcl-compiler-it`). It is
the acceptance record for the [workspace overview](overview.md): each hub is
re-measured when its crates build.

## Problem

A hub is a crate that nearly every worktree compiles, whatever it works on.
There are two kinds.

**Top hubs** sit above almost everything and are built because agents run
them. This design covers these five:

| Crate | Why every worktree builds it |
|---|---|
| `tcl-mcp` | `.mcp.json` starts `scripts/tcl-mcp`, which runs `cargo run --quiet --locked -p tcl-mcp` each time a client starts |
| `tcl-cli` | the compiler-explorer skill runs `cargo run -q -p tcl-cli --bin tcl` (`.claude/skills/compiler-explorer/explore.py:49`); and `tcl-mcp` depends on it |
| `tcl-lsp-server` | the lsp-client skill drives a built server (`lsp_client.py:719`); editors run it |
| `tcl-lsp-core` | it is in the cone of all three, of `xtask` (every `cargo xtask` gate), of `tcl-spectcl` until 288, and of `tcl-lsp-db`, `tcl-cli-support`, `tcl-spec-studio`, `f5-cli` and `f5-xc` |
| `tcl-compiler-it` | created by item 266; every compiler-split worktree runs its tests (item 287) |

**Low hubs** sit below almost everything, so an edit to one rebuilds most of
the workspace. Plan item 295's text names these: `tcl-dialect`,
`tcl-lexer`, `tcl-syntax`, `tcl-core-types` and `tcl-registry`. They are
covered in the last section.

### Measurement

`cargo tree -p X --prefix none | sort -u | wc -l`, the plan's measure, counts
a crate twice when it appears both expanded and as `(*)`, and counts the
`[dev-dependencies]` header lines. The table gives it and the deduplicated
count (`sed 's/ (\*)$//'`, header lines dropped), for all edges and for
normal and build edges only, on 26 November 2026:

| Crate | Plan's measure | Deduplicated, all edges | Deduplicated, normal | Workspace crates (all edges) |
|---|---|---|---|---|
| `tcl-mcp` | 467 | 341 | 341 | 43 |
| `tcl-cli` | 304 | 228 | 227 | 37 |
| `tcl-lsp-server` | 276 | 205 | 202 | 36 |
| `tcl-lsp-core` | 263 | 199 | 134 | 29 |
| `tcl-compiler-it` (not yet created; `tcl-compiler`'s dev cone, which it inherits) | 218 | 159 | — | 29 |
| for reference: `xtask` | 312 | 236 | 236 | 36 |

The workspace has 55 members. `tcl-mcp`'s cone holds 43 of them.

## Design

Each hub stops being one in a different way: by ceasing to exist, by no
longer being depended on, or by its edits rebuilding only itself.

### `tcl-lsp-core`: it ceases to exist

289 replaces it with eleven `tcl-ide-*` crates and two adapters. No consumer
then builds all of them:

| Consumer | Builds today | Builds after |
|---|---|---|
| `xtask` | `tcl-lsp-core` (for `semantic_tokens`, `file_uri`, `source_style`, `source_decode`, profile helpers) | `tcl-ide-tokens`, `tcl-ide-diagnostics`, `tcl-source-store`, `tcl-catalogue` |
| `f5-cli`, `bigip-query-wasm` | `tcl-lsp-core` | `tcl-ide-format`, `tcl-source-store` |
| `f5-xc` | `tcl-lsp-core` | `tcl-ide-diagnostics` |
| `tcl-spec-studio` | `tcl-lsp-core` | `tcl-ide-format`, `tcl-ide-names` |
| `tcl-cli-support` | `tcl-lsp-core` | `tcl-source-store`, `tcl-ide-model` |
| `tcl-mcp` | `tcl-lsp-core` | the ten `tcl-ide-*` crates its 21 modules map to (all but `tcl-ide-tokens`), plus `tcl-catalogue` |

Target: no `tcl-ide-*` crate's normal cone contains `tcl-vm`, `tcl-codegen`,
`tcl-spectcl`, `tcl-bigip`, `tcl-irules`, `tcl-sslictcl` or `salsa`. The
largest, `tcl-ide-edit`, stays below 134 crates (today's `tcl-lsp-core`
normal cone), and an edit to one rank-5 crate rebuilds none of its siblings.

### `tcl-mcp`: it depends on libraries only

293 removes the two CLI edges (projected cone 289). After 289 it names the
feature crates it uses rather than all of `tcl-lsp-core`. Nothing depends on
it, so its edits already rebuild only itself; what changes is how much of
the workspace a worktree compiles just to start the MCP server.

It still links `tcl-spectcl` (and so the VM) for its pack tools and
`tcl-bigip` for its BIG-IP tools. That is the tool's real surface.

Target: deduplicated cone at most 289 after 293, and no binary crate in it.

### `tcl-cli`: nothing depends on it, and the skill needs less of it

After 293 nothing depends on `tcl-cli`. Its cone stays large because the
`tcl` binary is the whole toolchain: 227 crates, including `tcl-codegen`,
`tcl-explorer`, `tcl-spectcl`, `tcl-spec-studio`, `tcl-pkg` and `rusqlite`.
After 289 and 294 it loses `tcl-lsp-core` (replaced by the crates it names:
`diagnostic_policy`, `formatting`, `minify`, `source_decode`,
`document_symbols`, `variable_symbol`, `procedure_symbol`, `graphs`,
`config_ini`, `check_payload`, `source_style`, `vfs`) and the explorer's
spectcl edge, though it keeps `tcl-spectcl` for its own pack verbs.

The skill is the lever. The compiler-explorer skill runs `tcl explore`, which
needs only `tcl-explorer` and the registry. After 294 the explorer's cone
holds no loader, VM or domain crate, so a small `tcl-explore` binary target
in a crate above `tcl-explorer` (or in `tcl-explorer` itself) gives the
skill the same output for a fraction of the build.

Target: `tcl-cli` has no reverse dependencies (rank gate rule 6), and the
skill builds at most `tcl-explorer`'s cone plus `tcl-catalogue`.

### `tcl-lsp-server`: edits rebuild one handler crate

290 splits it into seven crates. The transport's cone is still the whole LSP
stack, because the server is the product. What changes:

- an edit to a handler rebuilds that crate and the transport, not 70k lines;
- the test features (`truth-table`, `transport-fixtures`) are gone, so
  `cargo build` and `cargo test` share one build of every lower crate;
- `tcl-lsp-commands` alone holds `tcl-explorer`, `tcl-bigip-query` and
  `f5-xc`, so an explorer edit rebuilds the commands crate and the
  transport, not the handlers.

Target: the transport's cone does not exceed today's 202, and an edit to
`navigation/hover.rs`'s successor rebuilds two crates.

### `tcl-compiler-it`: it follows its tests

Item 266 creates it with `tcl-compiler`, `tcl-spectcl` and
`tcl-engine-tclvm` (and through them `tcl-vm` and `tcl-codegen`). Item 287
then points it at the split crates its tests name. The rule here:

- A test that names one split crate and needs nothing above it moves into
  that crate's own tests (`cargo test -p tcl-binding`), so a binding worktree
  never builds `tcl-compiler-it`.
- What stays is the tests that need `tcl-spectcl` or an engine. Their cone is
  bounded by those two crates' cones, which this item does not shrink; the
  overview's open question on `tcl-spectcl`'s VM edge (the pack loader runs
  packs in the VM through `tcl-spec-hooks`) is what would.
- Nothing depends on `tcl-compiler-it`, and it has no `[lib]` consumers.

Target: after 287, `rg -l` over `tcl-compiler-it/tests` finds no test that
names only one split crate, and its cone is the union of `tcl-spectcl`'s and
`tcl-engine-tclvm`'s.

### The low hubs

Direct dependents in the root workspace (normal, from `cargo metadata`), and
manifests that name each crate anywhere in the tree:

| Crate | Direct dependents | Dev | Transitive dependents | Manifests naming it |
|---|---|---|---|---|
| `tcl-dialect` | 40 | 2 | 48 | 44 |
| `tcl-lexer` | 34 | 0 | 44 | 39 |
| `tcl-syntax` | 32 | 0 | 41 | 33 |
| `tcl-core-types` | 27 | 1 | 49 | 30 |
| `tcl-registry` | 29 | 2 | 33 | 36 |

These cannot stop being depended on; they are the vocabulary. They stop being
hubs by stopping changing:

- `tcl-registry`: the registry split (items 261–265) moves per-command data
  into surface crates that only entry points take; items 117 and 118 take
  the native algorithms and BIG-IP data out.
- `tcl-dialect`: its `build.rs` generates a table into `OUT_DIR`, and
  `generated_release_facts.rs` is 11k lines (items 299 and 300). Release
  facts that only some consumers read move to the dialect crates of the
  registry split (`tcl-dialect-tcl`, `-jim`, `-f5`); `tcl-dialect` keeps the
  `DialectProfile` and family vocabulary.
- `tcl-lexer`, `tcl-syntax`, `tcl-core-types`: no per-feature data today.
  The rule is that none gains any; the rank gate's role column marks them
  `vocabulary`, and a new module in one needs a reviewed ledger row.

### Decisions

- **Measure deduplicated.** The plan's command overstates every cone by
  30–37%. Both are recorded so the history stays comparable.
- **Hubs by use, not by fan-in.** The top hubs have few or no dependents, but
  agent tooling builds them in every worktree. Their cost is set by the
  tooling (`.mcp.json`, the skills), so each design names the tool too.

### Open questions

- **Should `scripts/tcl-mcp` prefer a prebuilt binary in worktrees?** It
  already supports `--allow-prebuilt`. Settle by the owner: the trade-off is
  a stale MCP server against a full build per worktree.
- **A `tcl-explore` binary for the skill.** Settle by measuring its cone after
  294; adopt if it is under half of `tcl-cli`'s.

## Work list

1. Add a `scripts/dev/crate-cones.sh` that prints both measures for every
   crate, and record today's table in item 259's baseline.
2. As 288–294 land, re-run it and record each hub's numbers in the step's
   measurement record.
3. After 293: add the rank gate's rule 6 row checks for `tcl-cli` and
   `f5-cli`.
4. After 294: decide the `tcl-explore` binary and, if adopted, point the
   compiler-explorer skill at it.
5. After 287: move single-crate tests out of `tcl-compiler-it`.

## Verification

Done when every target above holds, measured by step 1's script on the tree
where the last of 288–294 builds, and:

- `tcl-lsp-core` does not exist;
- no crate depends on `tcl-cli`, `f5-cli` or `tcl-mcp`;
- a fresh worktree that starts the MCP server and runs the compiler-explorer
  skill compiles fewer crates than today's 341 plus 228 (item 259's script
  records the number and wall time).

## Risks

- **Moving targets.** Other agents' work adds dependencies while the split
  runs. The rank gate stops new upward edges; it does not stop cones from
  growing sideways. Re-measure at each step.
- **Tooling drift.** If a skill starts calling a heavier binary, the hub
  returns. The skills' launch commands are listed in this document so the
  check is one `rg`.

## Corrections to the overview

- Plan item 295's text and CSD's table name only the low hubs. The
  costliest hubs for parallel worktrees are the top ones above, which agent
  tooling builds in every worktree. The plan item should cover both kinds.
- The plan's dependent counts (43, 37, 33, 30, 34) match counting manifests
  across the whole tree (44, 39, 33, 30, 36 today) rather than root-workspace
  dependents (40, 34, 32, 27, 29). The text should say which.
