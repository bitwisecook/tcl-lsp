# Sol-3 review: fix-1-nested-body-cut-memo.patch

- Candidate: `luna-workspace/fix-1-nested-body-cut-memo.patch` (uncompiled)
- Base: `work` at 2a3928bc6. The five touched files are byte-identical at 4c3eb7953 and the patch applies cleanly.
- Method: source reading only. The patch was applied to scratch copies under `sol-3/mini/`, and `rustfmt --edition 2024 --check` was run on those copies. No cargo, tests or gdb were run.
- Line numbers below refer to the patched copies in `sol-3/mini/rust/...` unless they are marked HEAD.

## Verdict: ADMIT (source level), with conditions

I found no input on which the memoised walk gives a different first cut, term, reporting anchor, cut order, unavailable-ownership result or nested-validation outcome than the unpatched walk. The wiring is limited to `OriginalSourceProjectionMemo`. Every other caller keeps the old path. Compile risk is low, but one formatting failure is certain (F1). Before commit, the patch needs to clear the conditions C1 to C4 below.

## 1. Exactness

The walk of a body is a pure function of three things:

1. The body's bytes. `body_frame` (parse_cut.rs:858) lexes only `arena.bytes(body)`, and that is `image.bytes()[span]` (executable_parts.rs:226, HEAD).
2. The drain `config`. `NestedScript.config(config)` is used for lexing and the drain `config` for `content_part_frame`.
3. The `channel`.

`base` and `report_at` affect only the offsets of a cut, never whether a cut exists. A clean result therefore depends only on (bytes, config, channel).

- **Key soundness (parse_cut.rs:347, :375-391, :415-424).**
  - The span is computed as `origin + base + body`. The `base` chain is consistent: a nested `Words` frame's arena is built over the body image with `base = child_base`, and the `Parts` frames of variable indexes share that arena and base.
  - The `anchored` check proves that `local == whole[origin..]` and that the channels are equal. It is O(region), and it always passes for the only caller, because `local` is built from `whole[region]` (native_script_words.rs `local_region_tokens`).
  - `bound_to` pins one image per memo. In practice the memo is owned per `SourceOriginKind` allocation (command_binding.rs:650-678, HEAD), so the image is always `self.source_image()`. `Arc<[u8]>` equality short-circuits on pointer equality.
  - With the whole image fixed, the span fixes the bytes. `config` is the normalised drain config, so `base_offset`, `base_line` and `base_col` are 0, and every dialect axis (`braced_var`, `quote_termination`, `escapes`, and so on) is part of `LexerConfig`'s `Hash`/`Eq`. I found no key collision.
- **Clean-only recording (parse_cut.rs:701-708).**
  - A body is recorded only when its `Words` frame runs out of jobs. Because of stack discipline, every descendant frame has been popped by then.
  - A cut returns early, and that includes a `Parts` fallback, which returns at pop. An `Err` (unavailable geometry or lexical stream) also returns early.
  - So bodies with a cut and unavailable bodies are never recorded, and the unchanged walk always revisits them. Skipping a clean body is the same as pushing it, walking it without result and popping it.
- **Context independence.**
  - Quoting or brace context: the enclosing context decides only where the body ends, and the extent is the key. `report_at` inside `$a([...])` affects only offsets.
  - `ParseCutDescent::EnclosingScript` never reaches the Command branch. The memo entry point hard-codes `RecursiveCommands`.
  - The top-level region frame has `clean_key: None`, so a region is never confused with a body.
- **Depth guards.**
  - `MAX_SOURCE_NEST_DEPTH` controls `walk_source` above the capture, not the parse-cut walk. The parse-cut walk is deliberately unbounded (HEAD parse_cut.rs `Frame` docs).
  - The lexer's and structural index's caps (`MAX_NESTED_BRACKET_DEPTH`, the array-index depth) count depth within the buffer being scanned, which here is the body itself. They do not count native stack depth or global state. tcl-lexer has no `stacker`, `thread_local` or global atomics.
  - So a body that is clean at walk depth d is clean at any walk depth d'.
