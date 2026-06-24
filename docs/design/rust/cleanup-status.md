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

## Remaining / discovered work

Ordered by value. Each is in-scope Rust code-quality work (the standalone Python
implementation and Zig remain out of scope).

### 1. `DiagCode` follow-ups (the migration itself is done — see Completed)

Two cheap wins remain now that `DiagCode::ALL` is the single source of truth:

- **Generate `docs/generated/diagnostic_tables.md` from `DiagCode::ALL`** (an
  `xtask`), retiring the hand-maintained table.
- **Add a completeness guard**: a test asserting the emitted/documented code sets
  match `DiagCode::ALL`, so the drift in §2 below can never recur. (Severity and
  one-line descriptions would need to move onto the enum's macro table first, or
  the guard can check the code set only.)

### 2. Diagnostic-code documentation drift (concrete bugs)

The hand-maintained `docs/generated/diagnostic_tables.md` has drifted both ways:

- **Emitted but undocumented (add to the table):** `E004`, `E100`–`E103`,
  `E201`–`E206`, `IRULE3004`, `IRULE3103`, `IRULE5003`, `IRULE6001`, `S110`,
  `T103`–`T106`, `W310`, `W311`, `W312`.
- **Documented but never emitted (orphans — implement or retire):** `W130`–`W134`,
  and `W122` ("Mistyped IPv4 address, octet > 255") which is documented and has
  dedup-suppression handling (`analyser/diagnostics.rs:474`) but is never emitted —
  the octet-range check at `analyser/diagnostics/usage.rs:388` *skips* octets > 255
  rather than flagging them. Decide whether W122 is a missing check to implement
  or stale handling to remove.

Fixing #1's completeness guard makes this class of drift impossible going forward.

### 3. Investigated and rejected — do not re-open

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

### 4. Smaller / parked items

- **GVN `ExprKey = Vec<String>` interning** — the last named O11 slice, deferred:
  contained to `gvn.rs` but `ExprKey` escapes through result structs to consumers.
- **`tcl-lsp-server`/`tcl-lsp-db` god-files** — `lib.rs` monoliths could be split
  (`convert.rs`/`commands.rs`/`config.rs`), lower priority than the above.
- The two `Diagnostic` structs (`analyser::types` vs `compiler_checks`) are
  deliberately distinct envelopes for different pipeline stages — **not** a
  unify-me duplicate; leave as-is.

### 5. Other stringly-typed vocabularies — apply the `DiagCode` pattern

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
