<!-- markdownlint-disable MD013 MD033 -->
# Rust-rewrite compiler-pipeline parity audit

Deep parity review of the **Python compiler pipeline** (lexer, parser, CST,
compiler passes, and every analysis) against the **in-progress Rust rewrite**.
Companion to [`rust-rewrite-registries.md`](../../../rust-rewrite-registries.md),
which covers the command/event/object **registries**; this document covers the
**algorithms, data structures, diagnostics, and heuristics**.

- **Python baseline (source of truth):** working tree on
  `claude/sleepy-goodall-pa351e` (mirrors `origin/main`), packages
  `compiler/…`, `analyser/…`, `shared/…`. Python has since been **fully
  retired** from this repository (see `AGENTS.md`) — the comparison below is a
  historical snapshot, not a live moving target. Treat every row as "how the
  since-removed Python implementation compared to Rust at the time it was
  written", not as a description of a still-existing second implementation.
- **Rust baseline (rewrite):** `rust/tcl-lexer`, `rust/tcl-syntax`,
  `rust/tcl-compiler`, `rust/tcl-bytecode` on the same tree.

> **Snapshot date: 2026-06-19. Last revalidated: 2026-08-04, against `rust`
> tip `efe3dd9566dfc593bf875e95a03d1b55fabbb95c`.** This document is a
> point-in-time audit, not a live backlog — the Rust codebase has moved on
> since 2026-06-19 and several rows below are now stale. The 2026-08-04 pass
> (issue #1189) re-checked the memory-SSA section in full plus every
> cross-cutting `complexity_guarded` reference and confirmed/corrected the
> items marked **RESOLVED (2026-08-04 revalidation)** below, each with a
> current code citation. Rows *not* carrying that marker have **not** been
> re-verified since 2026-06-19 and may equally be stale — before relying on
> any unmarked row, re-check it against current code (or current tests)
> yourself. A gap confirmed still live in this pass has no separate GitHub
> issue unless one is explicitly linked; do not assume the mere presence of a
> gap here means it is tracked or scheduled — file an issue before treating it
> as planned work.

## Scope and method

The Python reference is ~80 K LOC across `compiler/` (30 K) + `compiler/parsing`
(6 K) + `compiler/codegen` (28 K) + `compiler/optimiser` (8 K) +
`compiler/taint` / `compiler/var_escape` (7.5 K) + `analyser/`. The Rust
`tcl-compiler` crate is a 123 K-LOC superset that folds in what Python splits
between `compiler/` and `analyser/`, plus the `tcl-lexer`, `tcl-syntax`, and
`tcl-bytecode` crates.

Each subsystem below was reviewed by reading the actual source on both sides
(not the design docs alone), inventorying every structure / algorithm /
diagnostic-code / heuristic, and classifying each comparison:

| Status | Meaning |
|---|---|
| ✅ parity | Same algorithm and same user-facing output (data-structure spelling may differ). |
| ⚠️ differs-benign | Different data structure or algorithm for idiomatic/performance reasons; output is the same or better. |
| ❗ differs-meaningful | A behavioural divergence that can change a diagnostic, a position, or a transform — needs a decision. |
| ❌ Rust-missing | Python capability with no Rust counterpart. |
| ➕ Rust-ahead | Rust does something Python does not, and it is the same or better. |

The intent of the rewrite is explicit: Rust may pick different
algorithms/representations **as long as the user-facing output is the same or
better**. Divergences are only flagged as problems where Rust produces *worse*
output (a missing diagnostic, a wrong position, a weaker transform, or unsound
reasoning).

> **Cross-cutting context already documented.** Three workspace-wide findings
> from [`review-findings.md`](review-findings.md) underlie several rows below
> and are not re-litigated here: **C1** LSP positions are emitted in byte
> columns rather than UTF-16 (`line_index.rs:92` vs the unused `:134`); **C2**
> no document-version guard; and the **one-tree-rebuilt-every-time** cost (the
> CST is the single parse representation since #538 but is rebuilt, not reused).
> This audit references them where a subsystem amplifies them.

---

## Executive summary

**Verdict.** The Rust rewrite is at **high parity on the core algorithms and the
bulk of the user-facing diagnostic surface**, with a clean set of concentrated,
mostly-known gaps. Of the ~183 observable diagnostic/optimisation codes
(verified: 65 W-codes, 33 IRULE, 31 optimiser `O*`, 12 E, 13 XC, 11 BIGIP, plus
S/T/H/I/TK/IAPP), the great majority emit identically; the misses cluster in a
handful of deferred checks and the F5 dialect subsystems. Where Rust differs by
choice — typed hook-ID enums, deterministic ordering, salsa per-item
incrementality, the `tcl-syntax` convergence crate, hand-rolled byte scanners —
it is the same or better. The genuine regressions are few but real, and several
are soundness issues that should gate the default-off shim flips.

**At full or near-full parity (same-or-better output):**

- **Taint analysis** — every T100–T106 + W201/W313 + IRULE3001–3004/3101/3103
  emits at parity and is wired live; Rust is *ahead* on `::`-qualified URI-split
  detection (§6). The strongest-parity subsystem.
- **Core SSA/CFG** — Cooper-Harvey-Kennedy dominators, Cytron dominance
  frontiers, and **semi-pruned (Briggs) SSA** on both sides, so phi placement is
  output-equivalent; CFG block model, naming, and control-flow lowering match
  (§4).
- **Bytecode foundation** — 155-opcode set, serialised format, jump-shrink
  layout, peephole, and disassembly text are faithful 1:1 ports under a real
  differential harness (§9).
- **The diagnostic per-code surface** — the overwhelming majority of
  E/W/H/I/IRULE codes emit at parity (§8); the expr parser/AST, type/constant
  lattices, interval domain, scan predicate, rendered-properties model, and
  signature scan are all solid (§2, §5, §6).

**Cross-cutting themes (touch several subsystems):**

1. ~~**`complexity_guarded` skip is absent everywhere in Rust**~~ **RESOLVED
   (2026-08-04 revalidation, issue #1189).** `ssa.rs` now carries both halves
   of the guard — `COMPLEXITY_GUARD_BLOCKS` (20,000 blocks, `is_complexity_guarded`)
   and `DEEP_ANALYSIS_BODY_BYTES` (256 KiB, for block-light-but-byte-huge
   bodies) — and `compilation_unit.rs::build_procedure_units` consults both
   before building each proc's `FunctionUnit`: an oversized body gets
   `FunctionUnit::trivial_guarded` (`compilation_unit.rs:618-637`) instead of
   real SSA/taint/SCCP, with `complexity_guarded: true` set on the unit so
   every per-proc consumer (`functions()`, interprocedural summarisation, …)
   filters it out (`compilation_unit.rs:1661,1702,1734`) rather than
   over-optimistically summarising it. Pinned by
   `complexity_guard_skips_oversized_ssa` (`ssa.rs`) and
   `complexity_guard_flags_byte_huge_proc` (`compilation_unit.rs`).
2. **C1 byte-vs-UTF-16 columns** (already in `review-findings.md`) is *amplified*
   by re-derivation: positions resolve at ~29 independent `LineIndex` sites
   (§1, §2).
3. **The WASM-codegen support passes are unported** — `IRInterpBoundary` node +
   `passes/interp_boundaries.py`, `passes/dce.py`, `passes/gvn.py`,
   `source_inliner`, `stdlib_prelude` (§3, §7). Expected while Rust WASM codegen
   is in progress, but a structural divergence to track.
4. **Redundant rebuild** — taint is computed 2–3× and the CompilationUnit is
   built 2–3× per document (§3); a perf cost, not a correctness one.
5. **Deterministic ordering** — Rust sorts diagnostics/dataflow nodes by
   position where Python emits in insertion order. This is a robustness *win*,
   but it means the Python↔Rust differential oracle **must compare diagnostics
   as a set, not an ordered list** — worth confirming (§8).

**Genuine regressions, by kind** (full list in the
[Consolidated gap register](#consolidated-prioritised-gap-register)):

- *Correctness/soundness (should block the relevant shim flips):* optimiser
  **O120** (string-compare rewrite without a numeric-value proof) and **O114**
  (incr idiom without an INT-type proof) drop soundness gates and can
  miscompile; **O108** ADCE is over-aggressive; shimmer severity (S100 emitted
  as Warning, not Information) and the phi-S101/expr-S100 code-string
  mislabels. *(The `${a\}b}` braced-variable lexer scan was already fixed
  and marked resolved in the original 2026-06-19 snapshot — §1 — but this
  bullet was never updated to match. SCCP escaping-var widening, the
  `while 1 {… break}` false-unreachable case, the memory-SSA upvar
  transitive-merge, the `IRUpFrame` clobber, and the `DynamicNameLocal` param
  trait are newly confirmed **RESOLVED** in the 2026-08-04 revalidation; see
  §4-§6 for each's current-code citation.)*
- *Capability gaps (Python feature, no/weak Rust):* the general proc inliner
  (`compiler/inlining/`, ~1900 LOC) is unported; `var_escape` is ported but
  **unwired** and missing the `pure_leaf` family; **snit** OO support is absent;
  the high-impact **IRULE1001** (command-invalid-in-event) and **E001**, **W125**,
  **IRULE5005** are deferred; iRules flow checks IRULE1201/1202/5002/5004 are
  linear MVPs (path-sensitivity + quick-fixes deferred to C44); the entire F5
  TK/BIGIP/IAPP/XC diagnostic set (~30 codes) is Python-only; optimiser **O128**
  and **O130** are unimplemented and **O104**/**O119** are hint-only; bytecode
  statement-position specialisation of `append`/`lappend`/`unset`/`upvar`/
  `global`/`tailcall`/`concat`/`string`/… falls to generic invoke.

**Rust-ahead (kept, same-or-better):** typed `LoweringHookId`/`CodegenHookId`
enums (exhaustive dispatch); per-item incremental analysis (salsa early-cutoff);
the `tcl-syntax` convergence crate (list/glob/number/format/mathfunc shared with
the runtime VM); the `structural_index` `info complete` experiment; determinism
canonicalisation across the optimiser; richer scope-alias detection; full
printf/format constant-folding; `escaping_loop_jumps` recursing into `try`.

The sections below give the full per-subsystem inventory; the gap register at
the end ranks every meaningful divergence with its subsystem and a suggested
disposition.

---

## 1. Lexer & low-level scanning

### Scope
Python: `compiler/parsing/{lexer,expr_lexer,recovering_lexer,recovery,token_positions,token_scanning,subst_nocommands,known_commands,argv}.py`; `shared/{tokens,source_map,ranges,tcl_subst,position,document_buffer}.py`.
Rust: `rust/tcl-lexer/src/{lexer,expr_lexer,tokens,span,line_index,source_map,ranges,structural_index,substitution,lib}.rs`.

Python's `TclLexer.tokenise_all` already dispatches to the Rust lexer via the
`tcl_lsp_rust` wheel when present (`lexer.py:1249`), pinned by a differential
oracle, so the two are wired to agree token-for-token. This review compares the
underlying logic.

### Parity table

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| `TokenType` enum (ESC/STR/CMD/VAR/SEP/EOL/EOF/COMMENT/EXPAND) | `shared/tokens.py:9-20` | `tokens.rs:26-46` | ✅ | All 9 variants both sides; `.name()` maps to the Python names. |
| Token representation | `shared/tokens.py:32-40` | `tokens.rs:128-153` | ⚠️ | Span+content_offset+in_quote vs inline pos+text; output-equivalent via `SourceMap`. |
| SEP/EOL whitespace classification | `lexer.py:296-337` | `lexer.rs:1289-1315` | ✅ | `\r`/VT/FF horizontal, `\n`/`;` EOL, mixed runs. |
| COMMENT scan incl. `\`-newline continuation | `lexer.py:1033-1123` | `lexer.rs:475-505` | ✅ | CRLF handled. |
| Bare-word ESC scan; mid-word `\`-newline break | `lexer.py:798-957` | `lexer.rs:507-570` | ✅ | |
| Braces `{…}` balance/inert-`\`/empty clamp | `lexer.py:626-785` | `lexer.rs:943-1039` | ✅ | Span ends on last inner char; empty `{}` extends to closer (#527). |
| Brackets `[…]` nesting + `${…}` sub-scan | `lexer.py:339-507` | `lexer.rs:1073-1199` | ✅ | |
| Quotes `"…"` → ESC w/ `in_quote` | `lexer.py:798-1031` | `lexer.rs:725-858` | ✅ | |
| `$name`/`$ns::var`/`$arr(idx)` forms | `lexer.py:561-616` | `lexer.rs:633-714` | ✅ | Unicode alnum, `::`, balanced `(…)`. |
| **`${name}` braced-var form** | `lexer.py:525-559` | `lexer.rs` `parse_var` | ✅ | Brace-depth + `\X` scan (FE-LEX, 2026-06-19); 9.0.3 reference. |
| Bare `$` → STR | `lexer.py:618-621` | `lexer.rs:657-663` | ✅ | |
| `{*}` EXPAND zero-width token | `lexer.py:787-819` | `lexer.rs:875-915` | ✅ | |
| Dialect flags (expand 8.5+, iRules `}{`, strict_quoting) | `lexer.py:57-106` | `lexer.rs:149-218` | ⚠️ | ContextVar vs `LexerConfig`; same behaviour. |
| Backslash decode (`\xNN`/`\uNNNN`/`\U…`/octal/line-continuation) | `shared/tcl_subst.py:70-145` | `substitution.rs:34-209` | ✅ | Escape table + 0x10FFFF cap + octal byte-ceiling match. |
| Expr lexer (numbers/ops/funcs/bools/Inf/NaN/word-ops) | `expr_lexer.py:128-422` | `expr_lexer.rs:131-456` | ✅ | Token-for-token incl. unknown flag. |
| LineIndex (`\n`-only, lone CR not a break) | `source_map.py:36-41` | `line_index.rs:59-75` | ✅ | Matches #537. |
| offset→position (byte column) | `source_map.py:58-68` | `line_index.rs:103-114` | ❗ | Both byte columns; Rust lacks UTF-16 lift on the default path (C1). |
| offset↔position UTF-16 | server `_lsp_conv` | `line_index.rs:138-212` | ➕ | Rust has the helpers; **unused** by token resolution. |
| Word-closer accessors (#527 empty word) | `shared/ranges.py:81-149` | `ranges.rs:78-124` | ✅ | |
| Error recovery (E201–E206 heuristics) | `recovery.py` (whole) | `lexer.rs` ghost mechanism only | ❗ | Rust has injection (`with_ghosts`) but **not** the detectors. See Gaps. |
| `structural_index` (script-complete / boundaries / reparse window) | — (prototypes in git history only) | `structural_index.rs` | ➕ | Rust-ahead experiment, not yet load-bearing. |
| `subst -nocommands` compile-time evaluator | `subst_nocommands.py` | — | ❌ | Not ported (lowering helper, not lexer core). |

### Gaps (Rust missing or weaker)

- **`${name}` braced-var parsing — RESOLVED (FE-LEX, 2026-06-19).** Rust
  `parse_var` (`lexer.rs`) now tracks inner-brace depth and consumes `\X` pairs
  per Tcl 9's `Tcl_ParseVarName`, matching Python (`lexer.py`): `${a\}b}` reads
  `a\}b`, `${a{b}c}` reads `a{b}c`. The `structural_index::scan_dollar_brace`
  got the matching scan, so its `Tcl_CommandComplete` faithfulness fuzz still
  agrees with the production lexer. Verified against `tclsh9.0` (9.0.3). NOTE:
  Tcl 8.4/8.5/8.6's `Tcl_ParseVarName` stops at the *first* `}` (no depth, no
  backslash) — the project standardises the `${…}` parse on 9.0.3 across all
  dialects per principle #0 (the Python lexer is test-pinned the same way under
  the default 8.6 dialect). See history.
- **Default position column is a byte offset, not UTF-16 (C1).** The lexer/
  `SourceMap` token-resolution path uses `position_at` (byte column,
  `line_index.rs:103-114`); the UTF-16-correct `position_at_utf16`
  (`line_index.rs:138`) is unused. Python emits a similar byte-ish column but
  converts to UTF-16 in the server layer; the Rust path needs the same lift.
- **Error-recovery heuristics (E201–E206 detectors) are not ported.**
  `recovery.py` is ~1000 lines of detectors (comment/command/brace break for a
  missing `]` → E201, newline heuristic for `"` → E202, de-indent +
  known-command + EXPR-arg heuristics for `}` → E203, the `_bracket_insert_inert`
  veto, dedupe). Rust has only the **injection** half (`ghosts` BTreeMap +
  `with_ghosts`/`ghost_at` + the unconditional ghost-`]` closer). There is no
  Rust `detect_recovery` and `known_commands.py` is unported. **Impact:** the
  Rust crate cannot, standalone, reproduce Python's recovered token stream /
  quick-fixes; it depends on a caller computing insertions.

### Rust-ahead / divergent-by-design
- **`structural_index.rs`** — `script_is_complete` (a `Tcl_CommandComplete`/`info
  complete` port verified against C Tcl 9.0.3), `command_boundaries`,
  `reparse_window`, and bracket/brace/paren structural indexes. No live Python
  equivalent; the de-risking experiment for the recovery port. Documented as not
  yet wired into production.
- **UTF-16 position helpers** (`position_at_utf16`/`offset_at_utf16`) — correct
  and surrogate-pair-aware, ahead of the lexer's own usage.
- **Backslash surrogate handling (benign divergence)** — `\uD800`-style lone
  surrogates map to U+FFFD because Rust `String` can't hold surrogate scalars;
  Python yields WTF-8 lone surrogates. Test-pinned; differs only for a
  pathological escape valid Tcl never emits.

### Algorithmic / data-structure differences (benign)
- Token = 16-byte `Copy` `Span` (Rust) vs inline positions + text (Python); the
  PyO3 bridge rebuilds the native `Token` and remaps the enum by `.type.name`,
  so `tok.type is TokenType.X` identity holds.
- `Iterator<Item=Result<Token,…>>` (Rust) vs `get_token()` loop (Python); same
  trailing-EOL contract. Strict-mode `LexError::SyntaxError` re-raised as
  `TclParseError` at the boundary.
- Expr tokens use inline `(start, end-inclusive)` offsets on both sides (short
  substrings, no SourceMap).

### Open questions for maintainer
1. ~~Is the simplified Rust `${name}` scan a known TODO?~~ **RESOLVED
   (FE-LEX, 2026-06-19):** `parse_var` now does the 9.0.3 brace-depth +
   backslash scan; the project standardises `${…}` on 9.0.3 across dialects
   (verified against `tclsh9.0`; 8.4–8.6 diverge but are intentionally not
   gated, matching the test-pinned Python lexer).
2. Plan for C1: switch the token path to `position_at_utf16`, or guarantee a
   higher-layer lift for the Rust path?
3. Recovery port: is the Python-computes-insertions / Rust-re-lexes-with-ghosts
   division permanent, or is `detect_recovery` slated for a Rust port (it needs
   registry access for `known_commands`)?
4. `structural_index` productionisation gating criterion before it replaces the
   two-pass `recovery.py`?

---

## 2. CST, segmenter & expression parser

### Scope
Python: `compiler/parsing/syntax/{green,red,build,descend,segment}.py`,
`green_tree.py`, `command_segmenter.py`, `incremental.py`, `expr_parser.py`,
`expr_ast.py`. Rust: `rust/tcl-syntax/src/{lib,backslash,list,glob,number,naming,format,expr/*}.rs`,
`rust/tcl-compiler/src/parsing/syntax/{green,red,build,descend,segment,mod}.rs`,
`segmenter.rs`.

### Parity table (selected)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| `TriviaKind` / `SyntaxKind` enums | green.py:38-51 | green.rs:40-59 | ✅ | DOCUMENT/COMMAND/WORD; WS/EOL/COMMENT — complete match. |
| `GreenToken` model + relative offsets | green.py:66-122 | green.rs:134-253 | ➕ | Rust adds `content_offset:u8` (span-based tokens). |
| `full_width` cache | green.py:96-101 | green.rs:167-356 | ✅ | |
| Trivia interning cache | green.py:180-194 | — | ⚠️ | Rust has none; benign (memory only). |
| Structural sharing (Rc/Arc) | trivia-cache + frozen identity | owned `Vec`/`String` | ⚠️ | Neither side truly shares green nodes; "shareable" is aspirational both sides. |
| Red `build_line_starts`/`position_at` | red.py:29-96 | red.rs:36-155 | ✅ | `\n`-only; codepoint (Py) vs byte (Rust) consistent with each lexer. |
| Red `parent` back-pointer | red.py:102,160 | omitted | ⚠️ | Rust drops the unused field. |
| `build_document` algorithm | build.py:35-228 | build.rs:52-380 | ✅ | start-to-start tiling, word-merge, `{*}` quirk, comment accumulation — mirrored. |
| `build` backslash-newline fold | build.py:178-211 | build.rs | ✅ | Reconciled to `build.py` (FE-LEX, 2026-06-19): quoted `\<newline>` is a kept fragment, not folded to trivia. |
| `pop().unwrap()` "fuzz targets" | n/a | build.rs:186,190 | ✅ | Both guarded by preceding `!is_empty()`; cannot panic. |
| `descend_token` / nested-body navigation | descend.py:69-89 | descend.rs:95-121 | ✅ | Both **re-lex** inner text via `build_document` (identical strategy). |
| `green_tree.py` cross-consumer intern scope | green_tree.py:1-372 | — | ❌ | No Rust `TokenRegion`/`Mode`/`NodeKind`/`node_for`; per-descent re-lex. |
| `SegmentedCommand` model | command_segmenter.py:61-96 | segmenter.rs:49-131 | ❗ | Rust missing `subcommand`, `braced_word`, `quoted_word`. |
| Segmenter is a CST view (#538) | yes | yes | ✅ | Both views over the green CST; token loop retired. |
| Recovery (suspicious token + scan) | command_segmenter.py:121-503 | segmenter.rs:371-516 | ✅ | line threshold 3, EOF-reach, known-command scan, absolute offsets. |
| Ghost-token recovery | recovery.py | segmenter.rs:518-610 + build.rs:60-78 | ➕ | Rust integrates `build_document_with_ghosts` + iterative E201 recovery. |
| Subcommand resolution / `TopLevelChunk` tiling | command_segmenter.py:237-398 | — | ❌ | No Rust port (Rust caches at salsa/per-item granularity). |
| Incremental reparse | incremental.py:1-498 | segmenter.rs:285-350 | ❗ | Rust is prefix-only; no chunk model, no braced-body interior splice, no suffix reuse. |
| Expr `BinOp`/`UnaryOp`/node kinds | expr_ast.py:27-184 | ast.rs:29-352 | ✅ | 32 BinOp, 5 UnaryOp, full node set incl. iRules ops. |
| Pratt precedence / binding powers | expr_parser.py:58-124 | parser.rs:37-118 | ✅ | Identical bp table; `**` right-assoc. |
| Parse error → Raw fallback | expr_parser.py:286-322 | parser.rs:333-377 | ✅ | |
| `render_expr` paren insertion | expr_ast.py:389-489 | ast.rs:476-566 | ✅ | |
| `vars_in_expr_node` | expr_ast.py:289-331 | ast.rs:360-471 | ⚠️ | Rust stops at command boundaries; relies on SSA layer to recover nested `[…]` vars. |
| tcl-syntax helpers (list/glob/number/naming/format/backslash) | scattered (`shared/tcl_list.py`, `dialects/tcl/format.py`, `shared/naming.py`, …) | list/glob/number/naming/format/backslash.rs | ➕ | Convergence crate re-derived from C Tcl 9.0; shared with the runtime port. |

### Gaps (Rust missing or weaker)

1. **Incremental reparse is far weaker** (`segmenter.rs:285-350`). Rust reuses
   only the command **prefix** before the edit and re-segments `lo`→EOF.
   Python's `incremental.py` is a chunk-level engine: `infer_edit_range`
   (prefix+suffix diff with line_delta), prefix+shifted-suffix reuse, and
   `_reuse_edit_in_braced_body` (incremental.py:208-341) — a brace-safe
   interior splice that avoids re-lexing a multi-MB `namespace eval { … }` body
   per keystroke (motivated by tcllib's 1.3 MB `filetypes.tcl`). **Impact:** on
   a large single-command file the Rust segmenter re-lexes from the edit to EOF
   per change. *Mitigant:* per-file incrementality is owned separately by salsa
   (`file_analysis_incremental`), so this is a segmenter-layer perf gap, likely
   not an end-to-end correctness gap — needs confirmation it is by-design.
2. **`SegmentedCommand` missing `subcommand`/`braced_word`/`quoted_word`**
   (segmenter.rs:49-76). Python populates these (segment.py:53-54,
   `_resolve_subcommands`) for the formatter/minifier brace/quote decisions and
   the resolved-subcommand tag. `segment.rs` computes the underlying signals and
   discards them. Blocks a clean Rust formatter/minifier port.
3. **No `green_tree.py` intern layer.** Python shares one tokenisation across
   the segmenter, `compiler_checks`, and `var_refs` via a contextvar-scoped
   intern index, so overlapping regions are lexed once. Rust re-lexes per
   descent — a perf gap, not correctness.
4. **Backslash-newline divergence — RESOLVED (FE-LEX, 2026-06-19).**
   `build.rs` no longer folds a quoted-content `\<newline>` ESC into trivia; it
   falls through to the fragment path, matching `build.py`. The frozen oracle
   and its edge-case table moved with it. Verified against `tclsh` 8.6/9.0
   (`puts "\<newline>"` is a one-argument command — the space-valued word is
   kept).
5. **Expr `vars()` shallower on Command/Raw** (ast.rs:436-449). Python recurses
   into command-substitution bodies; Rust stops at the boundary, asserting the
   SSA layer recovers them — end-to-end soundness depends on that wiring.

### Rust-ahead / divergent-by-design
- Ghost-token error recovery integrated into the segmenter crate
  (`build_document_with_ghosts`, iterative `segment_with_recovery`).
- Shared expr evaluator + math functions (`eval.rs`, `mathfunc.rs`) — a generic
  `ExprOps` tree-walk and C-Tcl-9.0-faithful dispatch shared by the const-folder
  and the future VM; no equivalent in Python's *parsing* scope.
- `parse_expr_cached` (Arc-returning LRU) mirroring Python's `@lru_cache`, kept
  separate for the VM hot path.
- tcl-syntax convergence crate consolidating logic scattered across Python.

### Algorithmic / data-structure differences (benign)
- Green nodes own and clone children on both sides (no real structural sharing
  either way). Codepoint widths (Python) vs byte widths (Rust); coincide for
  ASCII. Local-offset-0 build + red anchoring (Rust) vs base-subtract (Python).
- Red views are borrowed `Copy` structs with lazy iterators vs Python
  generators. `partition_point`/`BTreeMap` vs `bisect_right`/`dict`. Expr AST is
  one `ExprNode` enum vs 9 frozen dataclasses — same node set.

### Open questions for maintainer
1. Is the segmenter-level incremental gap intentional because per-file
   incrementality is owned by salsa? If so this row is "by design", not a gap.
2. Are `subcommand`/`braced_word`/`quoted_word` deferred to the Rust
   formatter/minifier port, or recomputed by the consumer?
3. Planned Rust analogue of the `green_tree.py` cross-consumer intern scope?
4. ~~Is the `SYNC-JUN08-1` backslash-newline strip tracked?~~ **DONE
   (FE-LEX, 2026-06-19)** — reconciled to `build.py`; see Gap 4 above.

---

## 3. IR, lowering & compilation unit

### Scope
Python: `compiler/{ir,lowering,compilation_unit,command_binding,dialect_context,eval_helpers,token_helpers,stdlib_prelude,source_inliner,inline_uplevel}.py`,
`lowering_hooks/*`. Rust: `rust/tcl-compiler/src/{ir,ir_helpers,compilation_unit,command_binding,alias,auto_path_eval,inline_uplevel}.rs`,
`lowering/{mod,structured,hooks/*}.rs`, `lowering_hooks.rs`.

### Parity table (selected)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| IRScript / IRAssign* / IRExprEval / IRCall / IRReturn / IRBarrier | ir.py:64-206 | ir.rs:60-265 | ✅ | Field-for-field; `canonical_command` is `Option` in Rust. |
| `IRIncr.safe_on_uninit` | ir.py:87 (registry-queried) | ir.rs:183 (hardcoded false) | ⚠️ | Rust hardcodes `false` (`incr.rs:39`). |
| **`IRInterpBoundary` node + pass** | ir.py:173, passes/interp_boundaries.py | — | ❌ | No variant, no pass, no CU wiring. WASM frame-sync concern. |
| `IRBlock` (`source_args`, `caller_scope`) | ir.py:211-289 | ir.rs:290 | ❗ | Rust `Block` lacks `source_args` and `caller_scope` (drives caller-local read recovery → false-positive risk). |
| `IRForeach.tokens` | ir.py:350 | — | ❗ | No Rust `Foreach.tokens`; braced-body `tcl_eval`-fallback fidelity may regress. |
| `IRProcedure` inlining-catalogue fields | ir.py:425 | ir.rs:559 | ❗ | Rust lacks `inline_decision`/`static_call_count`/`compiler_synthetic`. |
| CommandTrace + `traced_commands()` | ir.py:480-596 | flattened to `Module.traced_commands` + `has_dynamic_trace` | ⚠️ | Pre-flattened at lowering; output-equivalent. |
| Lowering dispatch order | lowering.py:1877 | lowering/mod.rs:1011 + lowering_hooks.rs:73 | ⚠️ | Same order; Rust uses typed `LoweringHookId` (24 variants) — an improvement. |
| Var/value hooks (set/incr/append/lappend/unset/global/variable/upvar/expr/return) | _var.py, _control.py | lowering_hooks.rs, hooks/{incr,control}.rs | ✅ | All 10 ported; `{*}` fallback mirrored. |
| Control hooks (if/while/for/foreach/lmap/catch/try/switch/dict) | lowering.py arms | lowering/structured.rs, mod.rs | ✅ | |
| `foreach_in_collection` arm | lowering.py:2484 | — | ❗ | No dedicated Rust arm; arity-error→IRBarrier not replicated. |
| `eval` barrier relaxation | lowering.py:1548 | mod.rs:1523 | ⚠️ | Ported; `eval [list …]` omits Python's list-requoting safety screen (less conservative). |
| `uplevel` barrier relaxation | lowering.py:1789 | mod.rs:1397 | ❗ | **Hard-disabled** (`STATIC_UPLEVEL_LOWERING=false`); pending VM frame-shift opcodes. |
| `namespace eval` static body → IRBlock | lowering.py:2419 | mod.rs:1587 | ❗ | Rust always emits Barrier, discards lowered body; proc-lifting kept. |
| `_DYNAMIC_BARRIER_COMMANDS` catch-all | lowering.py:2520 | — | ❗ | Missing; dynamic `uplevel $body` lowers to generic Call (not Barrier). |
| `_lower_stub_loop` / is_loop_command | lowering.py:2556 | — | ❌ | `-loop` stub commands barrier instead of becoming loop IR. |
| ArgRole enum + resolver>arg_roles priority | registry/{signatures,runtime}.py | tcl-registry/{arg_role,registry}.rs | ✅ | 15:15 1:1; dynamic resolver wins both sides. |
| `assigns_variable_at` tier | runtime.py:376 | spec.rs:94 (field unused in role path) | ❗ | Rust carries the field but no var-write consumer reads it. |
| `resolve_arg_role_map` (multi-role) | runtime.py:1497 | param_traits.rs:421 | ❗ | Rust single-role over Body/Expr/VarWrite/VarRead only; OO-body/regexp-pattern special-casing + `_REWRITE_ALIASES` not found. |
| command_binding lattice (name resolution) | command_binding.py | command_binding.rs | ✅ | BindingKind lattice — faithful port. |
| CompilationUnit / FunctionUnit | compilation_unit.py:42-60 | compilation_unit.rs:101-308 | ⚠️ | Rust lacks `execution_intent`, `known_classes`. `complexity_guarded` is **RESOLVED (2026-08-04)** — `FunctionUnit.complexity_guarded: bool` (compilation_unit.rs:256) exists and is set by `build_procedure_units`; see §4. |
| **Taint build count** | once/fn (core_analyses.py:3977) | **2× top-level + every proc** (compilation_unit.rs:206,584,596) | ❗ | First intra-proc taint discarded; confirms the "2–3×" review finding. |
| inline_uplevel | inline_uplevel.py | inline_uplevel.rs | ⚠️ | Same gates; Rust lacks the `redefined_procedures` guard; static path test-disabled. |
| source_inliner / stdlib_prelude | source_inliner.py / stdlib_prelude.py | — | ❌ | No Rust equivalent (compile-time bundling for WASM self-containment). |
| auto_path_eval / alias | auto_path_eval.py / shared/alias.py | auto_path_eval.rs / alias.rs | ✅/⚠️ | auto_path near-exact; alias primitives ported, chain-resolver open-coded. |

### Gaps (Rust missing or weaker)
1. **`IRInterpBoundary` node + pass entirely absent.** The IR-as-data
   frame-sync mechanism (`passes/interp_boundaries.py`, wired at
   compilation_unit.py:405) has no Rust counterpart. Expected while Rust WASM
   codegen is in progress, but a structural divergence.
2. **Taint computed twice per proc** (compilation_unit.rs:206 then 584/596).
   Python computes once (core_analyses.py:3977). Pure wasted work per proc.
3. **`uplevel` static relaxation hard-disabled + missing dynamic-barrier
   catch-all.** A dynamic `uplevel $body` lowers to a generic `Statement::Call`
   (uplevel_.rs has no `ArgRole::Body`), which downstream treats as *more*
   analysable than a frame-crossing barrier → potential unsound side-effect/
   escape/dead-store reasoning.
4. **`namespace eval` body discarded** (mod.rs:1598). Codegen/analysis relying
   on `IRBlock.namespace` for unqualified resolution inside the block diverges.
5. **`IRBlock.caller_scope`/`source_args` missing** → risk of false dead-store/
   unused findings for `eval {…}`.
6. **`_lower_stub_loop` missing**; **`source_inliner`/`stdlib_prelude` missing**
   (WASM-bundle self-containment).
7. **`eval [list …]` omits the list-requoting safety screen** (mod.rs:319-324
   checks only `$`/`[`; Python also rejects whitespace/`;`/`\`/leading-`#`/empty
   barewords) — Rust is *less* conservative.
8. **`assigns_variable_at` unused** and **OO-body/regexp-pattern arg-role
   special-casing not found** wrapping Rust's `arg_indices_for_role` — if not
   applied elsewhere, BODY/PATTERN/VAR_WRITE under-reported for
   `method`/`constructor`/`regexp`/`regsub`.
9. Smaller field drops: `IRForeach.tokens`, `IRIncr.safe_on_uninit`,
   `IRProcedure` inlining fields, `FunctionUnit.execution_intent`,
   `CompilationUnit.known_classes`, inline_uplevel `redefined_procedures`.
   (`FunctionUnit.complexity_guarded` is **RESOLVED** — the field exists;
   removed from this list 2026-08-04.)

### Rust-ahead / divergent-by-design
- Typed `LoweringHookId` enum (24 variants) replacing name-string `match`;
  registry-driven `resolve_call` handles aliases/subcommands more robustly.
- `ForeachLine` dedicated hook (Tcl 9.0 TIP 670).
- `reads_own_defs` via `READS_BEFORE_WRITE` trait in the default var-call path.
- `canonical_command` as `Option` with `canonical_command_or_source()` helper.
- Offset-0 rebase + salsa memoisation replacing in-process proc caches.

### Algorithmic / data-structure differences (benign)
- Single `enum Statement` with struct variants vs union of frozen dataclasses;
  `Span` resolved on demand vs inline `Range`. `functions()` sorts procs for
  determinism. FunctionUnit splits each lattice into first-class fields.
  `auto_path_eval.rs` reimplements CPython `os.path` semantics to match
  byte-for-byte.

### Open questions for maintainer
1. Is the double taint build intentional/temporary?
2. `uplevel` lowered as `Call` not `Barrier` — safe for the Rust analyses, or
   should `uplevel_.rs` get an `ArgRole::Body`/barrier fallback?
3. `namespace eval` IRBlock — port planned, or stays barrier + re-segmentation?
4. `IRBlock.caller_scope`/`source_args` and `IRForeach.tokens` — dropped or
   pending?
5. `IRInterpBoundary` — planned for Rust WASM codegen, or different frame-sync?
6. OO-body/regexp-pattern arg-role special-casing + `assigns_variable_at` —
   applied somewhere outside `registry.rs`?
7. `source_inliner`/`stdlib_prelude` — out of phase, or off the roadmap?

---

## 4. CFG, SSA, memory-SSA & dataflow

### Scope
Python: `compiler/{cfg,ssa,memory_ssa,def_use,dataflow_graph,place,place_bridge,loops,static_loops}.py`.
Rust: `rust/tcl-compiler/src/{cfg,cfg_layout,ssa,memory_ssa,def_use,dataflow_graph,place,place_bridge,loops,static_loops}.rs`,
`cfg_builder/{mod,cfg_lower,upvar_info}.rs`.

**Headline:** core algorithms are at high parity — both use **Cooper-Harvey-Kennedy
iterative dominators**, **Cytron dominance frontiers**, and **semi-pruned
(Briggs) SSA**, so phi placement is output-equivalent. The real gaps are in
*downstream wiring* (static-loop → SCCP folding, break-exit modelling,
complexity guard) and a few aliasing-merge divergences.

### Parity table (selected)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Basic-block / terminator model | cfg.py:744-784 | cfg.rs:33-131 | ✅ | `Range`→`Span` only. |
| Block naming scheme | cfg.py | cfg_builder/mod.rs:325-337 | ✅ | All prefixes + `{prefix}_{n}`. |
| if/while/for/foreach/switch/try lowering | cfg.py:1440-1994 | cfg_lower.rs:33-840 | ✅ | Incl. opaque switch wiring, try exception edges. |
| break/continue → CFG goto | cfg.py:1410-1438 | cfg_builder/mod.rs:261-278 | ❗ | Rust not gated on `faithful_exceptions` → codegen-path CFG diverges. |
| `loop_nodes` value | cfg.py:791 `(str, IRFor)` | cfg.rs:141-147 `{entry, span}` | ❗ | Rust drops the `IRFor`; blocks static-for SCCP folding. |
| Dominators (CHK iterative) | ssa.py:770-829 | ssa.rs:302-375 | ✅ | Python cites the Rust fn as reference. |
| Dominance frontier (Cytron) | ssa.py:856-872 | ssa.rs:419-457 | ✅ | |
| **Phi placement (semi-pruned, Briggs)** | ssa.py:974-998 | ssa.rs:487-525 | ✅ | Both semi-pruned — output-equivalent phi sets. |
| Variable renaming / v0 read-before-set | ssa.py:1054-1143 | ssa.rs:1122-1305 | ✅ | Sorted → deterministic versions. |
| `dom_in`/`dom_out` Euler-tour O(1) dominance | ssa.py:885-924 | absent | ⚠️ | Rust walks idom chain; perf-only. |
| **SSA complexity guard** (skip huge bodies) | ssa.py:1008-1043 | `COMPLEXITY_GUARD_BLOCKS`/`DEEP_ANALYSIS_BODY_BYTES` (ssa.rs:309-323), wired at compilation_unit.rs:962-970 | ✅ **RESOLVED (2026-08-04)** | Both the 20k-block and the 256 KiB body-byte half exist and gate `build_procedure_units` (a guarded proc gets `FunctionUnit::trivial_guarded`, not real SSA). Pinned by `complexity_guard_skips_oversized_ssa` / `complexity_guard_flags_byte_huge_proc`. |
| `dynamic_target_name_reads` (`set $X v`) | ssa.py:247-261 | `is_dynamic_write_target` (ssa.rs:384) + analyser suppression (state.rs) | ✅ **RESOLVED (2026-08-04)**, different mechanism | Not a literal port of the Python SSA-layer read-set, but the same user-facing guarantee — `set $p 1` does not dead-store/unused-flag `p` — is implemented in the analyser walker and pinned by `dynamic_name_write_does_not_dead_store_the_name_var` (state.rs). |
| MemoryLocationKind / MemoryOpKind | memory_ssa.py:40-126 | memory_ssa.rs:40-186 | ✅ | All variants 1:1. |
| upvar/global/variable/ns-upvar detection | memory_ssa.py:195-233 | memory_ssa.rs:361-411 | ⚠️ | Rust dispatches on surface `command`, Python on `canonical_command`. Re-checked 2026-08-04: still true, unchanged; not observed to cause a divergence in practice (surface and canonical agree for the un-aliased `upvar`/`global`/`variable` spellings these detectors match). |
| **upvar/global/variable transitive merge** | memory_ssa.py:303-311 | `compute_aliases` (memory_ssa.rs:533-593) | ✅ **RESOLVED (2026-08-04)** | `compute_aliases` was rewritten to a union-find over `MemoryLocation`: `upvar 1 x a; upvar 1 x b` now merges `a`/`b` into one alias set because both key off the shared caller-side node (memory_ssa.rs:545-555, comment explains the encoding). Pinned by `compute_aliases_merges_shared_caller_upvars`. The union-find is generic, so `global`/`variable` pairs get the same transitive treatment for free. |
| `IRUpFrame` clobber | memory_ssa.py:240-244 | `is_clobber` (memory_ssa.rs:437-439) | ✅ **RESOLVED (2026-08-04)** | `Statement::Barrier { .. } \| Statement::UpFrame { .. } => true` — a non-inlined `uplevel {…}` bumps the memory version. Pinned by `build_memory_ssa_emits_clobber_for_upframe`. |
| `eval`-origin IRBlock clobber | memory_ssa.py:245-254 | flattened (mod.rs:511) | ❗ | Rust more precise where enumerable; drops eval-barrier pessimism. |
| DefKind/UseKind + 2-pass build + v0 lazy PARAMETER | def_use.py:26-236 | def_use.rs:24-272 | ✅ | Full structural parity. |
| DataFlowGraph + 4 EdgeKinds + JSON `to_dict` | dataflow_graph.py:30-325 | dataflow_graph.rs | ✅ | All keys match; Rust sorts nodes. |
| `dataflow_graph_to_mermaid` | dataflow_graph.py:328-379 | absent | ❌ | No Mermaid renderer; only consumer builds Mermaid from JSON via LLM. |
| Place model (7 PlaceKind, overlap, may_alias) | place.py:37-254 | place.rs:33-394 | ✅ | Faithful incl. 8D/8E precision. |
| place_bridge read/write/alias extraction | place_bridge.py:49-341 | place_bridge.rs:46-734 | ✅ | Must-alias-kill / element-observed relocated into bridge. |
| Natural-loop detection (back-edge + reverse flood) | loops.py:79-195 | loops.rs:82-166 | ✅ | Rust RPO-ordered (deterministic). |
| `build_loop_forest` single source of truth | called by gvn/intervals/shimmer/core_analyses | loops.rs:127 (production caller: `analyser/diagnostics/helpers.rs:876`) | ✅ **RESOLVED (2026-08-04)** | No longer dead code: `build_loop_entry_only_undef` (the W210 read-before-set loop-entry-only-undef suppression) calls it live, on the production diagnostics path. Still re-derived inline in other spots (not fully consolidated), but "0 prod callers" is no longer accurate. |
| static `for` simulation (env, 4096 cap) | static_loops.py:254-305 | static_loops.rs:314-375 | ✅ | Same cap. |
| static `for` raw-string-args entry | static_loops.py:219-251 | absent | ❌ | Rust only accepts pre-lowered IR. Not re-verified 2026-08-04. |
| static-loop → SCCP/interval const-fold wiring | core_analyses.py:947-961 | absent | ❗ | `summarise_for_statement` exists but no SCCP caller. Not re-verified 2026-08-04. |
| break/`while 1 {… break}` post-loop reachability | core_analyses.py:1345-1374 (SCCP break-exit precompute) | `escaping_loop_jumps` (cfg_builder/mod.rs:1913) wires a real `break_target` CFG edge (cfg_lower.rs:498-537) at CFG-construction time | ✅ **RESOLVED (2026-08-04)**, different mechanism | Rust does not port a Python-style SCCP break-exit precompute; instead the CFG builder gives `break` a genuine successor edge to the post-loop block whenever a break is reachable, so ordinary SCCP reachability propagation (no special-casing needed) marks the post-loop block executable. Pinned by `rch_while1_with_break_post_loop_is_reachable` (control: reachable) alongside `rch_while1_no_break_post_loop_is_dead` (a `while 1` with **no** break still correctly reports the post-loop block unreachable) in `tests/compiler_analysis_residual.rs`. |

### Gaps (Rust missing or weaker)

> Items 1, 3, 4, 5 below (SSA complexity guard, SCCP break-exit /
> `while 1 {… break}`, upvar transitive-merge, `IRUpFrame` clobber) were
> **RESOLVED** as of the 2026-08-04 revalidation (issue #1189) — see the
> parity table above for the current-code citation and pinning test for each.
> They are struck through here rather than deleted so the historical gap
> numbering stays stable for cross-references.

1. ~~**No SSA complexity guard**~~ **RESOLVED** — see parity table.
2. **Static-loop summarisation not wired into SCCP/interval** —
   `summarise_for_statement` has **zero production callers**; compounded by
   `LoopNode` dropping the `IRFor`. Weaker constant propagation/bounds after a
   counted `for`. *(Not re-verified 2026-08-04 — re-check before relying on
   this.)*
3. ~~**SCCP break-exit modelling absent**~~ **RESOLVED** (via a real CFG
   `break_target` edge, not an SCCP-level precompute) — see parity table.
4. ~~**upvar transitive-merge divergence**~~ **RESOLVED** — see parity table.
5. ~~**`IRUpFrame` not a clobber in Rust**~~ **RESOLVED** — see parity table.
6. **`break`/`continue` not gated on `faithful_exceptions`** — the codegen-path
   CFG gains goto edges Python leaves as fall-through; relevant to the
   byte-identical-to-tclsh invariant. *(Not re-verified 2026-08-04.)*
7. `dataflow_graph_to_mermaid` missing (bounded: JSON `to_dict` present);
   static `for` raw-args entry missing. *(`dynamic_target_name_reads` and
   `build_loop_forest`-as-dead-code are **RESOLVED** — see parity table;
   the remaining two items in this line were not re-verified 2026-08-04.)*

### Rust-ahead / divergent-by-design
- `escaping_loop_jumps` recurses into `try` and stops at dead code (mod.rs:1090)
  — more sound than Python (cfg.py:238 does neither); matches tclsh.
- `Block` flattening = higher precision (inlines static `eval {…}` before
  memory-SSA), at the cost of barrier pessimism (gap 5b).
- `same_literal_element` explicit `Literal` gate (place_bridge.rs:449) more
  correct than Python's dead `getattr(index,"dynamic",False)`.
- Deterministic ordering throughout (sorts dataflow nodes, loop blocks; RPO).
- Split `build_cfg` (faithful) vs `build_cfg_codegen` entry points.

### Algorithmic / data-structure differences (benign)
- `Range`→`Span`, `dict`→`HashMap`/`BTreeMap`, `set`→`HashSet`/`BTreeSet`,
  arbitrary-precision int (Python, never overflows) vs `checked_add`/saturating
  (Rust, bounded by the shared 4096 cap). Owned `String` + SipHash keys (per
  review-findings) — values hash/compare identically. Determinism preserved by
  explicit sorting at output boundaries.

### Open questions for maintainer
1. ~~Is the absent SSA complexity guard intentional?~~ **Answered (2026-08-04):**
   it isn't absent — `is_complexity_guarded` + `DEEP_ANALYSIS_BODY_BYTES` gate
   `build_procedure_units` before SSA is ever built for an oversized proc.
2. Is the static-loop → SCCP/interval const-fold wiring planned? (`LoopNode`
   would need to carry the `IRFor`.) *(Not re-verified 2026-08-04.)*
3. ~~Does Rust SCCP handle the break-into-infinite-loop exit edge another
   way?~~ **Answered (2026-08-04):** yes — the CFG builder
   (`escaping_loop_jumps` + `break_target`) gives `break` a real successor
   edge to the post-loop block, so plain SCCP reachability propagation
   handles it without any SCCP-level special-casing.
4. ~~Should Rust match Python's single-set upvar merge?~~ **Answered
   (2026-08-04):** it already does — `compute_aliases`'s union-find merges
   `upvar 1 x a; upvar 1 x b` into one alias set.
5. Should memory-SSA detection consult `canonical_command` instead of surface
   `command`? *(Re-checked 2026-08-04: still dispatches on surface `command`;
   the maintainer question stands, though no concrete divergence was found in
   this pass.)*
6. Confirm codegen uses `build_cfg_codegen` so the un-gated break/continue goto
   edges don't leak into emitted bytecode. *(Not re-verified 2026-08-04.)*
7. ~~Is `build_loop_forest` meant to become the Rust single source of
   truth?~~ **Partially answered (2026-08-04):** it now has a real production
   caller (`analyser/diagnostics/helpers.rs:876`), so it is no longer dead
   code; whether it should also replace the *other* inline re-derivations is
   still open.

## 5. Value/type analyses — type inference, SCCP, intervals, shapes, shimmer

### Scope
Python: `compiler/{types,core_analyses,analysis_types,value_shapes,intervals,interval_bounds,rendered_properties,shimmer,tcl_expr_eval,expr_types,expr_registry,scan_format,tcl_constants}.py`.
Rust: `rust/tcl-compiler/src/{type_infer,types,analyses,value_shapes,intervals,interval_bounds,rendered_properties,tcl_expr_eval,scan_predicate,sccp}.rs`, `shimmer/{mod,expr,graph,hints,phi,span,thunking,use_site}.rs`.

### Diagnostic-code coverage (S-codes + bounds/type W/I-codes)

| Code | Meaning | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|---|
| **S100** | Single shimmer outside loop (INFO) | shimmer.py:287,544 | use_site.rs:171, expr.rs:273/322 | ❗ | Python S100→**Information** (diagnostics.py:49); Rust hardcodes **Warning** for all shimmer (compiler_checks.rs:113). Plus expr.rs always emits S100 (never S101 in loop). |
| **S101** | Shimmer in loop body (WARNING) | shimmer.py:281,545 | use_site.rs:171, phi.rs:146 | ❗ | phi.rs hardcodes "S101" ignoring `in_loop` (should be S100 outside loops); missing loop-invariant S101→S100 downgrade + incr-amount check. |
| **S102** | Var oscillates across loop iters (WARNING) | shimmer.py:996,1165 | thunking.rs:303 | ⚠️ | Core oscillation faithful; destructure-foreach suppression weaker. Severity OK. |
| **W230** | `lindex` index proven out-of-range | interval_bounds.py:463 | interval_bounds.rs:543 | ✅ | |
| **W231** | `lset` index proven out-of-range | interval_bounds.py:463 | interval_bounds.rs:539 | ✅ | Append-slot `i==length` legal both. |
| **W232** | `string index` proven out-of-range | interval_bounds.py:463 | interval_bounds.rs:541 | ✅ | |
| **W233** | Division/modulo by provably-zero divisor | interval_bounds.py:501 | DivZeroFinding interval_bounds.rs:642 (**DEAD**); emitted via diagnostics.rs:7275 (SCCP-only) | ❗ | Production path uses SCCP-constant-only; misses interval `[0,0]` divisors Python catches. |
| **I230** | Constant branch / `info exists` fold | core_analyses.py:1563 | sccp.rs:480/548 | ✅/⚠️ | Records produced both; Rust existence-fold gate weaker, doesn't prune edges. |
| **O107** | Unreachable block | executable_blocks | diagnostics.rs:5882 | ✅ **RESOLVED (2026-08-04)** | The CFG builder gives `break` a real successor edge to the post-loop block (`escaping_loop_jumps`/`break_target`, §4), so ordinary SCCP reachability marks it executable — no false O107 for `while 1 {… break …}`. Pinned by `rch_while1_with_break_post_loop_is_reachable` / `rch_while1_no_break_post_loop_is_dead` (`tests/compiler_analysis_residual.rs`). |

### Parity table (selected)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Type lattice (UNKNOWN/KNOWN/SHIMMERED/OVERDEFINED), join, numeric promotion | types.py | types.rs | ✅ | Faithful. |
| TclType domain (Int/Double/Bool/String/List/Dict/Bytearray/Numeric/Object/Channel) | types.py:30-42 | tcl_registry::TclType | ✅ | Single source of truth in registry crate. |
| Set-statement literal classifier (dec/hex/bin→INT, `0o`→STRING, signed) | core_analyses.py:3683 | type_infer.rs:55-82 | ✅ | |
| Expr-context literal classifier (`0o`→INT, unknown→NUMERIC) | expr_types.py:54-92 | type_infer.rs:128 (uses set-statement classifier) | ❗ | `expr {0o17+1}` literal → Python INT, Rust STRING; unknown → Python NUMERIC, Rust STRING. |
| Unary `~` (BitNot) type | expr_registry.py:94→INT | type_infer.rs:197 (identity) | ❗ | `~$double` → Python INT, Rust Double. |
| Arithmetic/bitwise/comparison/iRules-predicate/math-fn result types | expr_types/expr_registry | type_infer.rs:143-285 | ✅ | |
| Scope-alias (`global`/`variable`/`upvar`) def → OVERDEFINED | core_analyses.py:3905-3915 | — | ❌ | Rust widens only Barrier defs → shimmer FP risk on aliases. |
| TclOO/snit object typing (`[Foo new]`→OBJECT, `known_classes`) | core_analyses.py:3610-3680 | type_infer.rs:91-118 (registry only) | ❌ | No constructor typing; affects W307/W308. |
| Constant lattice incl. CONSTSET + 32-cap widening + join | analysis_types.py | analyses.rs / sccp.rs:100-164 | ✅ | Full CONSTSET; deterministic Vec ordering. |
| SCCP fixpoint, executable blocks/edges, const-branch pruning, try-edges | core_analyses.py:1286 | sccp.rs:231 | ⚠️/❗ | See SCCP gaps. |
| Interval domain (join/widen/intersect, add/sub/mul/neg, guards, fixpoint) | intervals.py | intervals.rs | ✅ | Rust adds `checked_*` overflow→inf (sound). |
| Value shapes `is_pure_var_ref` | value_shapes.py:65 | value_shapes.rs:9 | ❗ | Rust rejects `$arr(idx)` and `${a(1)}`; Python accepts. Different acceptance set. |
| Value shapes `parse_command_substitution` | value_shapes.py:103 (segmenter, multi-cmd bail) | value_shapes.rs:26 (whitespace split) | ⚠️ | Rust not brace/quote-aware. |
| Rendered properties: 13 flags, 3 masks, may/must lattice | rendered_properties.py | rendered_properties.rs | ✅ | Field-level full parity. |
| Rendered props: ESC backslash-rendering (`\x2f`→`/`) | rendered_properties.py:251-264 | rendered_properties.rs:437-465 | ❗ | Rust doesn't render numeric/hex escapes → W201 false-negatives. |
| Expr evaluator: all operators incl. iRules ops | tcl_expr_eval.py | tcl_expr_eval.rs + tcl-syntax walk | ✅ | Integer overflow→None (benign). |
| Expr math functions | tcl_expr_eval.py:818-858 | tcl-syntax mathfunc.rs | ✅/➕ | Rust adds isnormal/issubnormal/isunordered; `isqrt` f64-precision risk >2^52. |
| Scan predicate (`scan_provably_no_match`, W210) | scan_format.py | scan_predicate.rs | ✅ | Near line-for-line. |
| Format (printf) folding | codegen `_values.py:759` (`%s`/`%d`/`%%`) | tcl-registry format_.rs + tcl-syntax format.rs (full grammar) | ➕ | Rust far ahead. |

### Gaps (Rust missing or weaker) — by impact
1. **Shimmer severity regression (observable).** Rust hardcodes `Severity::Warning` for all shimmer (compiler_checks.rs:113,124); Python maps **S100→Information**. Every S100 over-elevated.
2. **Shimmer code-string bugs (observable).** `phi.rs:146` hardcodes "S101" ignoring `in_loop` (out-of-loop merge should be S100); `expr.rs:273,322` hardcode "S100" ignoring `in_loop` (in-loop expr shimmer should be S101).
3. **Shimmer loop-invariant reclassification missing** (S101→S100 downgrade for loop-invariant single-target vars; the largest use-site heuristic, shimmer.py:270-456). Rust over-reports S101.
4. **Shimmer `incr` amount not checked** (shimmer.py:564-606); Rust's Incr arm never inspects the amount.
5. ~~**SCCP break-edge reachability missing**~~ **RESOLVED (2026-08-04)** —
   see §4/O107 above (real CFG `break_target` edge, not an SCCP-level fix).
6. **SCCP optimistic (Wegman–Zadeck) UNKNOWN deferral missing** (sccp.rs:444-456 opens both arms immediately); loses loop-carried-constant folding. *(Not re-verified 2026-08-04.)*
7. ~~**SCCP externally-mutable/escaping-var widening missing (soundness).**~~
   **RESOLVED (2026-08-04)** — `evaluate_def` (sccp.rs:300-335) now calls
   `var_observability::analyse_var_observability(..).escaping_var_names()`
   and forces every escaping/global/namespace/upvar-aliased/traced
   definition to `Overdefined` before the fixpoint runs, with the same
   soundness rationale Python used (`set ::g 5; mut; expr {$::g+1}` must not
   fold through the opaque `mut` call).
8. **SCCP interpolation folding + registry-driven cmd-subst folding narrower** (Rust hand-codes list/format/llength/string-length/expr only; no CONSTSET cartesian product; no FOLD_HINTS rename guard).
9. **Type-prop scope-alias & TclOO object typing missing**; **expr-context literal typing & `~` typing wrong**; **W233 interval path bypassed**; **rendered-props ESC numeric escape rendering weaker** (W201 FNs); **`value_shapes.is_pure_var_ref` acceptance-set differs** (`is_braced_whole_name_array_ref` not ported).

### Rust-ahead / divergent-by-design
- Format (printf) folding vastly more complete and Tcl-version-aware.
- Three extra Tcl-9.0 expr classifiers (isnormal/issubnormal/isunordered).
- `matches_regex` intentionally not folded (avoids Python-`re` ≠ Tcl-ARE mismatch).
- Overflow-hardened interval/SCCP arithmetic (`checked_*`→inf/Overdefined).
- `resolve_list_length` phi-following compensates for non-pruned SSA shape.
- Deterministic ordering throughout (sorted phi preds, CONSTSET Vecs, span-sorted findings).
- Shared evaluator/grammar architecture reused by const-folder and runtime VM.

### Algorithmic / data-structure differences (benign)
- `while changed` round-robin fixpoints vs def→use worklists — same fixpoint, perf-only.
- Interval `dominates`/`loop_headers` re-implemented locally vs imported from `loops.py` — equivalent on reducible CFGs.
- CONSTSET `Vec`+sort vs `frozenset` (incl. NaN bucketing via `to_bits`).

### Open questions for maintainer
1. Shimmer severity + phi-S101/expr-S100 hardcodes — design simplification or port bugs?
2. Loop-invariant S101→S100 downgrade and incr-amount check — deferred or overlooked?
3. W233: is the SCCP-only divisor check deliberate (making `find_divide_by_zero` dead code)?
4. ~~SCCP break-edges / escaping-var widening — tracked simplifications or
   gaps?~~ **Answered (2026-08-04):** both are resolved — see items 5 and 7
   above. The optimistic (Wegman–Zadeck) UNKNOWN-deferral half of this
   question was not re-verified.
5. Type-prop TclOO object typing & scope-alias OVERDEFINED — planned?
6. expr-context literal typing (`0o`→INT, unknown→NUMERIC) and `~`→INT — distinct expr classifier needed?
7. `value_shapes.is_pure_var_ref` acceptance-set — match Python (shared primitive across passes)?
8. Rendered-props ESC rendering — deliberate simplification vs the design doc's requirement?

---

## 6. Taint, var-escape, interprocedural & side-effects

### Scope
Python: `compiler/taint/*`, `compiler/var_escape/*`, `compiler/{interprocedural,side_effects,execution_intent,var_observability,var_refs,var_resolve,var_scoping,connection_scope,command_trust,proc_arg_traits,proc_fingerprint}.py`, `analyser/signature_scan.py`.
Rust: `rust/tcl-compiler/src/{taint,taint_interproc,path_concat,uri_split,interprocedural,side_effects,execution_intent,var_observability,var_refs,var_resolve,var_scoping,connection_scope}.rs`, `var_escape/*`, `signature_scan/*`.

### Diagnostic-code coverage (T-codes + relevant W/IRULE codes)

| Code | Meaning | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|---|
| T100 | Tainted → code-exec sink; braced-`expr` numeric-coercion | _sinks.py:65,460,668 | taint.rs:1121,1964 | ✅ | Sink classify + expr-operand msg verbatim-equal. |
| T101 | Tainted → output (`puts`) | _sinks.py:70,465 | taint.rs:1127 | ✅ | |
| T102 | Option injection without `--` | _sinks.py:71,480 | taint.rs:2343 | ✅ | `--` filter match. |
| T103 | Tainted regexp/regsub (ReDoS) | _sinks.py:76,714 | taint.rs:1934 | ✅ | |
| T104 | Tainted network-address arg (SSRF) | _sinks.py:486 (registry-driven) | taint.rs:1200,2044 | ✅ | IP/PORT/FQDN colour suppression both. |
| T105 | Cross-interp code injection (`interp eval`) | _sinks.py:494 | taint.rs:1205 | ✅ | |
| T106 | Double-encoding | _sinks.py:77,735 | taint.rs:1609 | ✅ | |
| W201 | Manual path concat | _path_concat.py:191 | path_concat.rs:283 | ✅ | |
| W313 | Destructive file op on var/tainted path | _sinks.py:49-53 | taint.rs:1318 | ✅ | |
| IRULE3001-3004 | iRules output/header/log/redirect sinks | _sinks.py:56-62 | taint.rs:1164-1174 | ✅ | Rust hardcodes the command list (TODO note). |
| IRULE3101 | Setter constraint (path must start `/`) | _uri_split.py:50 | compiler_checks.rs:262 | ✅ | |
| IRULE3103 | `*::uri` getter + manual decomposition | _uri_split.py:55 | uri_split.rs:865 | ✅/➕ | Rust ahead on `::`-qualified detection. |

**All T100–T106 plus the related W/IRULE codes have Rust parity and are wired live** in `run_all_checks` (`compiler_checks.rs:130-139,245-303`). This is the strongest-parity subsystem in the audit.

### Parity table (cluster-level)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Taint lattice (states/join/colours) | _lattice.py:45-89 | taint.rs:92-253 | ⚠️ | Join identical; 2 colours missing in compiler mirror enum (below). |
| Taint sources/sanitisers | _lattice.py:167-231 | taint.rs:298-375 | ✅ | Both registry-driven. |
| Taint sinks/suppression | _sinks.py | taint.rs:1112-2210 | ✅ | All suppression masks match. |
| uri_split (IRULE3103) | _uri_split.py | uri_split.rs | ➕ | Rust closes a Python false-negative on `::split`/`::string`. |
| Interproc taint summaries/fixpoint | _interprocedural.py | taint_interproc.rs | ⚠️ | Faithful; drops internal summaries from result (benign). |
| interprocedural.py (call graph/summaries/fixpoint) | interprocedural.py | interprocedural.rs | ❗ | Several precision gaps (G1/G3/G4/G5 below). `TCL_LSP_RUST_INTERPROC` gate inert. |
| side_effects.py (effect classification) | side_effects.py | side_effects.rs | ❗ | Core ported; protocol-NS classifier + form resolution + trace-composition gaps. |
| execution_intent.py | execution_intent.py | execution_intent.rs | ⚠️ | Missing `has_expansion` (`{*}`) field. |
| var_escape (full subsystem) | compiler/var_escape/* | var_escape/* | ❗ | Transfer functions ported & tested, but **no top-level driver wired** + missing `pure_leaf` family + precision regressions. |
| var_scoping | var_scoping.py | var_scoping.rs | ❗ | Missing `allow_dynamic_target` param. |
| var_resolve | var_resolve.py | var_resolve.rs | ✅ | Faithful. |
| var_refs | var_refs.py | var_refs.rs | ❗ | 3 read-before-set recovery helpers + 2 recursion options unported. |
| var_observability | var_observability.py | var_observability.rs | ⚠️ | Fully ported but no Rust consumer yet. |
| connection_scope (iRules) | connection_scope.py | connection_scope.rs | ⚠️ | Missing branch-condition v0 sweep. |
| command_trust | command_trust.py | command_binding.rs:421-580 | ⚠️ | Divergent-by-design (explicit value vs scoped global); output-equivalent. |
| proc_arg_traits | proc_arg_traits.py | param_traits.rs + interprocedural.rs | ❗ | Missing `DYNAMIC_NAME_LOCAL` trait. |
| proc_fingerprint | proc_fingerprint.py | (none) | ❌ | `dependency_fingerprint` unported. |
| signature_scan | analyser/signature_scan.py | signature_scan/* | ✅ | Full parity; Rust is the sole live path (shim removed #241). |

### Gaps (Rust missing or weaker) — by severity
1. **Taint lattice: `PATH_JOINED` (1<<15) and `CHANNEL` (1<<16) colours missing from the compiler mirror enum** (taint.rs stops at FQDN 1<<14). The registry defines both; `reg_colour` uses `from_bits_truncate`, so they're silently dropped. Breaks the W201 `PATH_JOINED` suppression arm. **Currently dormant** (no engine propagates these yet) but the lattice cannot represent them.
2. ~~**`complexity_guarded` skip absent in Rust (cross-cutting).**~~
   **RESOLVED (2026-08-04)** — see §4's SSA-complexity-guard entry. The guard
   is checked once, at `FunctionUnit` construction
   (`compilation_unit.rs::build_procedure_units`), and a guarded proc's taint
   is trivially empty (`FunctionUnit::trivial_guarded` sets
   `taints: Arc::default()`) and is filtered out of every per-proc
   interprocedural consumer (`compilation_unit.rs:1661,1702,1734`) — so taint
   and interprocedural summarisation inherit the same skip as SSA, from one
   wiring point, rather than needing their own separate guard.
3. **var_escape not wired up.** No Rust `analyse_var_escape` orchestrator, no `CfgEscapeResult → ProcEscapeSummary` conversion; transfer functions reachable only from var_escape's own tests. Missing the entire `pure_leaf` family (`safe_to_inline`/`safe_to_dce`/`safe_for_frame_elision`) and its transitive IPA fixpoint.
4. **var_escape precision regressions:** array-element narrowing (`set arr($k) v` spills all locals in Rust vs only `arr`); multi-command `[a; b]` cmd-subst fallback unported; value-embedded cmd-subst head not recorded as a callee (breaks interproc through `set x [::ns::Helper ...]`); `with_escapes` missing `CALLEE_UPVAR` reasons + interprocedural UNKNOWN barrier.
5. **interprocedural precision:** constant-return/param-dependency derived textually vs Python's SSA/SCCP lattice; CFG-augmented upvar global-write 2nd pass missing; `can_fold_static_calls` doesn't consult `redefined_procedures`; `::call` indirection not seen through. (O103 fold recovered separately in `propagation.rs`, so end-to-end folding may still work.)
6. **side_effects:** protocol-namespace classifier (`.key` extraction, `-normalized`) unported; form-aware getter/setter resolution absent; trace-execution composition only a purity gate.
7. **proc_fingerprint entirely unported** — Rust memoises on body-hash + param_constants only; risk of stale per-proc diagnostics when an external symbol's resolution changes but the body is byte-identical (may be subsumed by whole-unit rebuild).
8. **proc_arg_traits `DYNAMIC_NAME_LOCAL` missing** — Rust emits plain `VarWrite` for `scan`/`lassign`/`regsub`/`set $p` callee-local dynamic-name cases; Python emits `DYNAMIC_NAME_LOCAL + VAR_READ` to avoid caller-side suppression FPs (#498/#499). Affects W211/W220/O109 suppression.
9. **var_refs read-before-set recovery helpers unported** (`command_sub_write_names`, `existence_test_names`, `body_write_names`) → read-before-set FP risk.
10. **Smaller:** `var_scoping` missing `allow_dynamic_target`; `connection_scope` missing branch-condition v0 sweep; `execution_intent` missing `has_expansion`.

### Rust-ahead / divergent-by-design
- uri_split closes a Python false-negative on `::`-qualified `split`/`string`.
- signature_scan: `BTreeMap` determinism + precomputed skip-heads; sole live path.
- param_traits dialect-aware (`LexerConfig` 8.4 vs 8.5+ `{*}`) + stub-overlay traits.
- interprocedural materialises transitive `calls` closure + `direct_calls`.
- var_escape packs pessimism into an `EscapeFlags` bitflags byte; `dynamic_barrier()` invariant in code.
- command_trust threads an explicit `ModuleCommandMutations` value vs scoped global.

### Algorithmic / data-structure differences (benign)
- `frozenset`/`dict` → `BTreeSet`/`HashSet`/`HashMap`/`BTreeMap`; `Range`→`Option<Span>`.
- Fixpoints differ in iteration strategy (Python reverse-call-graph worklists; Rust round-robin/sorted-name) — monotone lattices, identical least fixpoints.
- Hand-rolled byte scanners replace Python regex/parser helpers (verified equivalent for tested shapes).
- Module placement: `ProcTaintSummary` in `taint_interproc.rs`; `evaluate_proc_with_constants` in `propagation.rs`.

### Open questions for maintainer
1. **`TCL_LSP_RUST_INTERPROC` is inert** — the helper names it but nothing calls it, and the `interprocedural_summaries` binding is unused (exposes only `procedures`, never `methods`). Intentional, or lost wiring? Docs read as if it's live.
2. Is **var_escape staged work** (no orchestrator, passes only exercised by tests)? Are `pure_leaf` + its IPA fixpoint deferred because Rust inlining/DCE consumers don't exist yet?
3. **proc_fingerprint:** absence intentional because whole-unit interproc rebuild subsumes external-symbol invalidation, or a genuine incremental-correctness gap?
4. **`PATH_JOINED`/`CHANNEL` colours:** extend the compiler mirror enum to the full 17-bit registry set, or keep registry-only until propagation lands?
5. ~~**`complexity_guarded`:** replicate the guard (taint + interproc)?~~
   **Answered (2026-08-04):** yes, already done — see §4.
6. ~~**`DYNAMIC_NAME_LOCAL`:** add the variant and emit it?~~ **Answered
   (2026-08-04):** already done — see §8.
7. **side_effects:** protocol-NS classifier, form resolution, structured trace-composition — planned or accepted gaps?
8. **Registry trait fidelity** (var_escape slot resolution): do `FRAME_HASH_BUILTIN`/`INTROSPECTS_BY_NAME`/`TARGETS_VARIABLE_BY_NAME`/`DYNAMIC_EVAL_BODY` reproduce the Python frozensets exactly?

---

## 7. Optimiser & passes

### Scope
Python: `compiler/optimiser/*`, `compiler/passes/*`, `compiler/gvn.py`, `compiler/inlining/*`, `compiler/{inline_uplevel,specialise_factories,static_loops}.py`.
Rust: `rust/tcl-compiler/src/optimiser/*`, `rust/tcl-compiler/src/{sccp,gvn,loops,static_loops,inline_uplevel,specialise_factories,lattice_rebase,dead_stores}.rs`.
Default-off behind `TCL_LSP_RUST_OPTIMISER` / `TCL_LSP_RUST_GVN`. The O-codes split, on both sides, into a **rewrite pipeline** (`run_passes`) and a **diagnostic aggregator** (`run_all_checks`, owns O105/O106) — faithfully mirrored.

### O-code coverage (O100–O130, 31 codes)

| O-code | Pass | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|---|
| O100 | Constant propagation | _propagation.py:276 | propagation.rs:1081 | ✅ | |
| O101 | Fold constant expr | _branch_folding.py:159 | branch_folding.rs:288 | ⚠️ | Branch-condition post-cascade fold Python-only. |
| O102 | Fold expr cmd-subst | _propagation.py:401 | propagation.rs:180 | ⚠️ | Rust reuses "O102" for literal load-forwarding; the expr-cmd-subst transform delegated elsewhere. Net behaviour covered. |
| O103 | ICIP interprocedural fold | _propagation.py:537 | propagation.rs:769 | ❗ | Naive namespace resolution (no chain walk), no flow-sensitive `rename` gate. |
| O104 | String-build chain | _pattern_recognition.py:316 | pattern_recognition.rs:98 | ❗ | **Hint-only in Rust**; Python emits the fold + dead-write deletions. |
| O105 | GVN/CSE redundancy | gvn.py:1398 | gvn.rs:1535 → compiler_checks.rs:211 | ✅ | Full + partial redundancy both. |
| O106 | LICM loop-invariant | gvn.py:814 | gvn.rs:1185 → compiler_checks.rs:217 | ⚠️ | Logic present, but **O106 missing from `profiles.rs` `OPT_CATEGORIES`** → unsuppressable under non-full profiles. |
| O107 | DCE unreachable | _elimination.py:638 | elimination.rs:350 | ✅ | |
| O108 | ADCE transitive | _elimination.py:320 | elimination.rs:588 | ❗ | Rust treats all assignments as side-effect-free (drops `[`-check + exec-intent gate) — over-aggressive. |
| O109 | Dead store | _elimination.py:516 | elimination.rs:426 | ✅ | |
| O110 | InstCombine | _expr_simplify.py | helpers/expr_simplify.rs:882 | ❗ | Missing identities (ternary shapes, `x&&1`/`x||0`, regex/glob→string-op, full inversion/De Morgan). |
| O111 | Brace-expr hints | _propagation.py:91 (dead registration) | — | ✅ | Correctly absent. |
| O112 | Constant condition / SCCP collapse | _structure_elimination.py | structure_elimination.rs | ✅ | Near 1:1; messages verbatim. |
| O113 | Strength reduction | _expr_simplify.py:1346 | helpers/expr_simplify.rs:710 | ✅ | `x**2→x*x`, `x%2^N→x&mask`. |
| O114 | Incr idiom | _pattern_recognition.py:537 | pattern_recognition.rs:145 | ❗ | **Drops INT-type soundness gate** → rewrites float `$x` unsoundly. |
| O115 | Nested expr unwrap | _branch_folding.py:107 | propagation.rs:1022 | ⚠️ | Branch-condition path Python-only. |
| O116 | List folding | _propagation.py:335 | propagation.rs:1275 | ✅ | |
| O117 | String-length zero-check | _expr_simplify.py:1396 | helpers/expr_simplify.rs:627 | ⚠️ | Operator coverage narrower in Rust. |
| O118 | Lindex folding | _propagation.py:335 | propagation.rs:1276 | ✅ | |
| O119 | Multi-set packing | _pattern_recognition.py:612 | pattern_recognition.rs:57 | ❗ | **Hint-only in Rust**; Python emits `lassign`/`foreach` + deletions. |
| O120 | String compare eq/ne | _expr_simplify.py:484 | helpers/expr_simplify.rs:1040 | ❗ | **Unsound in Rust**: fires on any string literal without numeric check → `$x == "1"`→`eq` flips result. Non-recursive. |
| O121 | Tail-call detection | _tail_call.py:348 | tail_call.rs:489 | ✅ | Both gate to 8.6+. |
| O122 | Tail-recursion → loop | _tail_call.py:404 | tail_call.rs:155 | ✅ | Rust emits a real `while {1}` rewrite (doc header stale). |
| O123 | Accumulator introduction | _tail_call.py:505 | tail_call.rs:369 | ❗ | **Over-fires** (no single-self-call guard, no associative-op requirement; wrongly fires on fib). |
| O124 | Unused proc (iRules) | _unused_procs.py | unused_procs.rs | ✅ | iRules gating, RULE_INIT exclusion, reachability BFS. |
| O125 | Code sinking | _code_sinking.py | code_sinking.rs:145 | ❗ | Misses cross-event-var guard, already-covered guard, multi-branch/deepest-target descent. |
| O126 | Unused variable assignment | _elimination.py:569 | elimination.rs:534 | ✅ | |
| O127 | Inline single-use / load forwarding | _propagation.py:1023 | propagation.rs:230 | ✅ | |
| O128 | End-offset index rewrite | _pattern_recognition.py:815 | — (only profiles.rs:63) | ❌ | **Not emitted anywhere in Rust.** |
| O129 | Fold pure builtin cmd-subst | _propagation.py:284 | propagation.rs:1279 | ✅ | Rust adds rename-trust gate (tightening). |
| O130 | Lappend build chain | _pattern_recognition.py:309 | — (only profiles.rs:78) | ❌ | **Not emitted anywhere in Rust.** |

### Parity table (passes & infrastructure)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Pass manager façade | _manager.py:188 | manager.rs:38 | ⚠️ | Python interleaves per-statement; Rust runs discrete passes via `run_passes`. Output-equivalent intent. |
| Pipeline ordering | _manager.py:339 | mod.rs:329 `PassId::all()` | ⚠️ | Orders differ but both reach a comparable fixpoint via overlap selection. |
| Overlap selection | _helpers + _manager.py:624 | helpers/select + manager.rs:106 | ⚠️ | Rust adds determinism sort + `renumber_groups`. Python's O112-replacement var-ref protection + orphaned-O125 cleanup — verify Rust equivalent. |
| Const-prop ↔ dead-store coupling | _elimination.py:718 | manager.rs:268 | ✅ | Faithful. |
| Multipass/fixpoint | _manager.py:769 (`disabled` param) | manager.rs:669 (**no `disabled` param**) | ⚠️ | Rust callers must filter per-iteration (CLI does; verify others). |
| Profiles (off/readability/standard/full/aggressive) | shared/optimisation_profiles.py | optimiser/profiles.rs | ❗ | **`OPT_CATEGORIES` 30 entries, missing O106→code_motion** (Python 31). |
| GVN/CSE (O105) + LICM (O106) | gvn.py | gvn.rs → compiler_checks.rs | ✅ | Logic present (the premise that Rust lacks LICM is false). |
| Uplevel inlining | inline_uplevel.py | inline_uplevel.rs | ✅ | |
| General proc inlining | inlining/inline_pass.py (1859 LOC) + decision.py + _rename.py | — | ❌ | **No Rust port of the v0–v3 splice inliner.** |
| Static loop evaluation | static_loops.py | static_loops.rs / loops.rs | ✅ | Cap 4096; loop *unrolling* unimplemented both. |
| Factory specialisation | passes/specialise_factories.py | specialise_factories.rs | ✅ | |
| Interp boundaries | passes/interp_boundaries.py | — | ❌ | No `IRInterpBoundary`-insertion pass. |
| WASM-codegen DCE (IR-rewriting) | passes/dce.py | — | ❗ | Rust `elimination.rs` produces diagnostics, not rewritten IR for codegen. |
| WASM-codegen GVN rewrite | passes/gvn.py | — | ❗ | IR-rewriting transform; no Rust analogue. |
| SCCP core analysis | core_analyses | sccp.rs / lattice_rebase.rs | ✅ | Underpins O112/O100. |

### Gaps (Rust missing or weaker)
1. **O128 and O130 entirely unimplemented** — present only as profile category entries.
2. **O104 and O119 are hint-only** where Python emits applicable rewrites.
3. **O106 missing from `OPT_CATEGORIES`** → LICM hints unsuppressable under non-full profiles. (Add `("O106", OptCategory::CodeMotion)`.)
4. **General proc inlining (`compiler/inlining/`) unported** — the largest missing in-scope feature.
5. **O120 unsound** (string-compare rewrite without numeric proof, non-recursive).
6. **O114 drops the INT-type gate** (unsound float→`incr`).
7. **O108 ADCE over-aggressive** (treats all assignments as pure).
8. **O103 weaker resolution/gating** (no namespace-chain walk, no flow-sensitive rename gate).
9. **O125 code-sinking gaps** (no cross-event-var guard, single-branch only).
10. **O110 InstCombine missing identities.**
11. **O101/O108/O115 branch-condition coverage** absent in Rust's `propagate_into_branches`.
12. **WASM-codegen passes (`passes/{dce,gvn,interp_boundaries}.py`) unported** — codegen-quality regression if Rust drives the same emitter.
13. **`optimise_source_multipass` lacks a `disabled` parameter** — per-iteration filter must be replicated by callers.
14. **O105 string-interpolation propagation labelled O100 in Rust** — transform-equivalent, but telemetry keyed on O105 sees nothing.

### Rust-ahead / divergent-by-design
- Determinism canonicalisation (`renumber_groups` + total-order sort) for byte-identical output across HashMap iteration / offset-0 memo (supports salsa early-cutoff).
- Richer scope-alias detection (`namespace upvar`, `trace add variable`).
- O129 rename-trust gate (module-wide rebinding check) Python lacks.
- O112 glob via `tcl_syntax::glob` runtime matcher (arguably more correct than Python `fnmatch`).

### Algorithmic / data-structure differences (benign)
- Pass orchestration model (interleaved per-statement vs discrete passes) — output-equivalent intent.
- O102 code-label reuse with the Python transform delegated to other Rust modules.
- O127 excludes Terminator uses from the single-use count (deliberate, documented).
- GVN/LICM live in the diagnostic aggregator on both sides — expected, not a gap.

### Open questions for maintainer
1. Is the **O106 omission from `OPT_CATEGORIES`** a known gap? (Recommend adding it.)
2. Are **O104/O119/O128/O130** meant to ship as applicable rewrites before default-on, or is hint-only/absent acceptable for v1?
3. The **O120/O114 soundness gates** are dropped in Rust — deliberate or missed? They can miscompile, so likely block default-on.
4. Is **general proc inlining** in scope for the Rust optimiser, or codegen/WASM-only? Same for `passes/{dce,gvn,interp_boundaries}.py`.
5. Does Rust's overlap-selection replicate Python's **O112-replacement var-ref protection** and **orphaned-O125 cleanup**?
6. **GVN docstring code labels** (gvn.rs:954,987) contradict the correct emit — cosmetic fix.

> **Bottom line.** Core folding (O100/O101/O116/O118/O127/O129), DCE
> (O107/O109/O126), structure elimination (O112), tail-call (O121/O122),
> unused-procs (O124), GVN/LICM diagnostics (O105/O106), uplevel inlining,
> static-loop eval, and factory specialisation are at or near parity. The
> blockers for flipping `TCL_LSP_RUST_OPTIMISER`/`_GVN` default-on are: missing
> O128/O130, hint-only O104/O119, the O120/O114 soundness regressions, the O106
> profile-category omission, weaker O103, and the unported general inliner.

## 8. Analyser checks & diagnostics emission

This is the most diagnostic-dense subsystem and the heart of the audit. A
Python-AST-aware extraction over the whole tree (not the one-line `diag(` grep,
which undercounts because `diag(...)` calls routinely span multiple lines) finds
**183 registered codes**: BIGIP(11), E(12), H(1), I(2), IAPP(3), IRULE(33),
O(31, the optimiser codes in §7), P(1, PGO), S(3, in §5), T(5, in §6), TK(3),
W(65), XC(13). The per-code table below is exhaustive over every **diagnostic**
code (E/W/H/I/IRULE/IAPP/TK/BIGIP/XC); S/T/O/P are tabulated in §5/§6/§7.

### Scope
Python: `analyser/` (`semantic_model`, `compiler_checks`, `irules_checks`,
`class_hierarchy`, `mro`, `signature_scan`, `proc_lookup`, `checks/*`,
`_analyser/*`), `compiler/{irules_flow,irules_static_names,class_names}.py`,
`dialects/f5/*`, `shared/codes.py`, `server/features/`.
Rust: `rust/tcl-compiler/src/analyser/*`, `compiler_checks.rs`, `irules_checks.rs`,
`taint.rs`, `interval_bounds.rs`, `path_concat.rs`, `signature_scan/*`, plus
`tcl-registry`, `tcl-lsp-server`, `tcl-lsp-core`.

### Per-code coverage — Errors (E)

| Code | Meaning | Python emitter | Rust emitter | Status | Notes |
|---|---|---|---|---|---|
| E001 | Missing subcommand (bare `string`) | compiler_checks.py:716 | — | ❌ | Deferred (diagnostics.rs:3541 comment). Arity E002/E003 ported; empty/missing-subcommand not. |
| E002 | Too few arguments | compiler_checks.py:929 | diagnostics.rs:3826 | ✅ | Registry-signature driven both. |
| E003 | Too many arguments | compiler_checks.py:930 | diagnostics.rs:3841 | ✅ | |
| E004 | Malformed `if` clause shape | compiler_checks.py:69 | validity.rs:`emit_e004_clause_shape_diagnostic` (registry `commands::tcl::if_::walk_if` hook) | ✅ | Rust exceeds parity: precise per-clause messages/spans, no leading-else FP, no redundant E002 |
| E100 | Syntax | checks/_syntax.py:18 | syntax_checks.rs:691 | ✅ | |
| E101 | Unclosed bracket / missing open brace | _analyser/_utils.py:134 | recovery.rs:310 | ✅ | Same insert-`{` CodeFix. |
| E102 | Syntax | checks/_syntax.py:117 | syntax_checks.rs:756 | ✅ | |
| E103 | Unexpected token / stolen close brace | _analyser/_utils.py:135 | recovery.rs:438 | ✅ | |
| E200 | Shimmer/partial-command parse error | _analyser/_utils.py:133 | state.rs:774 | ✅ | |
| E201 | Unterminated `[` | parsing/recovery.py:43 | segmenter.rs:807 | ✅ | Rust adds ghost-`]` recovery. |
| E202 | Unterminated `"` | parsing/recovery.py:44 | syntax_checks.rs:393 | ✅ | |
| E203 | Unterminated `{` | parsing/recovery.py:45 | syntax_checks.rs:443 | ✅ | |
| E204–E206 | Lexer-warning→error map | parsing/recovery.py:48 | state.rs:776 | ✅ | Same message→code map. |

### Per-code coverage — Warnings (W)

| Code | Meaning | Python emitter | Rust emitter | Status | Notes |
|---|---|---|---|---|---|
| W001 | Unknown subcommand | compiler_checks.py:717 | diagnostics.rs:3611 | ✅ | |
| W002 | Domain | checks/_domain.py:35 | diagnostics.rs:3508 | ✅ | |
| W003 | Expr operator not in dialect | checks/_domain.py:259 | diagnostics.rs:9002 | ✅ | |
| W004 | Command option not in dialect | checks/_domain.py:109 | diagnostics.rs:8942 | ✅ | |
| W100 | Unbraced expr body | _style.py | diagnostics.rs / lib.rs:6595 | ✅ | |
| W101 | Security | checks/_security.py:23 | state.rs:3437 | ✅ | |
| W102 | Security | checks/_security.py:114 | diagnostics.rs:4812 | ✅ | |
| W103 | Security/bounds | checks/_security.py:362 | bounds_checks.rs:1512 | ✅ | |
| W104 | String concat for list building | checks/_style.py:546 | diagnostics.rs:2984 | ✅ | |
| W105 | Unbraced block / missing `variable` in `namespace eval` | checks/_style.py:325 | diagnostics.rs:2552 | ✅ | |
| W106 | Style | checks/_style.py:441 | diagnostics.rs:3096 | ✅ | |
| W108 | Non-ASCII confusable in identifier | checks/_style.py:1412 + _confusables.py | diagnostics.rs:2886 + confusables_table.rs | ✅ | **The "confusables" check.** Same modes (strict for F5/iRules, confusables otherwise). |
| W110 | Use `eq`/`ne` not `==`/`!=` | checks/_style.py:902 | diagnostics.rs:3368 | ✅ | |
| W111 | Line exceeds max length | server diagnostics.py:238 | lib.rs:6101 + source_style.rs | ✅ | LSP-feature layer. |
| W112 | Trailing whitespace | shared/codes.py:289 | lib.rs:6101 | ✅ | |
| W113 | Proc shadows built-in | _analyser/_utils.py:151 | state.rs:1539 | ✅ | |
| W114 | Style | checks/_style.py:1620 | diagnostics.rs:3301 | ✅ | |
| W115 | Backslash-newline in comment swallows next line | shared/codes.py:290 | utils.rs:817 | ✅ | |
| W116 | Stub command shadows built-in | _analyser/_utils.py:156 | scope.rs:975 | ✅ | |
| W117 | Stub expr def shadows built-in | _analyser/_utils.py:157 | scope.rs:981 | ✅ | |
| W118 | Inconsistent line endings | shared/codes.py:291 | lib.rs:6101 | ✅ | |
| W120 | Command without `package require` | shared/codes.py:292 | state.rs:1524 | ✅ | |
| W121 | Style | checks/_style.py:1799 | diagnostics.rs:2865 | ✅ | |
| W122 | Style | checks/_style.py:1878 | diagnostics.rs:8728 | ✅ | Dedup-suppressed when W124 same line (both). |
| W123 | Unresolved command (off by default) | _analyser/_utils.py:198 | handlers.rs:3675 | ✅ | Uses unknown-proc/CHA to gate FPs. |
| W124 | Invalid IP literal | _analyser/_diag_ip.py | diagnostics.rs:7524 | ✅ | |
| W125 | Orphaned control-flow keyword as standalone | _analyser/_commands.py:171 | — | ❌ | Deferred to "C41e5" (commands.rs:12). |
| W126 | Non-channel value in channel arg | _analyser/_diag_channel.py | diagnostics.rs:7212 | ✅ | |
| W127 | compiler_checks | compiler_checks.py:814 | scope.rs:987 | ✅ | |
| W128 | Call after rename/delete → `unknown` | shared/codes.py:293 | state.rs:2749 | ✅ | |
| W130–W134 | tclpkg package-manager diagnostics | shared/codes.py:304-314 | — | ❌ (out of scope) | Package-manager layer; no Rust analyser port. |
| W200 | `exec` result not captured | checks/_style.py:633 | diagnostics.rs:2806 | ✅ | |
| W201 | Path-concat taint | taint/_path_concat.py:38 | path_concat.rs:46 | ✅ | |
| W210 | Variable read before set | _diag_var_lifecycle.py | state.rs:362 | ✅ | |
| W211 | Variable set but never used | _diag_var_lifecycle.py | state.rs:362 | ✅ | See param-trait caveat below. |
| W212 | `$x` where var name expected | checks/_style.py:1111 | state.rs:1983 | ✅ | |
| W213 | Var may not exist — use `unset -nocomplain` | _analyser/_utils.py:167 | state.rs:1621 | ✅ | |
| W214 | Unused proc parameter | _analyser/_utils.py:172 | state.rs:1775 | ✅ | `DynamicNameLocal` is implemented (see §8), so `set $p` verdicts match Python. |
| W215 | Var name unreachable via `$`-subst | _analyser/_utils.py:177 | scope.rs:616 | ✅ | |
| W216 | Broken brace-form array ref `${arr}(x)` | _analyser/_utils.py:182 | state.rs:1983 | ✅ | |
| W220 | Dead store | _diag_var_lifecycle.py | state.rs:1640 | ✅ | |
| W230–W233 | Bounds / div-by-zero | checks/_bounds.py / _diag_interval_bounds.py | bounds_checks.rs / interval_bounds.rs | ✅ | (W233 production-path caveat in §5.) |
| W240 | Loop never enters (const-false) | checks/_bounds.py:643 | bounds_checks.rs:60 | ✅ | |
| W241 | Provably infinite loop | checks/_bounds.py:649 | bounds_checks.rs:69 | ✅ | |
| W242 | Loop termination unprovable (off by default) | checks/_bounds.py:658 | bounds_checks.rs:105 | ✅ | |
| W300–W313 | Security / style / domain family | checks/_security.py, _style.py, _domain.py, taint/_sinks.py | diagnostics.rs (4378–5215), state.rs, taint.rs | ✅ | W302 catch-without-var, W304 missing `--`, W307 non-literal command name, W308 `subst` w/o `-nocommands` all ported (W307 consumes CHA `method_target`). |

### Per-code coverage — Hints / informational (H/I)

| Code | Meaning | Python emitter | Rust emitter | Status |
|---|---|---|---|---|
| H300 | Possible paste error — repeated assignment | _analyser/_utils.py:136 | diagnostics.rs:6271 | ✅ |
| I230 | Constant branch — alternate unreachable | _analyser/_utils.py:141 | diagnostics.rs:7061 | ✅ |
| I231 | Constant switch arm unreachable | _analyser/_utils.py:146 | diagnostics.rs:7032 | ✅ |

### Per-code coverage — iRules (IRULE)

| Code | Meaning | Python emitter | Rust emitter | Status | Notes |
|---|---|---|---|---|---|
| IRULE1001 | Command invalid/ineffective in this event | irules_checks.py:72 | — | ❌ | **High-impact gap.** Registry legality-matrix in Python; only a doc comment in Rust (spec.rs:152), not wired in dispatch. |
| IRULE1002 | `when` references unknown event | checks/_domain.py:443 | irules_event_checks.rs:234 | ✅ | |
| IRULE1003 | Deprecated iRules event | irules_checks.py:524 | irules_event_checks.rs:309 | ✅ | |
| IRULE1004 | `when` missing explicit `priority` | irules_checks.py:557 | irules_event_checks.rs:334 | ✅ | |
| IRULE1005 | `*_DATA` event without matching `*::collect` | irules_flow.py:741 | irules_checks.rs:561 | ⚠️ | Rust anchors span on body entry vs Python `when` token. |
| IRULE1006 | `*::payload` without `*::collect` | irules_flow.py:762 | irules_checks.rs:604 | ✅ | |
| IRULE1007 | `*::collect` without `*::release` | irules_flow.py:777 | irules_checks.rs:618 | ✅ | |
| IRULE1008 | `*::release` without `*::collect` | irules_flow.py:793 | irules_checks.rs:633 | ✅ | |
| IRULE1201 | HTTP command after respond/redirect | irules_flow.py:407 (path-sensitive) | irules_checks.rs:749 (linear) | ❗ | Rust linear scan misses cross-branch cases ("MVP, deferred to C44"). |
| IRULE1202 | Multiple respond/redirect on diff branches | irules_flow.py:391 | irules_checks.rs:733 | ❗ | Same path-sensitivity gap. |
| IRULE2001 | Deprecated `matchclass`→`class match` | irules_checks.py:180 (+quick-fix) | diagnostics.rs:5157 (no fix) | ⚠️ | Rust drops the quick-fix. |
| IRULE2002 | Deprecated iRules command | checks/_domain.py:345 | diagnostics.rs:5126 | ✅ | |
| IRULE2003 | Unsafe iRules command (context escalation) | checks/_domain.py:374 | irules_event_checks.rs:262 | ✅ | |
| IRULE2101 | Heavy `regexp` in hot event | irules_checks.py:251 | irules_event_checks.rs:360 | ✅ | |
| IRULE3001–3003 | Taint sinks (HTTP::respond/header, log) | taint/_sinks.py:56-58 | registry + taint.rs | ✅ | Registry-driven. |
| IRULE3101 | URI-split taint (setter constraint) | taint/_uri_split.py:50 | uri_split.rs / compiler_checks.rs | ✅ | |
| IRULE3102 | Domain — getter form | checks/_domain.py:400 | irules_event_checks.rs:286 | ✅ | Rust getter-form heuristic approximates registry `FormKind::GETTER`. |
| IRULE3103 | URI-split taint | taint/_uri_split.py:55 | uri_split.rs:865 | ✅ | |
| IRULE4001 | Write to `static::` outside RULE_INIT | irules_checks.py:350 | irules_event_checks.rs:403 | ✅ | |
| IRULE4002 | Generic `static::` name (collision) | irules_flow.py | irules_checks.rs:936 | ⚠️ | Rust hardcodes name set, ignores user `generic_variable_patterns`. |
| IRULE4003 | Variable scoping concern across events | irules_checks.py:453 | irules_event_checks.rs:430 | ✅ | |
| IRULE4004 | Constant `set` hoistable to once-per-connection | irules_flow.py:108 | irules_checks.rs:808 | ❗ | Rust narrower: literal-only, hardcoded 9-event list, generic message, no target-event resolution. |
| IRULE4005 | Racy `static::` cross-event | _analyser/_diag_racy.py | diagnostics.rs:8796 | ✅ | |
| IRULE5001 | Ungated `log` in hot event | irules_checks.py:289 | irules_event_checks.rs:381 | ✅ | |
| IRULE5002 | `drop`/`reject` without `event disable all`/`return` | irules_flow.py | irules_checks.rs:232 | ❗ | Rust linear vs path-sensitive; drops quick-fix. |
| IRULE5003 | `while {$v != 0}` decrement misses zero | checks/_style.py:1553 | irules_event_checks.rs:146 | ✅ | |
| IRULE5004 | `DNS::return` without `return` | irules_flow.py:123 | irules_checks.rs:316 | ❗ | Rust linear, no quick-fix. |
| IRULE5005 | Direct proc invocation without `call` | _analyser/_commands.py:228 (ERROR + fix) | — | ❌ | Deferred to "C41d6" (commands.rs:14). |
| IRULE5006 | Top-level-only command in nested body | _analyser/_utils.py:152 | irules_event_checks.rs:178 | ✅ | |
| IRULE5007 | Event-context command at top level | _analyser/_utils.py:153 | irules_event_checks.rs:200 | ✅ | |
| IRULE6001 | Global namespace variable (TMM pinning) | irules_checks.py:640 | irules_event_checks.rs:487 | ✅ | Both attach quick-fix. |

### Per-code coverage — F5 dialect subsystems (Tk / BIG-IP / iApp / XC)

| Code | Meaning | Python emitter | Rust emitter | Status | Notes |
|---|---|---|---|---|---|
| TK1001–1003 | Tk geometry/widget/option checks | analyser/checks/tk.py:35-37 | — | ❌ | No Rust port; `tk_detection.py` has no Rust counterpart. |
| BIGIP6001–6011 | BIG-IP iApp/config validator | dialects/f5/bigip/validator.py:36-46 | — | ❌ | `tcl-bigip/lint.rs` emits unrelated `Finding`s, not BIGIP6xxx codes. |
| IAPP7001–7003 | iApp template validation | dialects/f5/bigip/iapp_diagnostics.py:28-30 | — | ❌ | No Rust port. |
| XC100–301 | BIG-IP→F5-XC iRule **translator** labels (all `internal=True`) | dialects/f5/xc/translator.py:81-93 | — | ❌ | Translator diagnostics, not analyser checks; no Rust translator. |

### Parity table — orchestration, OO/MRO, signature scan, iRules flow, incremental

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Check orchestration | _analyser/_core.py (CST walk) + compiler_checks.py + checks/_orchestrator.py (IR pass) | state.rs + commands.rs (inlined into walk) + compiler_checks.rs (CFG/SSA only) | ⚠️ | Python: separate IR pass with trait-keyed `_TRAIT_CHECK_MAP`. Rust: per-command checks folded into the single CST walk. Output-equivalent; structurally different. |
| Command→handler dispatch | _commands.py if/elif | commands.rs:339 bool-handler chain | ✅ | Faithful. |
| Diagnostic dedup | _core.py:719 | diagnostics.rs:8693 | ✅ | Identical key + line-suppression rules. |
| Diagnostic ordering | unsorted (emission order) | sorted by `(span,code,severity,message)` | ❗/⚠️ | Rust sorts to make `analyse`==per-item byte-identical; Python emits in order. **Confirm the differential oracle compares as a set, not ordered.** |
| Disabled-code / noqa / file suppression | per-emitter guards + noqa | centralised `apply_disabled_diagnostics` | ✅ | |
| Snapshot / chunked re-analysis | _analyser/_snapshot.py | analyser/snapshot.rs | ✅ | Rust keys scopes by index-path; Python by `id()`. Benign. |
| Per-item incremental | — (none) | per_item.rs, item_tree.rs, state.rs | ➕ | Rust-ahead: offset-stable item identity + per-body memoisation, fuzzer-gated byte-identical to `analyse`. |
| MRO algorithm | mro.py:38 `_tcloo_dfs` (two-pass DFS + late placement) | mro.rs:91 | ✅ | **Not C3** — both implement TclOO's `tclOOCall.c` two-pass DFS (correct for TclOO). |
| Class hierarchy (CHA) | class_hierarchy.py | class_hierarchy.rs | ➕ | Parity + Rust defensive backfill/re-linearise. Non-deterministic `errors` order only (benign). |
| TclOO construct coverage | _analyser/_oo.py (full) | analyser/oo.rs, handlers.rs | ✅ | Stale-note fix: the three gaps previously listed here (`oo::class` dropping `new`/`createWithNamespace`; `initialise` body not walked; `property -get/-set` accessor bodies not walked) are all covered — see `oo_class_create_with_namespace_is_recognised`, `oo_initialise_body_is_walked`, `oo_property_accessor_bodies_are_walked` in `oo.rs`. |
| snit support | _oo.py:492-790 + class_names.py | — | ❌ | **Largest OO gap** — no snit handling in the Rust analyser. |
| Signature discovery | analyser/signature_scan.py | signature_scan/* | ✅ | Full parity (proc/namespace/import/package/source/interp-alias/oo::class/itcl::class, conditional recursion, tcllib factory resolver). |
| proc lookup / reference matching | analyser/proc_lookup.py | signature_help.rs:322 | ✅ | Identical 3-way name predicate. |
| Param traits | proc_arg_traits.py (7 traits incl `DYNAMIC_NAME_LOCAL`) | `ProcArgTrait` (analyser/types.rs, 8 variants incl. `DynamicNameLocal`) | ✅ **RESOLVED (2026-08-04)** | `DynamicNameLocal` is a real variant, deliberately distinct from `VarWrite`/`VarRead` (see the doc comment at its definition and PR #498/#499), and is emitted for `set $p 1`/`scan`/`lassign`/`regsub` callee-local dynamic-name uses in `param_traits.rs`. Pinned by `lassign_records_dynamic_name_local` and siblings in `param_traits.rs`. Rust is additionally dialect-aware (➕). |
| Connection scope | connection_scope.py | connection_scope.rs | ⚠️ | Rust omits branch-condition v0 sweep. |
| Event-ordering model | irules_flow.py:1545 `extract_event_order` | — | ❗ | Not ported (the 1002/1003/1004 checks themselves are at parity). |
| Static names | irules_static_names.py (configurable) | inlined `GENERIC_STATIC_NAMES` | ⚠️ | Default-config equivalent; user patterns not plumbed. |

### Gaps (Rust missing or weaker)
**Codes with NO Rust emitter (analyser-relevant):** E001, W125 (C41e5), IRULE1001 (high-impact, frequently-firing), IRULE5005 (C41d6), TK1001–1003, BIGIP6001–6011, IAPP7001–7003, XC100–301, W130–W134 (tclpkg, out of scope).
**Weaker behaviour (emit but diverge):** IRULE1201/1202/5002/5004 (linear vs path-sensitive; dropped quick-fixes); IRULE4004 (narrower); snit unported; connection_scope branch-condition sweep; event-ordering model; configurable name patterns. *(Param-trait `DynamicNameLocal` and the three OO-body-walk gaps this line used to list are **RESOLVED** — see the parity table above.)*

### Rust-ahead / divergent-by-design
- Per-item incremental analysis (offset-stable item identity + per-body memoisation for salsa early-cutoff); no Python equivalent.
- Deterministic output ordering (sorts diagnostics by source position).
- CHA backfill/re-linearise pass with error de-dup.
- Dialect-aware param-trait inference + `StubOverlay` participation.
- Ghost-`]` recovery for swallowed-command recovery.

### Algorithmic / data-structure differences (benign)
- MRO: TclOO two-pass DFS + late placement on both (not C3) — byte-equivalent for diamonds/mixins/cycles; only `errors` iteration order differs.
- Orchestration shape: separate IR pass + trait table (Python) vs single CST walk with inlined emitters (Rust).
- Snapshot scope keying: `id()`+remap (Python) vs index-path (Rust).
- Static-name sets: regexes (Python) vs pre-expanded constant set (Rust) — identical for default config.

### Open questions for maintainer
1. **Output-ordering oracle** — does the Python↔Rust differential test compare diagnostics as an unordered set or an ordered list? Rust sorts by position; if the oracle is ordered this is the one place strict byte-equivalence breaks for the ported codes.
2. **Deferred-code roadmap** — E001, W125, IRULE5005, IRULE1001 are tagged deferred. Scheduled or dropped? IRULE1001 is a core, frequently-firing check.
3. **F5 dialect subsystems** (TK/BIGIP/IAPP/XC ~30 codes) — intended to stay Python-only, or should the Rust analyser eventually emit them?
4. ~~**`ProcArgTrait::DynamicNameLocal`** — confirm intent to port.~~
   **Answered (2026-08-04):** already ported — see the parity table above.
5. **iRules path-sensitivity (C44)** — IRULE1201/1202/5002/5004 are linear MVPs; is the path-sensitive walker (and the dropped quick-fixes) on the roadmap?
6. **snit support** — intended for the Rust analyser or out of scope?

---

## 9. Bytecode codegen

### Scope
Bytecode codegen only (WASM emitter excluded — known in-progress). Python:
`compiler/codegen/bytecode/{*.py, bytecoded/*.py}`. Rust:
`rust/tcl-compiler/src/codegen/{*.rs, emitter/*.rs}` + the shared artifact crate
`rust/tcl-bytecode/src/{format,layout,lib}.rs`.

**Differential harnesses (both present):** `bytecode-compare` skill (Python vs
real tclsh 8.4–9.0, 219 snippets) and `rust/tcl-compiler/tests/differential_codegen.rs`
(Rust vs Python oracle, Exact/Semantic/Divergent classification over
`tests/fixtures/codegen/`). The latter is the authoritative parity gate.

### Parity table (foundation)

| Item | Python loc | Rust loc | Status | Notes |
|---|---|---|---|---|
| Opcode set (155 ops) | opcodes.py:61-219 | tcl-bytecode/lib.rs:105-257 | ✅ | Exact 1:1, identical order (verified). |
| Mnemonics + sizes | opcodes.py:222-377 | lib.rs:264-628 | ✅ | |
| LVT-op / jump-op classification | opcodes.py:380-413 | lib.rs:422-458 | ✅ | |
| BinOp/UnaryOp → Op maps | opcodes.py:416-462 | lib.rs:632-682 | ✅ | All 34 BinOps + 5 UnaryOps incl. iRules ops. |
| Interning tables (literal/LVT) | _types.py:37-103 | lib.rs:769-864 | ✅ | Same intern/dedup; LVT name-identity (no slot coalescing) both. |
| `parse_tcl_index` / `INDEX_END` / str-class ids | opcodes.py:11-58 | lib.rs:26-100 | ✅ | |
| Layout: jump-size opt + label resolution | layout.py | layout.rs | ✅ | Iterative 4→1 jump shrink (max 10), same skip rules. |
| Disassembly format (flat text) | format.py:50-151 | format.rs:60-275 | ✅ | Byte-identical by construction. |
| `_esc`/`esc` literal escaping | format.py:13-47 | format.rs:16-48 | ❗ | Two divergences: astral codepoints >U+FFFF (Python `\UXXXXXXXX`; Rust `\u{…}`) and raw C0 controls other than `\0\n\t\r\v\f` (Python escapes; Rust passes through). Latent if corpus is ASCII. |
| Peephole passes | _peephole.py | peephole.rs | ✅ | Full parity (trailing-pop, const push/pop nop fold, dedup push, tail-return→done, start-cmd fixups, NO_DEDUP strip). |
| Emitter linearise/RPO, loop reorder, proc-def emission | _emitter.py:180-396 | emitter/{ordering,proc_defs}.rs | ✅ | Faithful; benign VecDeque-vs-pop(0) / byte-vs-line ordering key. |
| Terminator (goto/branch/return), jump-table | _expressions.py:330-499 | emitter/terminator.rs | ⚠️ | Missing `![cond]` negation-flip; `RETURN_IMM` level operand differs (Py `0,1` vs Rust `0,0`). Flagged for tclsh confirmation. |
| Expression constant folding | _expressions.py:51-65,149-168 | (absent in `emit_expr`) | ❗ | Python folds `expr {1+2}` in codegen via `eval_tcl_expr`; Rust does not (and the codegen test pipeline doesn't run the optimiser). Tracked in divergent corpus. |
| Expr nodes (literal/string/var/binary/unary/ternary/raw/call) | _expressions.py:68-216 | expressions.rs:27-213 | ✅ | `ExprCommand` nested inside an operator tree is a placeholder in Rust; the `[expr {…}]` body path works. |
| cmd_subst parsing/template helpers | _cmd_subst.py / _helpers.py | cmd_subst.rs / helpers.rs | ⚠️ | Faithful; `parse_subst_template` preserves the conservative `None`-bailout. Two Rust-ahead extensions (namespace `:` in bare vars; `consume_array_index`). |
| `{*}` expansion in cmd-subst | _cmd_subst.py:148-265 | cmd_subst.rs:207 | ❗ | Rust `parse_cmd_parts` drops the `expand` flag; statement-level `{*}` handled separately but without Python's literal-name strip / `list {*}` specialisation. |
| `builtin_is_trusted` rename gate | _bytecoded.py:40 | (absent) | ❗ | No equivalent gate; confirm a redefined builtin can't be wrongly inline-specialised (may be moot if `registry.resolve_call` excludes tampered builtins). |
| Conservative fallback-to-generic-invoke | _bytecoded.py / _helpers.py | bytecoded.rs / helpers.rs | ✅ | Preserved; `parse_subst_template`→`None` bailout intact. |

### Per-command bytecoded specialisation — the systemic gap

Python dispatches all specialisations at **statement position** (`_bytecoded.py::_try_bytecoded`, ~22 hooks). Rust splits into (a) 7 `CodegenHookId` variants, (b) structured IR lowering, and (c) **value-position-only** `[cmd …]` inlining. The systemic divergence: many commands Rust specialises only in value position fall through to a generic `invokeStk` as a bare statement, where Python (matching tclsh) bytecodes them.

| Command/group | Status | Notes |
|---|---|---|
| `llength`/`lassign`/`lrange`/`linsert`/`lset` | ✅ | Dedicated `CodegenHookId` + value-pos. |
| `incr`, `set x v` assign | ✅ | Structured lowering. |
| `dict get/set/incr/append/lappend/unset` (proc) | ⚠️ | Proc-context match; value-arg interning differs slightly. |
| `info exists`, `lindex`, `lreplace`, `list`, `regexp`, non-proc `dict get/create` | ❗ | **Value-position only**; bare statement → generic invoke. |
| `string …` (length/index/range/equal/compare/match/toupper/…/trim*/cat/first/last/is) | ❗ | **Largest gap** — no statement hook; value-pos missing `toupper/tolower/totitle/trim*/match/first/last/cat`. `STR_UPPER/LOWER/TITLE/TRIM/FIND/RFIND` have 0 emit-sites. |
| `append`/`lappend`/`unset`/`upvar`/`global`/`tailcall`/`concat` | ❌ | `APPEND_*`/`LAPPEND_*`/`UNSET_STK`/`UPVAR`/`NSUPVAR`/`TAILCALL`/`CONCAT_STK` all **0 emit-sites** (verified). Always generic invoke. |
| `dict exists` / `INVOKE_REPLACE` forms / `subst` inline | ❌ | No Rust path. |
| `set x [cmd]` pure cmd-subst, `set $x v` indirect, `set x` read-form | ❗ | Statement assign routes through `emit_value_interpolated`, which pushes the raw `[foo bar]` literal for a lone `[cmd]` instead of evaluating. |
| `for`/`while`/`foreach`/`switch`/`if`/`proc`/`expr`, `catch`/`try` | ✅/⚠️ | Structured on both; "frozen for/while" + `namespace eval` shapes unverified vs tclsh. |

### Gaps (Rust missing or weaker)
1. **`append`/`lappend`/`unset`/`upvar`/`global`/`tailcall`/`concat` never specialised** (0 emit-sites, verified). Always generic `invokeStk`.
2. **`string` subcommand specialisation is the single largest gap** — no statement hook; value-pos missing many arms.
3. **Statement-position under-specialisation generally** (`lindex`/`lreplace`/`list`/`regexp`/`info exists`/non-proc `dict`). 7 `CodegenHookId` variants vs ~22 Python statement hooks.
4. **Expression constant folding absent in Rust codegen.**
5. **`set x [cmd]`, `set $x v`, `set x` read-form, nested `set x [set y …]`** not handled at statement assign.
6. **`esc` Unicode/control divergence**; **`{*}` cmd-subst expansion dropped**; **`builtin_is_trusted` gate absent**; **`![cond]` negation-flip** and **`RETURN_IMM` level operand**; `dict exists`/`INVOKE_REPLACE`/`subst` inline absent.

### Rust-ahead / divergent-by-design
- `LINDEX_MULTI` for `[lindex $l i j]` (Python's `lindex` only handles 2-arg).
- `string replace 0 N` fast-path (Python routes to generic invoke).
- `Instruction.push_verbatim`/`foreach_vars` carried out-of-band while keeping disassembly identical.
- `invokeStk1→invokeStk4` promotion when argc≥256 (robustness).
- Exhaustive enum matches (no stringify fallback).

### Algorithmic / data-structure differences (benign)
- Python `_Emitter` mixin split into focused modules (documented intentional).
- `Vec+HashMap` interners vs `list+dict`; `VecDeque` pending-proc queue vs `list.pop(0)` — same emission order.
- Structured explorer JSON view lives in a separate crate (`_NO_PARITY`, test-pinned), outside the flat-text byte-diff scope.

### Open questions for maintainer
1. Are the statement-position specialisation gaps tracked work or regressions? Only `expr-radix-fold` + `if-catch-cond` are in the divergent corpus, yet the 0-emit-site opcodes imply far broader divergence. Suggest adding fixtures for bare-statement `string toupper`, `append`, `unset`, `upvar`, `tailcall`, `concat` and running `bytecode-compare summary`.
2. `RETURN_IMM` level operand for top-level `return value` — Python is internally inconsistent (`0,1` vs `0,0`); Rust uses `0,0`. Which matches tclsh 9.0?
3. Does the byte-diff corpus contain raw C0 controls / codepoints >U+FFFF (the `esc` divergence)?
4. Does `registry.resolve_call` already exclude renamed/redefined builtins, making the missing `builtin_is_trusted` gate moot?
5. Is routing statement `AssignValue` through the fuller `emit_value(value, true)` the planned remedy for the single-`[cmd]` inline gap?

> **Bottom line.** The codegen *foundation* is at strong parity — opcode set,
> serialised format, layout/jump-shrinking, peephole, disassembly text, block
> linearisation, and the conservative fallback contract are faithful 1:1 ports
> gated by a real differential harness. The meaningful divergences concentrate
> in the **per-command specialisation layer**: Rust specialises mostly in value
> position, leaving statement-position `append`/`lappend`/`unset`/`upvar`/
> `global`/`tailcall`/`concat`/`string`/`regexp`/`lindex`/`lreplace`/non-proc
> `dict` as generic invoke, plus the absent codegen-level constant folding.

## Consolidated prioritised gap register

Every meaningful divergence (status ❗ / ❌), ranked. ✅/⚠️/➕ rows from the
sections are not repeated. "Disposition" is a *suggestion* for the maintainer,
not a decision.

### P0 — Soundness / correctness (can produce a wrong answer; gate the shim flips)

> **Reconciliation note (2026-06-19, post-publication).** Two P0 rows were
> re-checked against *landed* Rust work: **#1 (O120) was briefly withdrawn then
> re-confirmed OPEN** after a Codex review (PR #650) — the numeric gate
> (`node_provably_numeric`, `optimiser/helpers/expr_simplify.rs:97`) exists but
> guards only the operand-dropping identities; the string-compare promotion
> `streq_promote_node` (`:1040`, called from `simplify_node_once` `:408` and
> `try_strength_reduce_expr_typed` `:695`) takes **no** `NumericCtx` and fires on
> any `==`/`!=` with a `String` operand, so `$x == "1"` → `$x eq "1"` is still
> unsound. **#11 (DynamicNameLocal) is downgraded to a conflict** — Rust
> `param_traits.rs:479-482` *does* mark `scan`/`lassign`/`regexp`/`regsub`
> out-vars `VarWrite`, and `docs/rust-rewrite.md:2019` argues the trait is
> intentionally-not-ported-and-benign; needs reconciliation, not a confirmed
> regression. The authoritative live open-work set is maintained in
> [`../../rust-rewrite.md`](../../rust-rewrite.md).
>
> **Revalidation (2026-08-04, issue #1189).** Re-checked against `rust` tip
> `efe3dd9566dfc593bf875e95a03d1b55fabbb95c`: **#4, #5, #7, #8, and #11 are
> RESOLVED** — code, tests, and (for #7/#8) doc comments now agree, with no
> remaining conflict. #11 specifically: `ProcArgTrait::DynamicNameLocal`
> (`analyser/types.rs`) is a genuine, distinct, tested variant — the 2026-06-19
> note's "conflict" framing undersold it; it is not merely benign-by-omission,
> it is actually implemented. See each row below and the memory-SSA / SSA /
> SCCP sections (§4-§6) for the current-code citation. #1 (O120) was
> re-checked and remains genuinely open — see its row.

| # | Subsystem | Gap | Evidence | Disposition |
|---|---|---|---|---|
| 1 | Optimiser §7 | **O120** string-compare `==`→`eq` promotion fires on any `String` operand with **no non-numeric proof** → `$x == "1"` → `$x eq "1"` flips the result when `$x` is numeric | `streq_promote_node` (`optimiser/helpers/expr_simplify.rs:1040`, no `NumericCtx`); called `:408`/`:695`. Python gates both operands via `_is_provably_non_numeric_expr_node` (`_expr_simplify.py:484`) | Gate the promotion on provably-non-numeric operands (mirror Python's D5-O120). |
| 2 | Optimiser §7 | **O114** incr idiom rewrites `set x [expr {$x+N}]`→`incr x N` gating only the literal, not `$x` numericity → suggests a rewrite that errors on float `$x` | pattern_recognition.rs:219 (no var gate) | Confirm whether Python gates the variable; add the gate if so. |
| 3 | Optimiser §7 | **O108** ADCE treats every assignment as side-effect-free → can delete `set x [impureCmd]` | elimination.rs:740 | Verify against landed RHS-purity gate; restore if genuinely dropped. |
| 4 | Value/SCCP §5 | ~~SCCP omits **escaping-var widening**~~ **RESOLVED (2026-08-04)** | `evaluate_def` (sccp.rs:300-335) now consults `var_observability::analyse_var_observability(..).escaping_var_names()` and forces every escaping def to `Overdefined` | Closed. |
| 5 | Value/SCCP §5 | ~~SCCP omits **break-edge reachability**~~ **RESOLVED (2026-08-04)**, via a real CFG edge rather than an SCCP precompute | `escaping_loop_jumps`/`break_target` (cfg_builder); pinned by `rch_while1_with_break_post_loop_is_reachable` | Closed. |
| 6 | Lexer §1 | ~~**`${a\}b}` / `${a{b}c}`** braced-var scan stops at first `}`~~ **DONE (FE-LEX, 2026-06-19)** — `parse_var` + `scan_dollar_brace` track inner-brace depth and consume `\X` (9.0.3 reference; verified against `tclsh9.0`) | lexer.rs `parse_var` | Landed; see history. |
| 7 | Memory-SSA §4 | ~~**upvar transitive-merge** divergence~~ **RESOLVED (2026-08-04)** — `compute_aliases`'s union-find merges `upvar 1 x a; upvar 1 x b` into one alias set; code/test/doc now agree | memory_ssa.rs:533-593; `compute_aliases_merges_shared_caller_upvars` | Closed. |
| 8 | Memory-SSA §4 | ~~**`IRUpFrame` not a clobber**~~ **RESOLVED (2026-08-04)** — `is_clobber` matches `Statement::Barrier { .. } \| Statement::UpFrame { .. }` | memory_ssa.rs:437-439; `build_memory_ssa_emits_clobber_for_upframe` | Closed. |
| 9 | Shimmer §5 | **S100 severity** emitted as Warning, not Information; **phi-S101 / expr-S100** code-strings ignore `in_loop` | compiler_checks.rs:113; phi.rs:146; expr.rs:273 | Map S100→Information; compute code from `in_loop`. *(Not re-verified 2026-08-04.)* |
| 10 | Lowering §3 | dynamic `uplevel $body` lowers to generic **Call** not **Barrier** (no `_DYNAMIC_BARRIER_COMMANDS`, `uplevel_.rs` lacks `ArgRole::Body`) → downstream treats it as analysable | mod.rs:1397/1609 | Add a Body role / barrier fallback. *(Not re-verified 2026-08-04.)* |
| 11 | Analyser §8 | ~~**`DynamicNameLocal`** trait absent~~ **RESOLVED (2026-08-04)** — it is a real, tested `ProcArgTrait` variant, not merely a benign omission | analyser/types.rs (definition); param_traits.rs (emission + tests) | Closed. |
| 12 | Value/type §5 | expr-context literal typing wrong (`0o`→INT/unknown→NUMERIC) and `~$double`→INT | type_infer.rs:128/197 | Use a distinct expr-context classifier. |

### P1 — Missing user-facing capability (a diagnostic or transform the user loses)

| # | Subsystem | Gap | Evidence | Disposition |
|---|---|---|---|---|
| 13 | Analyser §8 | **IRULE1001** (command invalid/ineffective in event) not emitted — high-impact, frequently-firing | spec.rs:152 (doc only) | Wire the registry legality-matrix check. |
| 14 | Analyser §8 | **snit** OO support entirely absent (`snit::type`/`widget`/`widgetadaptor`) | no Rust counterpart to _oo.py:492 | Port if snit is in scope. |
| 15 | Analyser §8 | F5 dialect diagnostics **TK1001-1003, BIGIP6001-6011, IAPP7001-7003, XC100-301** (~30 codes) have no Rust emitter | dialects/f5/* unported | Decide Python-only vs Rust port. |
| 16 | Analyser §8 | **E001**, **W125**, **IRULE5005** deferred (no Rust emitter) | diagnostics.rs:3541; commands.rs:12,14 | Schedule or document as dropped. |
| 17 | Analyser §8 | iRules flow **IRULE1201/1202/5002/5004** linear-scan MVPs (miss cross-branch); quick-fixes dropped on 5002/5004/2001 | irules_checks.rs:749/733/232/316 | Port path-sensitive walker (C44) + restore quick-fixes. |
| 18 | Optimiser §7 | **O128** (end-offset index) and **O130** (lappend chain) unimplemented; **O104**/**O119** hint-only | pattern_recognition.rs; profiles.rs:63,78 | Implement applicable rewrites before default-on. |
| 19 | Optimiser §7 | General proc inliner (`compiler/inlining/`, ~1900 LOC) unported | no Rust `inline_module` | Decide scope (codegen/WASM vs LSP). |
| 20 | Codegen §9 | Statement-position specialisation of `append`/`lappend`/`unset`/`upvar`/`global`/`tailcall`/`concat`/`string`/`regexp`/`lindex`/`lreplace`/non-proc `dict` → generic invoke (0 emit-sites verified) | grep: APPEND_*/LAPPEND_*/UNSET_STK/UPVAR/TAILCALL/CONCAT_STK/STR_UPPER… | Add statement-position hooks; add divergent-corpus fixtures. |
| 21 | Codegen §9 | Codegen-level expression constant folding absent (`expr {1+2}`) | emit_expr (no fold) | Tracked in divergent corpus. |
| 22 | Value/type §5 | TclOO/snit object typing (`[Foo new]`→OBJECT) + scope-alias OVERDEFINED widening missing → shimmer FPs, weaker W307/W308 | type_infer.rs:91 | Port object/alias typing. |
| 23 | Taint/IPA §6 | `var_escape` ported but **unwired** (no orchestrator); missing `pure_leaf` family + precision regressions | var_escape/* (tests only) | Wire driver + port `pure_leaf` when inlining/DCE consumers land. |
| 24 | Value/SCCP §5 | **W233** production path is SCCP-constant-only (interval `find_divide_by_zero` dead) → misses interval `[0,0]` divisors | interval_bounds.rs:642 | Wire the interval path, or document the simplification. |
| 25 | Bounds/Rendered §5 | rendered-props ESC numeric/hex escape rendering weaker (`\x2f`→`/`) → W201 false-negatives | rendered_properties.rs:437 | Port the escape rendering. |

### P2 — Cross-cutting structural / robustness

| # | Subsystem | Gap | Evidence | Disposition |
|---|---|---|---|---|
| 26 | SSA §4 / Taint §6 | ~~**`complexity_guarded` skip absent everywhere**~~ **RESOLVED (2026-08-04)** — `is_complexity_guarded` + `DEEP_ANALYSIS_BODY_BYTES` gate `FunctionUnit` construction; a guarded proc's taint/SSA/SCCP are all trivial and it is filtered from interprocedural summarisation | ssa.rs:309-323; compilation_unit.rs:618-637,962-970,1661,1702,1734 | Closed. |
| 27 | IR/WASM §3,§7 | WASM-codegen passes unported: `IRInterpBoundary`+`passes/interp_boundaries.py`, `passes/dce.py`, `passes/gvn.py`, `source_inliner`, `stdlib_prelude` | absent | Expected while WASM codegen is in progress; track. |
| 28 | Analyser §8 | Diagnostic **ordering**: Rust sorts by position, Python emits in order — confirm the differential oracle is set-based | diagnostics.rs:8747 | Confirm oracle semantics. |
| 29 | IR §3 | Taint + CompilationUnit built **2–3×** per document (perf) | compilation_unit.rs:206,584,596 | Build once, share `&CompilationUnit`. |
| 30 | Interproc §6 | **`TCL_LSP_RUST_INTERPROC` gate is inert** (named but never called; binding unused) | rust_shim_enabled | Wire it or update the docs (they read as live). |
| 31 | Memory/CFG §4 | `break`/`continue` not gated on `faithful_exceptions` → codegen-path CFG gains goto edges (byte-identical-to-tclsh risk) | cfg_builder/mod.rs:261 | Confirm codegen uses `build_cfg_codegen`. |
| 32 | Segmenter §2 | Incremental reparse far weaker (prefix-only; no braced-body interior splice) — likely OK if salsa owns per-file incrementality | segmenter.rs:285 | Confirm by-design. |
| 33 | Various §3,§6 | Smaller field/precision drops: `IRBlock.caller_scope`/`source_args`, `IRForeach.tokens`, `IRProcedure` inlining fields; `proc_fingerprint`; O103 namespace-chain/rename gating; side_effects protocol-NS classifier; var_refs read-before-set helpers; O106 missing from `OPT_CATEGORIES` | per-section | Triage individually. |

### Suggested sequencing

> **2026-08-04 note:** #4, #5, #7, #8, #11, and #26 (referenced below as
> already-closed prerequisites) are **RESOLVED** — see their rows above. The
> sequencing below is otherwise as originally written and has not been
> re-verified beyond those six items.

1. **Before flipping `TCL_LSP_RUST_OPTIMISER`/`_GVN` default-on:** close the
   remaining open P0 soundness gates — **#1** (O120, confirmed still open) and
   **#18** (O128/O130/O104/O119). #4/#5 (the other former SCCP soundness
   gates in this range) are already closed.
2. **Before retiring the Python analyser fallback:** close #6, #9 (lexer
   brace-var — already done — and shimmer severity/codes) and decide #13–#17
   (IRULE1001, deferred codes, F5 subsystems, snit). #11 (`DynamicNameLocal`)
   is already closed.
3. **Independent of flips:** #28 (ordering-oracle confirmation) is a
   low-effort, high-value robustness fix. #26 (`complexity_guarded`) is
   already closed.
4. **WASM-path (#27, #20, #21):** sequence with the Rust WASM codegen work;
   out of the LSP critical path.

## Related
- [`review-findings.md`](review-findings.md) — workspace-wide correctness /
  performance / memory review (C1, C2, the one-tree-rebuilt cost).
- [`current-architecture.md`](current-architecture.md) — crate graph, ownership
  rules, authoritative paths, and the live `TCL_LSP_RUST_*` shim gates.
- [`rust-rewrite-registries.md`](../../../rust-rewrite-registries.md) — the
  companion registry-data parity audit.
- The per-subsystem design docs under [`docs/design/compiler/`](../compiler/)
  describe the intended algorithms each section was checked against.
