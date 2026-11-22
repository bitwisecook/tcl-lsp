# Editor actions and their tool equivalents

**Question:** will rename and the other LSP actions (and the MCP and CLI tools that do the same jobs) run efficiently under the design, and has the architecture taken them into account?

**Method:**
- A release `tcl-lsp-server`, built from `204509ef`, was driven over JSON-RPC with the repository's `lsp-client` skill on three workspaces:
  - A: about 10,500 lines, five Tcl library files plus a two-file pair;
  - B: the two-file pair alone;
  - C: one six-line file.
- The core functions were timed in-process at opt-level 2.
- The code paths were read: the server's rename tiers, the workspace index and scan, the fresh-analysis cache, and the MCP tools.

Scripts, fixtures and raw results are in [`editor-actions/`](handover/oo-dispatch-probe/editor-actions/).

## Verdict

**No, on both counts.**

- **The actions do not work well today.** Rename succeeds only when the file is alone in the workspace. Any `source` line, literal or computed, stops navigation within the file. Requests in a 10,000-line workspace take 1.5–22 seconds.
- **The designs do not account for them.** No design page sets latency targets, says which facts each action needs, separates fast point queries from workspace-wide ones, covers `prepareRename`, cancellation or progress, or makes the tools behave like the editor. The caching page mentions "latency budgets in CI" in one line, and the reverse index (plan item 67) and per-body queries (item 95) are necessary but not enough.

## Measured

| Case | Result |
| --- | --- |
| C: one six-line file | rename and `prepareRename` work for a procedure and a local variable in 2–12 ms; definition 1–10 ms; references 63–79 ms |
| B: the same procedure with `lib/util.tcl` also in the workspace | rename refused for the procedure ("a completed original command world is unavailable for collision coverage") and for a local variable ("cannot account for every original variable name, alias target and read in this document"); `prepareRename` returns `null` with no reason |
| B, with `source [file join [file dirname [info script]] lib util.tcl]` as line 1 | every `prepareRename`, rename and definition in the file returns `null`, including a call to a procedure defined three lines above; references still find the cross-file declaration |
| B, with `source lib/util.tcl` | definition of the same-file call still returns nothing |
| A: workspace scan | 58 s for 7 files (10,513 lines) |
| A: opening `http.tcl` (5,451 lines) | 28 s to diagnostics |
| A: requests in the 15-line `main.tcl` | `prepareRename` 20 s and rename 23 s on a cross-file call; references 19 s on a local procedure |
| A: requests in `http.tcl` | `prepareRename` 1.5–4 s; rename 1.7 s; definition 1.6 s; references 21–22 s; all renames return nothing |
| A: after a one-line edit to `http.tcl` | rename 13 s (a full re-analysis); references 22 s |
| In process, `http.tcl` | analysis 13.2 s; full re-analysis after a one-line edit 12.5 s; after analysis, rename 1.2–1.9 s, references 0.8–1.5 s, definition 2.5 s, hover 3.4 s |

**Why:**
- **Whole-file granularity.** The server's `FreshAnalysisCache` keys one analysis on the document's complete input, so any edit re-analyses the whole file. `prepareRename`, rename, definition and references all wait for that analysis first.
- **Requests that grow with the file size.** After analysis, point requests still take seconds on a 5,000-line file, so the positioned queries walk or rebuild rather than look up.
- **Requests that grow with the workspace.** The same request on the same 15-line file takes about 100 ms with one other file in the workspace and about 20 s with six. Rename also rescans the workspace folders when the source inventory looks stale (`refresh_stale_complete_host_inventory_for_rename`).
- **All-or-nothing coverage.** Rename's safety gates demand a completed command world and complete variable coverage for the *whole* document, and the cross-document tier for the *whole* workspace. An unrelated file, or one `source` whose effects are not modelled, refuses everything. That is the "one unknown resets the world" fault ([cross-cutting review](cross-cutting-review.md), fault 3), applied to edits.
- **`source` is a barrier, not an import.** Neither the computed-path idiom nor a literal path brings the sourced file's declarations into the name world, and the unknown effect suppresses even same-file navigation.
- **The tools are a different product.**
  - MCP `rename`, `find_references` and the refactor tools take one source text, run a fresh analysis per call (`tcl-mcp/src/tools.rs` `analyse`), see no workspace, and call `tcl_lsp_core::rename` directly. They skip the server's roughly 2,600 lines of rename tiers (cross-document, namespace, method family).
  - Calling the core function correctly needs inputs that match exactly. In-process calls with and without the pack overlay got different refusals ("source analysis does not match the current document and input"), so each caller must rebuild the server's inputs by hand.
