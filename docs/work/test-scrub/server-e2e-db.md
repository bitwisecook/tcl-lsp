# server-e2e-db scrub

Triage of the failing tests of `tcl-lsp-server` (end to end and library) and `tcl-lsp-db`, 17 November 2026. No test was edited or deleted. Source: `lists/server-e2e-db-failing-2026-11-17.txt` (430 failing: 254 e2e, 104 server lib, 72 db).

- Ledger rows (mine): **216** in `ledger/server-e2e-db.tsv`.
- Routed away, not mine: **214** in `ledger/server-e2e-db-routed.tsv` (navigation groups to lsp-features, db memo/incremental to analysis-performance).

## Priority finding: "open document never publishes" (coordinator item 2)

Reproduced on 17 November, run alone with `--test-threads 1`: 7 failed, 17 passed. **It is the server, not the harness, and it is not a total publication failure.** `await_diagnostics_settled` correctly waits for the latest `publishDiagnostics` of the URI to satisfy the test's predicate; every failing document did receive publications:

- three sourced-file tests (`callback_from_a_sourced_file…`, `caller_in_a_sourcing_file…`, `sourced_child_package_edits…`): last publish `[]`, or `I230` kept where the sourced file's call evidence should clear it. Cross-file call-site evidence does not reach the sourcing file. Fifteen sibling issue1331 tests pass, so publication and cross-file W123/arity work; the missing piece is call-site evidence from a sourced file (`project_call_inputs`). The db lib test `cross_file_evidence_sync_settles_after_one_pass` is the lower reproduction.
- three iRule tests (`braced_variable_wrapped`, `t101_wrap_strip_crlf`, `t103_no_proc_insert…`): only fast-tier codes (IRULE1001, IRULE3102, W214) are published; the taint code never arrives. The t101 fixture is `puts $raw`, and `puts` draws W123 in the iRules dialect, so the sink is unknown: a dialect or fixture question for the taint/iRules sessions.
- two spec-pack tests (`the_bundled_upf_body_resolver…`, `an_open_pack_keeps_its_notices…`): published `[]`.

These rows are re-caused in the ledger (`cross-file-source-evidence-not-delivered`, `irule-taint-code-not-published`, `spec-pack-body-notices-not-published`).

## What the e2e tests hold over their lower layer

- Over-the-wire only (keep): publication ordering and settling, codeAction request/response shape and tier, completion item shape, lens/command shape, spec-pack lifecycle, recovery symbols over the wire, Jim hint-only documents.
- Same fact as a lower test (MERGE, survivor named in the ledger): 11 rows; the survivor must assert code, range and message before the e2e row is deleted.
- No lower test exists (the e2e test is the only holder, so REWRITE into `tcl-lsp-core` rather than delete): `$`-completion for `uplevel`, `try ... on error` bindings and command-form locals (`tcl-lsp-core/src/completion/original_variables.rs` is the only file naming `upvar`/`dict with`).
- A contract row cites the contract by file; passage-level citations are still to be added.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| KEEP-CONTRACT | 163 |
| KEEP-ORACLE | 29 |
| MERGE | 10 |
| KEEP-F5 | 10 |
| REWRITE | 2 |
| FIX-FIXTURE | 1 |
| UNDECIDED | 1 |

`MERGE` rows name a survivor whose assertions and fixture were read against the e2e test; `KEEP-CONTRACT` rows cite contract file and section; `KEEP-ORACLE` rows carry the tclsh releases run (8.4.20 to 9.1.0, `tmp/tcl*/unix/tclsh`, tcllib on `TCLLIBPATH` for snit/report). Pending oracles: `needs-itcl-oracle` (1: no Itcl build under `tmp/tcl*`), `needs-jim-oracle` (1: no `jimsh`). One `UNDECIDED` (`tracked_locks`), one `FIX-FIXTURE` (`w210_still_fires_for_a_genuinely_dynamic_condition_call_923_idx122`: `doesNothing` must return a true value).