- **Omitted state.** The walk changes only its local stack. When a body is skipped, recording of its descendants is skipped too, but those descendants were recorded before the body itself (inner bodies finish first). Nothing observable is lost.
- **Theoretical gap (Low, unreachable).** Suppose `arena.bytes(body)` returned `None` for a span that an earlier walk had recorded as clean under the same whole-image key. The unpatched walk would return `Err(SourceBounds)` and the memo would skip instead. This cannot happen, because arena body spans always lie inside their own image. I record it only for completeness.

## 2. Scope and reuse

- **Wiring.** Only `OriginalSourceProjectionMemo::capture` passes `Some(&self.nested_cuts)` (original_source_projection.rs:114-120).
  - `OriginalSourceCommandProjection::capture`, `original_native_compiler_words` (about 10 callers), `native_script_words_in{,_with_purpose}` and `reborrow_command_substitution_plan` (which passes `None` on the InheritedCommand path) keep the unpatched walk.
  - The test-only call at parse_cut.rs:1529 is unchanged.
- **`reborrow_command_substitution_plan` (0597b94c5) is not a substitute here.**
  - It works top-down: it needs the parent's recursively checked `NativeScriptWordsPlan` and `NativeWord`, then skips validation (`InheritedCommand`).
  - Captures happen bottom-up. `record_evaluated_source_point` runs after `prepare_source_arguments` → `walk_substitutions`, which matches the gdb samples, where the depth falls from 41 to 23. At the time an inner capture runs, its parent's plan does not exist yet.
  - The capture is also keyed by (origin, offset, config, word vector) and requires a single-command region, which a reborrowed body plan does not provide.
  - Reusing it would mean threading parent plans through `SourceExecutionContext` and changing the capture contract. The new type is justified. It also lives in the correct owner (`parse_cut.rs`, the parse-error-cut owner) and reuses the existing per-origin memo lifetime.
- **Contract row.**
  - `cargo xtask owner-resolution` will not fail. The manifest is row-driven and `parse_cut.rs` is the owner path.
  - However, AGENTS.md § Shared semantic owners requires the contract to be updated for an owner-shaped addition. The `parse-error cut` row (shared-utility-contracts-rust.md:424) lists entry points and does not mention this memo.
  - The proof doc row for `OriginalSourceProjectionMemo` (docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md:88) describes what is memoised and is now incomplete. See C3.

## 3. Compile, lint and style

- **Signatures checked against HEAD.**
  - `LexerConfig`, `Span` and `SourceChannel` all derive `Copy`/`Eq`/`Hash`. `SourceImage` is `Debug`/`Clone`/`Eq`.
  - Each `Frame::Words` construction site gets `clean_key`: in `command_cut` and in `body_frame`.
  - Every caller of `capture_native_script_words` gets the new argument: the reborrow path, `_with_purpose`, and `_with_purpose_and_memo`.
  - `body` is copied out of `component.part` as at HEAD, so using it twice is fine.
  - `match … {}.map_err(…)?` used as a match-arm expression parses; rustfmt accepted it.
  - The let-chain at parse_cut.rs:791-795 needs edition 2024, which the workspace uses (Cargo.toml:162, rust-version 1.99).
  - `#![deny(missing_docs)]`: every new public item has a doc comment.
- **F1 (certain; `make rust-check` fails).** rustfmt rejects the `pub use parse_cut::{…}` reflow at lib.rs:110-114. The other four files pass `rustfmt --check`. Fix: run `cargo fmt`.
- **too_many_arguments.**
  - `command_cut`, `command_part_frame`, `first_parse_cut_channel_in_anchored` and `first_parse_cut_region_in_checked_with_memo` all have exactly 7 parameters.
  - Clippy's default threshold fires only above 7, and there is no clippy.toml override. They pass, but they now sit at the ceiling.
  - No new `#[allow]` was added.
- **Other pedantic lints.**
  - `single_match_else` should not fire at native_compilation_source.rs:221: the `None` arm is a call, not a block.
  - `needless_pass_by_value` is satisfied, because the image is moved into its callee.
  - `large_enum_variant`: `Words` grows by about 40 bytes, which is well under the 200-byte threshold.