- **Refusals without reasons.** `prepareRename` returns `null` where rename returns an error with a reason, so the editor says "cannot rename" with no explanation.

## What each action needs

| Action class | Examples | Facts needed | Target, warm |
| --- | --- | --- | --- |
| Point query | hover, definition, `prepareRename`, signature help, highlight | the annotated command at the cursor (binding tier) and the target's declaration | ≤ 50 ms at p95, independent of file size |
| Collection query | references, rename, call hierarchy, code-lens counts, workspace symbols | reverse index entries for one identity, the files that can see it, and its potential-execution coverage | ≤ 300 ms for a 10,000-line workspace |
| Local transform | extract or inline variable, `if` to `switch`, `switch` to `dict`, `brace_expr` | per-body binding, places and def-use, and edit permission | ≤ 100 ms |
| Whole-program transform | optimise, minify, format | the full per-file pipeline | in the background, with progress |
| After an edit | any of the above | re-binding of only the edited body unit | point query ≤ 100 ms after a one-line edit |

Only whole-program transforms need SSA and SCCP over everything. Point and collection queries need the binding tier (the annotated command, the name world, places), plus the binding and value loop only for sites whose heads or names are dynamic.

## Design

1. **Separate tiers per body unit.** Binding facts (annotated commands, the name world, places) and per-file exports are separate salsa queries from SSA, SCCP and diagnostics. Point and collection queries read the binding tier only. A one-line edit re-binds one body unit. This extends plan items 66, 73 and 95 with an explicit rule: no editor request waits for the full pipeline.
2. **Point lookups by position.** Each body's annotated commands are held in an interval index, so a point query costs O(log n) in the file size. No positioned query walks the source ([architecture review](architecture-review.md), item 97).
3. **A persistent workspace tier.**
   - Each file's declarations, exports and reverse-index contributions come from the binding tier, never from the full pipeline. They are cached on disk by content hash, dialect and pack key, like installed packages.
   - The cold scan builds only this tier.
   - Full analysis runs only for open documents and on demand.
   - Requests never re-analyse other files.
   - An inventory refresh is incremental per file, not a rescan.
4. **`source` as an import edge.** A `source` whose path the value kernel can evaluate brings the sourced file's exports into the name world at that point. That covers a literal path and the `[file join [file dirname [info script]] …]` idiom, with `info script` known for each file. A path it cannot evaluate adds per-name unknowns for what an unknown file could define. It never suppresses navigation or refuses rename for the rest of the file.
5. **Scoped completeness.**
   - Rename coverage and collision checks cover only the files and bodies that can see the identity (from the reverse index and exports), and only the names that could collide in the scopes the edit touches.
   - Unknown code makes the result incomplete only where it could reach that identity, as computed by the potential-execution owner ([design integration](design-integration.md), decision 8).
   - An unrelated file or unknown effect never refuses a rename.
6. **Refusals with reasons, early.** `prepareRename` runs the same gates as rename at point-query cost and returns an error with the reason, as the LSP allows, instead of `null`. Every refusal reaches the editor, the MCP result and the CLI ([cross-cutting review](cross-cutting-review.md), fault 11).
7. **One action engine for every surface.**
   - Rename, references, refactors and their checks live in one engine over a session: a workspace root plus overlays for unsaved or supplied text.
   - The LSP server, MCP and the CLI call it. MCP keeps a session per workspace instead of a fresh analysis per call; for single-text calls it creates a one-file session.
   - The server's rename tiers move into the engine.
   - Callers never rebuild analysis inputs by hand.
   - The agreement harness checks that the surfaces return identical edits or refusals.
8. **Cancellation, progress and snapshots.**
   - Salsa cancellation for requests superseded by edits.
   - `workDoneProgress` for workspace-wide renames and scans.
   - The existing workspace source-plan token keeps multi-file edits on one snapshot.
9. **Checks after the edit.** After computing a rename, the engine re-binds the affected body units with the edits applied and checks that every edited site resolves to the renamed identity and nothing else changed target. With per-body queries this is cheap enough to run on every rename; the tclsh execution check stays a test.
10. **Budgets as gates.** The workspaces and scripts in [`editor-actions/`](handover/oo-dispatch-probe/editor-actions/) become a benchmark in the analysis-performance suite, with the targets above per action class and file size, checked in CI on release builds.

## Plan

Items 100–106 in [plan.md](plan.md), scheduled with phases 3 (point and collection queries on the binding tier) and 6 (the action engine on the semantic model). The precision fixes (`source` as an import, scoped completeness, refusal reasons) are correctness work for the current release scope.
