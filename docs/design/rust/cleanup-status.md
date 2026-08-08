# Rust workspace cleanup — status & remaining work

Living tracker for the code-quality cleanup driven by the 2026-06-2x review docs
in this directory (`architecture-and-quality`, `coherence-and-coverage`,
`workspace-deep-review`, `production-readiness`, `lsp-server-deep-review`,
`review-findings`, `srv-incremental-review`). Those remain as historical
analyses; this file is the go-forward summary — completed findings are recorded
here briefly and dropped from active tracking, and newly-discovered work is added
at the bottom.

## Completed

Landed and verified green (clippy `-D warnings`, fmt, workspace tests). The
review docs predate most of this, so many findings they list as "remaining" are
in fact done — verify against current code before re-opening any of them.

- **Diagnostics god-file split (O19)** — `analyser/diagnostics.rs` split into the
  `diagnostics/` family modules (usage, commands, injection, dataflow, dispatch,
  dialect, security, validity, helpers) + a tests module.
- **CFG/SSA interning (O11)** — block names → `BlockId(u32)`, SSA variable names →
  `Symbol(u32)`, both with per-function interners. `cfg2` goldens regenerated for
  the resulting source-ordered SSA version numbering.
- **Position column type-safety** — `Utf16Col`/`Utf16Position` for the LSP UTF-16
  column and `ByteCol`/`SourcePosition` for the byte column. Byte-vs-UTF-16
  column confusion is now a compile error workspace-wide (addresses the recurring
  class behind #537 CR-counting and #670 UTF-16 work).
- **tower-lsp → tower-lsp-server migration (O17)**, type hierarchy wired,
  position-encoding negotiated.
- **Workspace-index read lock instead of deep clone (O12)**; one `LineIndex` per
  request (O15); document outline via the incremental query (O14).
- **FxHashMap/FxHashSet in the hot dedup/fixpoint paths (O10)**.
- **`Severity` consolidated into `tcl-core-types` (O20)**; typed `BigipError`
  (O22); GIL released in PyO3 pure-Rust compute (O25a); `cargo test` + `cargo deny`
  on the gate (O7/O18); GLOSSARY refreshed to the Rust implementation (O26); proc
  inliner exposed-but-unwired clarified (O9).
- **Recursion DoS guards** confirmed present in every user-facing parser
  (`MAX_BODY_DEPTH`, `MAX_EXPR_DEPTH=256`, regex `MAX_PARSE_DEPTH=1000`, structured
  lowering) — the production-readiness doc's "unbounded recursion" findings are
  already addressed.
- **Optimiser O129 trust gate** wired in the production `optimise()` path.
- **Release-only `unused_variable` warnings** in the interproc taint fixpoint
  silenced (`#[cfg_attr(not(debug_assertions), allow(unused_variables))]`).
- **CLI `tcl diag`/`validate` ↔ LSP diagnostic consistency** — the CLI now
  surfaces the compiler-check (shimmer / taint / iRules-flow) diagnostics the LSP
  already showed.
- **`DiagCode` migration** — the ~160 diagnostic codes are now a typed `DiagCode`
  enum in `tcl-core-types` (generated from one macro table, with `as_str`/`Display`/
  `FromStr`/`family()`/`is_optimisation()`/`DiagCode::ALL`). The `code` field on every
  diagnostic-bearing struct (both `Diagnostic`s, `ShimmerWarning`, `TaintWarning`,
  `Optimisation`, `RedundantComputation`, `BoundsFinding`, `IrulesCheckWarning`,
  `PathConcatWarning`, and the registry's `SetterConstraint`) is `DiagCode`; the
  ~800 emit/compare/dispatch sites across 9 crates are migrated. Strings survive
  only at the LSP-wire / JSON / PyO3 / config boundaries. Behaviour is byte-identical
  (`as_str`/`Display` reproduce the prior spellings).
- **`DiagCode` doc metadata + Rust-generated code tables** — each code's
  documentation metadata (section/category, one-line description, default-on flag)
  now lives on the `DiagCode` macro table in `tcl-core-types`, reachable via
  `doc_row()` and the new `DiagSection` / `OptCategory` / `DocRow` types. The
  optimisation `OptCategory` is consolidated here too: the optimiser's
  `profiles.rs` re-exports it and *derives* its code→category map from
  `DiagCode::ALL` rather than keeping a parallel table. The three published tables
  (`docs/generated/diagnostic_tables.md`, `diagnostic_codes.md`,
  `optimisation_codes.md`) are now **generated** from that single source by
  `cargo xtask diag-tables`, and the Python codegen's three doc targets
  (`scripts/codegen/editor_settings.py` + the `.md.j2` templates) were retired in
  favour of it. An xtask `committed_tables_match_generated` test on the required
  `rust-gate` (`cargo test`) fails the build if the committed tables drift from
  what the enum would render, and `cargo xtask diag-tables --check` gives the same
  audit on demand — drift (a code, description, section, or default changed without
  regenerating) can no longer recur. The migration also surfaced and added the 26
  codes the frozen Python-era tables were missing (`E004`, `E100`–`E103`,
  `E201`–`E206`, `S110`, `T103`–`T106`, `TK1001`–`TK1003`, `W310`–`W312`,
  `IRULE3004`/`3103`/`5003`/`6001`), so the catalogue is now complete.

## Remaining / discovered work

Ordered by value. Each is in-scope Rust code-quality work (the standalone Python
implementation remains out of scope).

### 1. Reserved diagnostic codes not yet emitted (implement when support lands)

**Update (issue #1317):** the classification below was corrected. `W122` was
not a stubbed check to finish — its octet-range logic had already moved to
`W124` (which covers both halves of `W122`'s description: octet > 255 *and*
a leading zero), leaving `W122` with no producer and a dead dedup rule
(`analyser/diagnostics.rs`) suppressing a code that could never fire
alongside it. It has been **retired**: the enum variant, the dedup rule, and
its FP regression test are all removed. `W130`–`W134` were correctly
classified — see below, now formalised with a `reserved` flag rather than
being a plain prose note.

`DocRow::Diagnostic` now carries a `reserved: bool` field
(`tcl-core-types/src/diag_code.rs`): a reserved code is documented (a real
section, a real description) but has no producer *yet*, and is excluded
from the generated editor-settings catalogues the same way an `internal`
code is (for the opposite reason — `internal` is always on and never
configurable, `reserved` is never on and would offer a setting that visibly
does nothing) while staying a full member of `DiagCode::ALL` and the
published code tables:

- **`W130`–`W134` (tclpkg).** The `tcl-pkg` crate (lockfile / CAS / installer /
  policy) and the `tcl pkg` CLI verbs exist, and the design docs
  (`tclpkg-architecture.md`, `contracts/tclpkg-lockfile.md`) specify these editor
  diagnostics — but the analyser does not yet surface them. Wire them when it
  gains tclpkg-awareness for `tclpkg.tcl` / `tclpkg.lock`, at which point drop
  `diag_reserved(...)` back to `diag(...)` and they resume being offered as
  ordinary editor settings.

An "every code is emitted" guard is now **added** —
`cargo xtask diag-emission-check` (wired into `make xtask-check`) — since the
`reserved` flag removes the false-positive the old note cited: the guard
skips every `internal` and every `reserved` code, and fails the build if any
other code in `DiagCode::ALL` has no real construction site left anywhere in
`rust/tcl-compiler/src`, `rust/tcl-lsp-core/src`, or `rust/tcl-lsp-server/src`
— the W122 class of bug, and the sibling W308 mislabelling (its registry row
described a dead `subst`-without-`-nocommands` check years after the code
was repurposed to the TclOO unknown-method check), both caught by issue
#1317's sweep.

### 2. Investigated and rejected — do not re-open

- **`ByteOffset` newtype for `Span` offsets.** Measured: a spike newtyping
  `Span::start()/end()` produced 285 compile errors across 44 files in
  `tcl-compiler` alone (~500–700 workspace-wide, ~6–7× the column-newtype effort).
  Value: the offset-vs-column/length confusion it guards against has no bug
  history (the one offset bug on record, `9c54233b`, is an off-by-one a newtype
  does **not** prevent), `Span` is already encapsulated and never exposes a
  column, and even an ergonomic design adds `.get()`/`.as_usize()` noise or needs
  `Add<u32>`/`PartialOrd<u32>` impls that defeat the safety. Not cleaner; not
  worth it. (Contrast the column newtypes, which were low-cost and addressed a
  real recurring bug class.)

### 3. Smaller / parked items

- **GVN `ExprKey = Vec<String>` interning** — the last named O11 slice, deferred:
  contained to `gvn.rs` but `ExprKey` escapes through result structs to consumers.
- **`tcl-lsp-server`/`tcl-lsp-db` god-files** — `lib.rs` monoliths could be split
  (`convert.rs`/`commands.rs`/`config.rs`), lower priority than the above.
- The two `Diagnostic` structs (`analyser::types` vs `compiler_checks`) are
  deliberately distinct envelopes for different pipeline stages — **not** a
  unify-me duplicate; leave as-is.

### 4. Other stringly-typed vocabularies — apply the `DiagCode` pattern

Surveyed for closed string vocabularies that are matched/dispatched on and would
be safer as a typed enum/newtype. The core compiler is already well-typed
(`TokenType`, `TaintColour`, `OptCategory`, `BlockId`, `Symbol`, `Severity`); the
stringly patterns cluster in dialect threading, CLI validation, and output
formatting. Ordered contained-and-cheap first; the big one last.

- **Taint basis names** (S, medium) — `taint_interproc.rs:41` `BASIS_ORDER:
  [&str; 15]` (`"generic"`, `"path"`, `"non_dash"`, `"crlf_free"`, `"shell_atom"`,
  `"list_canonical"`, `"regex_literal"`, `"path_normalised"`, `"path_bounded"`,
  `"header_token_safe"`, `"html_escaped"`, `"url_encoded"`, `"ip"`, `"port"`,
  `"fqdn"`) matched in `basis_lattice(basis: &str)`. Self-contained to one file; a
  `TaintBasis` enum with a `lattice()` method removes the `_ => generic` silent
  fallback. Best first win.
- **`tcl-bigip` graph formats / grep directions / lint category+severity**
  (S each, low–medium) — `graph.rs:957` `GRAPH_FORMATS = ["dot","json","mermaid"]`,
  `grep.rs:20` `DIRECTIONS = ["both","forward","reverse"]`,
  `lint.rs:22` `CATEGORIES`/`SEVERITIES` (the `Finding.severity`/`.category` fields
  are `&'static str`). Contained to one crate each; enum + match-dispatch.
- **`f5-cli` tmsh kinds / mermaid directions / `tcl-cli` diff formats**
  (S each, low–medium) — `tmsh.rs:16` `VALID_KINDS` (10 object kinds),
  `bigip-query/renderers/mermaid.rs:28` `DIRECTIONS = ["LR","RL","TB","BT"]`,
  `tcl-cli/commands/diff.rs:40` `["ast","ir","cfg"]`. CLI-input validation; typed
  `clap` value-enums would fold the validation into parsing.
- **Dialect / Tcl version** (L, high — the gorilla) — `dialect: Option<&str>`
  threaded through **~240 sites** across the analyser, compiler, LSP, and CLI
  (`"tcl8.4/8.5/8.6"`, `"tcl9.0"`, `"f5-irules"`, `"f5-bigip"`, …). `tcl-registry`
  already has a typed `DialectSet` (bitflags) to converge on — the compiler/LSP
  side should thread that (or a `Dialect` newtype) instead of raw `&str`, with
  parsing confined to the boundary. **Real bug risk:** `"irules"` is a legacy alias
  for `"f5-irules"` and the two spellings are matched inconsistently
  (`matches!(dialect, Some("f5-irules" | "irules"))` in some places, only
  `"f5-irules"` in others) — exactly the typo/missed-case class an enum closes,
  and dialect silently gates security/taint checks. Large mechanical migration;
  highest value of the group.