- **F2 (Low).** `first_parse_cut_region_in_checked_with_memo` (parse_cut.rs:406) is `pub` inside `pub mod parse_cut`. That grows the cut owner's public API, but its only consumer is in the same crate. Make it `pub(crate)`, or list it in the contract row.
- **F3 (Low).** `#[derive(Debug)]` on the public `NestedBodyCutMemo` (parse_cut.rs:337) prints the bound image bytes and the set of clean keys. Its owner, `OriginalSourceProjectionMemo`, deliberately has a manual `Debug` ("Derived cache warmth is neither semantic identity nor diagnostic output"). Use a matching manual `Debug`.
- **F4 (Nit).** native_script_words.rs:4 adds a separate `use crate::parse_cut::NestedBodyCutMemo;` line, and :256 calls the function by its full path. The neighbouring code imports through the `crate::{…}` group.
- **Comment density** matches the surrounding code. The comments are plain and describe current behaviour, with no history.

## 4. Complexity: does it reach O(L) per level?

The test (package_resolver/tests.rs:176) has DEPTH = 2000 `[list …]` wrappers, about 14 KB in total. `walk_source` descends to `MAX_SOURCE_NEST_DEPTH`, about 64 levels.

- **Before the patch.** Each level's capture re-lexes every nested body below it. That costs Σ_{j>k} L_j, about 13 MB of lex, `SourceImage::from_bytes` (SipHash plus `LineIndex`) and arena decomposition per level, times roughly 64 levels. All 9 gdb samples are inside this path.
- **After the patch.**
  - The first capture, at the innermost level that is walked, still does one full O(D·L) walk and records about 1,936 clean bodies.
  - Each later level k lexes its region (L_k), decomposes it, walks body k+1 (L_{k+1}) and gets a memo hit on body k+2.
  - That is O(L) per level, with a constant of about 10 linear passes.
  - The key's config matches across levels: `nested()` is idempotent and the config is normalised, and `context.config` does not change across `walk_substitutions`.
- **Other per-level costs Luna lists.**
  - Segmenter (`walk_source_image` and the capture's own `segment_commands_image_with_offset_and_config` over the region): one linear pass each. Not visible in the samples.
  - `walk_source`'s `SourceImage::from_bytes`: linear.
  - The survey `Walker::script` → `native_script_words_in` (reachability/original.rs:223) runs once per script region. It does not walk operand substitutions. So it adds one more full O(D·L) walk in total, not one per level.
  - `prepare_source_chunk` → `compiled_preflight::preflight_image` → `native_script_words_in_with_purpose` would cost a full walk per level, over a fresh per-level image the memo cannot key. It only runs in `NativeCompilationMode::BytecodeObject`, which the default survey entry does not select, and no sample shows it.
- **Expected result.** About 2–3 full walks instead of about 64 or more, so roughly 20–30 times less work. That should fit the unchanged 180 s budget, but only a timed run can confirm it (C4).
- **Residual risk.** Not this test: a deeply nested document analysed with a BytecodeObject entry still does a quadratic walk per level in preflight.

## Conditions before commit

- **C1.** Run `cargo fmt` (F1).
- **C2.** Add deterministic tests in tcl-lexer `parse_cut`:
  - Memoised and unmemoised walks give identical `Result<Option<ParseCut>>` over a fixed corpus, with one shared memo and overlapping regions. The corpus covers: a clean body shared by two regions; a body with a cut inside a clean outer body; an unterminated quote whose fallback comes after a clean body; a `$a([…])` `report_at` anchor; an unavailable stream; and the same body under two `LexerConfig`s where one is clean and the other cuts. For the last case, `quote_termination`/`strict_quoting` or the `braced_var` dialect would work.
  - A foreign image bypasses the memo.
  - Add a linked proof test next to the `OriginalSourceProjectionMemo` tests showing that warmth leaves the projection unchanged.
- **C3.** Contract updates.
  - Extend the `parse-error cut` row (shared-utility-contracts-rust.md:424) with `NestedBodyCutMemo` and the memo entry point. State in the axis column that it is clean-only and keyed by whole-image body span, full `LexerConfig` and channel, and that it is derived warmth, not semantic identity.
  - Update original-point-operand-projection.md:88 to match.
  - Apply F2 and F3.
- **C4.** Compile, then run clippy pedantic and time `parse_pkg_index_survives_deeply_nested_source_wrapper` against its unchanged 180 s budget, after the guarded capture has finished.