## Causes ranked by tests blocked

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `source-view-graph-rehoming` | 28 | KEEP-CONTRACT 28 |
| `original-world-seed-and-publication` | 21 | KEEP-CONTRACT 18, REWRITE 2, UNDECIDED 1 |
| `quick-fix-diagnostic-or-action-absent` | 19 | KEEP-CONTRACT 17, MERGE 2 |
| `flow-diagnostics-w210-w220-parity` | 17 | KEEP-ORACLE 11, KEEP-CONTRACT 3, FIX-FIXTURE 1, MERGE 1, KEEP-F5 1 |
| `command-resolution-w123-parity` | 16 | KEEP-ORACLE 7, KEEP-CONTRACT 6, MERGE 3 |
| `value-transfer-memo-parity` | 11 | KEEP-CONTRACT 11 |
| `dollar-completion-variable-scope` | 10 | KEEP-CONTRACT 8, KEEP-ORACLE 2 |
| `logical-source-arity-cross-file` | 10 | KEEP-CONTRACT 10 |
| `project-token-input-split` | 9 | KEEP-CONTRACT 9 |
| `shimmer-s10x-not-reported` | 9 | KEEP-CONTRACT 6, MERGE 3 |
| `class-factory-fixpoint` | 7 | KEEP-CONTRACT 7 |
| `editor-feature-wire-shape` | 6 | KEEP-CONTRACT 6 |
| `source-view-graph-parity` | 6 | KEEP-CONTRACT 6 |
| `dialect-gating-absent` | 4 | KEEP-F5 4 |
| `display-consumer-fallback` | 4 | KEEP-CONTRACT 3, MERGE 1 |
| `irule-taint-code-not-published` | 4 | KEEP-CONTRACT 4 |
| `rewrite-wire-standalone-unit-o127` | 4 | KEEP-CONTRACT 4 |
| `spec-pack-lifecycle` | 4 | KEEP-CONTRACT 4 |
| `irule-s110-collect-bootstrap` | 3 | KEEP-F5 3 |
| `method-body-and-trace-arity` | 3 | KEEP-ORACLE 3 |
| `cross-file-source-evidence-not-delivered` | 3 | KEEP-CONTRACT 3 |
| `deep-nesting-no-publication` | 3 | KEEP-CONTRACT 3 |
| `release-gated-display` | 3 | KEEP-ORACLE 3 |
| `spec-pack-body-notices-not-published` | 2 | KEEP-CONTRACT 2 |
| `project-input-roles` | 2 | KEEP-CONTRACT 2 |
| `recovered-tail-rules` | 2 | KEEP-CONTRACT 2 |
| `autoload-w123-832` | 1 | KEEP-ORACLE 1 |
| `irules-event-completion-order` | 1 | KEEP-F5 1 |
| `irules-octal-literal` | 1 | KEEP-F5 1 |
| `needs-jim-oracle` | 1 | KEEP-ORACLE 1 |
| `needs-itcl-oracle` | 1 | KEEP-ORACLE 1 |
| `method-suggestion-mro` | 1 | KEEP-CONTRACT 1 |

## Distinct failing behaviours (what is worth fixing)

