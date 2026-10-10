# naming.compiler.original-summary-dependency-source-heads

Kind: `implementation-contract`

## Problem statement

A byte scan of command-substitution heads can miss quoted, braced, escaped or Unicode source values. A summary memo can then omit a callee dependency even when that callee source changes; dynamic or incomplete source must preserve an unresolved dependency residual.

## Question

How do summary memo dependencies retain complete original static head values under selected source grammar while keeping unresolved dependencies conservative?

## Conclusion

command_subst_callees_with_config parses complete original substitution command words under the supplied LexerConfig and obtains static head bytes through the shared original word-value owner. Static names contribute finite known summary dependencies; incomplete, dynamic, unresolved or malformed candidates retain every known summary. The default command_subst_callees sibling is explicitly standalone. Database summary_deps_key passes the actual FunctionUnit source configuration into the collector and the tracked proc_summary_cascade memo. Dependency overapproximation supplies no selected invocation or execution edge.

## Scope

Three marked software controls cover quoted, braced, escaped and Unicode static heads, relative source-name resolution, multiple commands, an empty substitution, dynamic/unresolved/incomplete candidates and C8.4 versus C8.6 escape grammar. Brackets in comments, escaped text or literal data may conservatively add dependencies. A tracked Database document fixture changes only the callee source, checks unchanged caller source, two dependent summary recomputations and zero unchanged replays, then compares against a fresh database. These are lexical source-dependency and memo-currency definitions, not observed Rust passes. All seven external providers are not tested. Dependency names establish no runtime command identity, reached handler, receiver, Normal completion or Native frame/contents.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-compiler-original-summary-dependency-source-heads-rust-tcl-compiler-src-taint_interproc.rs` (implementation): [rust/tcl-compiler/src/taint_interproc.rs](../../../../rust/tcl-compiler/src/taint_interproc.rs). SHA-256 `28c35255b1e332ce1b286316ac5985ef9615074c8b0931dd67c487a90262af73`. Current source/API owner and software assertion definition; no executed Rust or native provider result.
- `naming-compiler-original-summary-dependency-source-heads-rust-tcl-lsp-db-src-lib.rs` (implementation): [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs). SHA-256 `90cbc64251c19d3621c67a1c9c26fed364496e128d86b455acb4238d5741b0ac`. Current source/API owner and software assertion definition; no executed Rust or native provider result.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/taint_interproc.rs](../../../../rust/tcl-compiler/src/taint_interproc.rs), `command_subst_callees_with_config`: Retain original complete static substitution head values under actual source grammar, with all-known conservative dependencies for unresolved source; no selected call edge.
- [rust/tcl-compiler/src/taint_interproc.rs](../../../../rust/tcl-compiler/src/taint_interproc.rs), `command_subst_callees`: Provide the explicitly requested default standalone grammar sibling independently of supplied source configuration.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `summary_deps_key`: Retain actual FunctionUnit source lexer configuration and known callee summaries in the tracked dependency key.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `proc_summary_cascade`: Consume the genuine tracked source-summary dependency key; source memo currency supplies no entered runtime call.
- [rust/tcl-compiler/src/taint_interproc.rs](../../../../rust/tcl-compiler/src/taint_interproc.rs), `taint_interproc::tests::summary_dependencies_read_original_static_heads_under_source_grammar` (linked): Whole selected source words retain quoted/braced/escaped/Unicode and relative static heads. Multiple original commands add finite dependencies; an empty substitution adds none. The dependency set is lexical memo metadata, not selected execution.
- [rust/tcl-compiler/src/taint_interproc.rs](../../../../rust/tcl-compiler/src/taint_interproc.rs), `taint_interproc::tests::summary_dependencies_keep_unresolved_and_dialect_residuals` (linked): Dynamic, unresolved or incomplete candidates conservatively retain all known summary dependencies. Actual C8.4 versus C8.6 source escape grammar distinguishes an original static head from an unresolved residual without a runtime alias/command conclusion.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::summary_memo_observes_callee_edits_through_original_quoted_and_braced_heads` (linked): An actual SourceFile and tracked proc_summary_cascade query retain quoted/braced callee dependencies. Unchanged requests execute zero new memo queries; a callee-only edit changes dependencies and recomputes both callers, matching a fresh database with caller source unchanged. This software definition supplies no runtime invocation or observed native result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Current source/API owners and exact marked software control definitions only. No command is run by this publication; actual executable receipts and native provider observations retain independent scopes.