- **source-view-graph-rehoming** (28): Source sites sourced from another file rehome declarations/callers; rename of a sourced pair, autoload merge and release selection reach the library. e2e adds over its lower layer: no: lib tests are the lower layer; e2e naming_* tests add wire shape only.
- **original-world-seed-and-publication** (21): Original-world seeds, publication gating and pack roles survive edits and refusal. e2e adds over its lower layer: n/a (lib layer).
- **quick-fix-diagnostic-or-action-absent** (19): Quick-fix/refactor actions are offered from the published diagnostic (W201/W216/S100/T102, IRULE3001/3002/6001, T101/T103/T106). e2e adds over its lower layer: yes: the e2e test is the only one that drives codeAction from a published diagnostic (wire shape); the diagnostic itself duplicates compiler tests.
- **flow-diagnostics-w210-w220-parity** (17): W210/W220/I230 read-before-set and dead-store parity with tclsh frames. e2e adds over its lower layer: mostly MERGE candidates into the compiler tests once their survivor is named; not decided here.
- **command-resolution-w123-parity** (16): W123 / library, autoload, load-only package and namespace import resolution. e2e adds over its lower layer: e2e adds publication across open/closed files.
- **open-document-never-publishes-in-e2e** (12): A document opened over the wire publishes diagnostics within the barrier. e2e adds over its lower layer: yes: publication ordering and cancellation is e2e-only.
- **value-transfer-memo-parity** (11): Memoised and direct lattices agree. e2e adds over its lower layer: db layer is the only holder.
- **dollar-completion-variable-scope** (10): $-completion offers variables by frame (dict keys, try bindings, upvar aliases, uplevel 0/1, command forms, named-object methods). e2e adds over its lower layer: e2e adds wire shape; lower tcl-lsp-core completion tests hold the frame logic.
- **logical-source-arity-cross-file** (10): Cross-file arity and callback arity on the Logical branch. e2e adds over its lower layer: lower layer already holds the logic; these are db lib tests.
- **project-token-input-split** (9): Project token inputs: class cards, shadowing, roles. e2e adds over its lower layer: db layer is the only holder.
- **shimmer-s10x-not-reported** (9): S100/S101/S102 shimmer diagnostics reach the editor. e2e adds over its lower layer: e2e adds publication only.
- **class-factory-fixpoint** (7): Metaclass/factory chains converge across files. e2e adds over its lower layer: db layer; e2e issue1296 tests are routed navigation.
- **editor-feature-wire-shape** (6): Code lens counts, command wire. e2e adds over its lower layer: e2e is the wire shape.
- **source-view-graph-parity** (6): Source-view inventory and owner controls. e2e adds over its lower layer: e2e only.
- **dialect-gating-absent** (4): iApps/tmsh 8.4.6 trunk gating and iRules banned commands. e2e adds over its lower layer: e2e is the only holder of wire shape.
- **display-consumer-fallback** (4): Hover, document symbols and token fallbacks. e2e adds over its lower layer: lower lsp-features tests hold logic.
- **method-body-and-trace-arity** (4): itcl/snit method bodies and trace callback arity. e2e adds over its lower layer: e2e adds publication.
- **rewrite-wire-standalone-unit-o127** (4): Rewrite proposals carry source association. e2e adds over its lower layer: lib only.
- **spec-pack-lifecycle** (4): Spec pack reload, claim, lifecycle. e2e adds over its lower layer: e2e only.
- **irule-s110-collect-bootstrap** (3): S110 round trip with collect bootstrap. e2e adds over its lower layer: e2e only.
- **release-gated-display** (3): callback/link/my hover and unknown by release. e2e adds over its lower layer: e2e adds wire.
- **project-input-roles** (2): Supplied project inputs withdraw/keep roles. e2e adds over its lower layer: db only.
- **recovered-tail-rules** (2): Recovery tail symbols/clauses. e2e adds over its lower layer: e2e adds symbols over the wire.
- **autoload-w123-832** (1): Autoloaded library command is not W123 (issue 832). e2e adds over its lower layer: lib vs e2e command_resolution overlap.
- **irules-event-completion-order** (1): Valid event commands rank first in completion. e2e adds over its lower layer: e2e.
- **irules-octal-literal** (1): iRules 010 folds as octal. e2e adds over its lower layer: e2e.
- **jim-hint-only-document** (1): Jim documents draw hints only. e2e adds over its lower layer: e2e.
- **method-suggestion-mro** (1): W308 suggestion from MRO. e2e adds over its lower layer: e2e.

## Routed away

- lsp-features navigation: 131
- lsp-features navigation (lib): 50
- analysis-performance memo/incremental (hand-over: not mine): 33

## Exact-copy and overlap groups touching these crates (passing included)

Source: `data/clusters_*.tsv`; 58 tests of `tcl-lsp-server`/`tcl-lsp-db` sit in overlap groups, 2 in exact groups. The pairs are listed in `ledger/server-e2e-db-clusters.tsv`; a MERGE needs the survivor to assert at least the same.

