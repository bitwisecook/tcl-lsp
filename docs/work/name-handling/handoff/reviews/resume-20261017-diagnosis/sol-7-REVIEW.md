# Sol-7 review: nested-body-cut-memo V2

- **Candidate:** `luna-5/nested-body-cut-memo-v2.patch`, with `luna-5/NOTES.md` and the `a/` and `b/` copies.
- **Prior review:** Sol-3 `REVIEW.md`, conditions C1–C4 and F1–F4.
- **Patch integrity.**
  - `a/` is byte-identical to 4c3eb7953 and to f606aa20c.
  - `a/` plus the patch gives `b/` exactly.
  - `git apply --check` passes.
- **HEAD moved during this review.** The orchestrator committed 60051c3c2 ("Integration37 candidate"). All eight touched files in it are byte-identical to `b/`. Every file:line below is therefore valid at the current HEAD as well as in `b/`.
- **Method.**
  - I applied V1 and V2 to scratch copies and diffed them against each other.
  - I read `parse_cut.rs`, `native_script_words.rs`, `lexer.rs` (`parse_command`, `warn_or_error`), `script.rs`, `source_map.rs`, `segmenter.rs`, `xtask/src/owner_resolution.rs` (and `source_internal.rs`), `segmentation_drift.rs`, and both proof scripts.
  - I ran `rustfmt --edition 2024 --check` on the scratch copies.
  - I ran the proof gate's own Python (`render`, `test_body`, `schema_errors`, and the full `validate()`) against overlays.
  - I did not run cargo, tests or gdb, and I did not modify the repo.

## Verdict: ADMIT (source level)

Every Sol-3 condition that can be met at source level has been met, and the evidence is real:

- C1/F1: the files are rustfmt-clean.
- F2: the memo entry function is now `pub(crate)`.
- F3: `Debug` is manual and shows no cache contents.
- F4: the import grouping is fixed.
- C2: there are seven lexer tests plus one linked compiler test.
- C3: the contract row, the manifest and the proof doc are all updated.

I traced every new test by hand through the lexer and walk code, and every expected value holds. Each test exercises at least one memo hit, a record that must not happen, or a memo bypass. None is vacuous on that axis.

The changes to the manifest and proof doc are consistent with the generator, and the proof gate reports no new error. The owner-resolution rules are satisfied. No existing assertion, selector, fixture or timeout changed, and no `#[allow]`, `#[expect]` or `#[ignore]` was added. I found no compile risk against HEAD signatures.

**C4 is still open.** That means compile, clippy pedantic, running the 8 new selectors, and timing `parse_pkg_index_survives_deeply_nested_source_wrapper` against its 180 s budget. Only the orchestrator can close it.

- The 60051c3c2 commit message reports "Build37 passed with zero warnings; 19/24 targeted selectors pass".
- Before final admission, confirm that none of the 5 failing selectors is one of the 8 new tests or the deep-wrapper test.

## Conditions check (V1 to V2 diff)

| Cond | V2 change (file:line at HEAD/b) | Status |
| --- | --- | --- |
| C1 / F1 rustfmt | `rust/tcl-lexer/src/lib.rs:110-115` has been reflowed. `rustfmt --edition 2024 --check` exits 0 on all 5 `.rs` files in `b/`. The same invocation on the V1 `lib.rs` reproduces the F1 diff, so the check really is sensitive. | Met |
| F2 `pub(crate)` | `parse_cut.rs:413`. It is not re-exported from `lib.rs`. The only callers are `native_script_words.rs:255-264` and the tests. | Met |
| F3 manual Debug | `parse_cut.rs:337-348` writes only `"NestedBodyCutMemo"`, with the same comment as the owner's. `NestedBodyKey` (`:353`) dropped `Debug`, and nothing needs it: `Frame`, `WordJob` and `MemoAnchor` derive no `Debug`. A test asserts that warmth leaves the Debug output unchanged (`:1605`, `:1618`). | Met |
| F4 imports | `native_script_words.rs:4-9` uses the grouped `crate::{…}` with `parse_cut::first_parse_cut_region_in_checked_with_memo`. The call at `:255` no longer uses the full path. | Met |
| C2 lexer corpus | `parse_cut.rs:1550-1763`, a new module `nested_body_cut_memo_tests` with 7 tests (traced below). It covers every case Sol-3 listed: a shared clean body, a cut beside a clean body, the unterminated-quote fallback, the `$a([…])` anchor, an unavailable stream, two `LexerConfig`s on the same body, and a foreign image. | Met |
| C2 linked proof test | `original_source_projection.rs:167-190` (`body_words`) and `:377-451`. It has the proof ID and the document comment, and the matching `linked` binding is in `manifest.json:1133947-1133951`. | Met |
| C3 contract row | `shared-utility-contracts-rust.md:424`. | Met (see O-1) |
| C3 proof doc | `manifest.json:1133863` (owner purpose); the rendered `original-point-operand-projection.md:88,103`; and the hand-written paragraph `:122`. | Met (see G-1) |
| C4 build/clippy/timing | Not executable by a reviewer. | Open (orchestrator) |

`native_compilation_source.rs` is identical in V1 and V2. Sol-3 reviewed it, and it stays correct.

## New lexer tests traced against the code

The walk facts these traces rely on:

- **Key coordinates.** `MemoAnchor::key` is `origin + base + body` (`parse_cut.rs:390-404`). Arena component spans are in frame-image coordinates (`content_part_frame` decomposes `image` at `content`). A nested frame's `base` is `child_base = base + body.start` (`:853`). So the same body reached from different regions maps to the same whole-image span.
- **Record point.** A body is recorded only when its `Words` frame runs out of jobs (`:708-714`). At that point every child frame has been popped without a cut.
- **Top-level frame.** The top-level command frame has `clean_key: None` (`:583`).
- **Hit.** A matching key skips the push (`:799-803`).
- **Binding.** `bound_to` compares `SourceImage` equality, and that equality includes the channel (`source_map.rs:71-75`).
- **Key config.** The key's `config` is the drain config. Because `NestedScript.config` is `nested()`, which sets only `leading_bom = Content` (`lexer.rs:436-441`), keys are consistent across regions. That includes production, which forces `Content` for a nonzero region anyway.

| Test (parse_cut.rs) | Trace | Hit / non-record exercised |
| --- | --- | --- |
| `…shares_a_clean_body_between_overlapping_regions` :1601 | Region 1 walks body1 and its two inner bodies, giving count 3. Region 2's top-level words carry the two inner bodies, and both are hits. Region 3 (whole) gets a hit on body1. Every case is `Ok(None)`. | Hits in regions 2 and 3 (see T-1) |
| `…rewalks_a_cut_beside_a_clean_body` :1621 | `list a` is recorded. `set y {a}b` is welded, which gives `ExtraAfterBrace` (`word_job` `:615-620`). The enclosing body and the cut body are never recorded, so the count stays 1 across 3 regions. | Non-record of a cut body and of a cut-containing parent; a hit on `list a` beside a cut |
| `…unterminated_quote_fallback_after_a_clean_body` :1641 | Region 1 records `list a`. In the whole region the `"` never closes, so the walk reaches the `Content` with fallback `MissingQuote` (`:668-672`). Body `list [list a] b` is walked: its inner body is a hit, the body itself is recorded, and the count becomes 2. The fallback is still returned. The second pass gets a direct hit on the body, and the fallback is unchanged. This matches precedent `"puts \"unterminated"` giving `MISSING_QUOTE` (`:1302`). | A body-level hit with the fallback preserved |
| `…keeps_the_variable_index_reporting_anchor` :1667 | The index `Parts` frame has `report_at = Some(5)` (`$`), and the body frame inherits it (`:807`). The inner `list x` is a hit. The welded `{y}z` gives `Cut(weld, weld)`, reported at `report_at`, so offset = `$` and term = `z` = `find("{y}z") + 3`. This mirrors `deep_index_cut_preserves_outer_anchor_and_exact_inner_term` (`:982`). | A hit under `report_at`; non-record of the cut index body |
| `…never_records_unavailable_streams` :1691 | **Strict half.** The top-level `parse_command` scans bracket content flatly (`lexer.rs:1920-`), so the region lexes. The body lex of `list "x"y` under `strict_quoting` hits `warn_or_error` and returns `Err`, so `LexicalStream` is returned from `body_frame` before any push. `list a` is recorded (count 1), and the second pass hits it. **Tampered half.** `words[2].tokens.end` is pushed out of range, so `written_span` returns `None`, giving `Unavailable` and then `SourceGeometry`. This follows precedent `:1015`. | A refusal is never recorded, and a hit precedes a refusal (see T-2) |
| `…keys_each_lexer_configuration_separately` :1727 | Under tcl8.6 `{*}` expands, so the walk is clean and recorded. Under tcl8.4 the welded `{*}{a b}` gives `ExtraAfterBrace` (precedent `cut_follows_the_configured_dialect` `:1264`). If `config` were not in the key, the 8.4 pass would hit the 8.6 entry, and `memoised == plain` would fail. | Key discrimination, a load-bearing assertion |
| `…bypasses_a_foreign_image` :1745 | The memo binds to `list [list {x} y]`, body [6,16). Image 2 has the same extent [6,16), but `{x}y` cuts there. Without the bypass it would hit, and the equality check would fail. Image 3 and the native-channel copy would each add a key if they were served, so `count == 1` proves the bypass. | Bypass, with load-bearing assertions |

Compile check of the test code:

- `use super::*` reaches the parent's private `use` imports (`Lexer`, `group_commands_bytes`, `SourceImage`, `Span`, `LexerConfig`) and its private fields (`memo.clean`).
- `SourceImage::native(&[u8])` works through `impl Into<Arc<[u8]>>`.
- `ParseCut` and `ParseCutUnavailable` derive `PartialEq`, `Clone` and `Debug`.
- `WordSpan.tokens` is pub, as precedent `:1015` shows.
- `LexerConfig::for_dialect(&str)` exists.
- The tests add no casts beyond the existing `u32 as usize` pattern, and I see no pedantic-lint shapes: no `similar_names`, nothing near `too_many_lines`, no items after statements.

## Linked compiler test (original_source_projection.rs:377)

- **`body_words` mirrors `capture` exactly.** It segments the region sub-image with `config.nested()` at the body offset. `capture` does the same at `native_compilation_source.rs:319-323`. It then calls `CommandTokens::from_segmented` over the whole image map with `config`. `image.source_map()` is `SourceMap::from_image(self)` (`source_map.rs:297`), the same as `capture`'s. So the IR-vector check passes by construction for the clean captures.
- **Expected `Some`/`None` per capture:**
  - `list a`@13, body@7, top0, and `list a`@43 are clean.
  - The malformed body@37 cuts at its own top level. That gives `fatal_tail`, then `commands.len() == 0`, then `None`.
  - top1 cuts in the nested body, then `None`.
- **Hits happen in both orders.**
  - Forward: top0 hits `list a`@13 and `list b`, which were recorded by the body@7 capture. top1 hits `list a`@43, recorded by the malformed body capture.
  - Reverse: the order is the mirror image of forward.
- **Memos are created per (grammar, channel, order).** Each one binds a single image.
- **Signatures exist.**
  - `capture_with_nested_cuts` is `pub(crate)`.
  - `native_words()` and `command()` are `pub(crate)` (`native_compilation_source.rs:380-386`).
  - `SegmentedCommand` and `NativeWord` derive `PartialEq` and `Debug`.
  - `geometry(&Arc<_>)` coerces at the annotated closure parameter.
  - The `for ((offset, words), complete) in ordered` default-binding pattern gives `&u32`, `&Vec<WordExpr>` and `&bool`, and these are used as `*offset`, `words` and `*complete`.

## Proof generator and gate

- **The generator is a Python script, not an xtask.** `scripts/dev/update-name-resolution-proof-index.py` provides `render(record)`, and `make check-name-resolution-proofs` runs `scripts/dev/check-name-resolution-proofs.py`.
- **Rendered prefix.** For both `a/` and `b/`, `render(manifest record)` is a byte-exact prefix of `original-point-operand-projection.md`, so the rendered rows match the manifest. `json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"` reproduces `b/manifest.json` byte for byte, so serialisation is canonical. Only the `naming.source.original-point-operand-projection` record changed.
- **Gate helpers.** The schema has no errors. `test_body` finds the new selector, and its body contains the proof ID and the document path. The owner symbol is present.
- **Full gate.** I ran `validate()` with the eight files overlaid from `a/` (pre-patch) and from `b/`. The error sets are listed below under "Gate delta". The two evidence-digest mismatches on `lib.rs` and `native_script_words.rs` are pre-existing, as the NOTES say: the manifest digest matches neither `a/` nor `b/`.
- **Gate delta.** `validate()` returned 2065 structural errors with the `a/` overlay and 2065 with the `b/` overlay. The two error sets are identical, so V2 adds no gate error. All of those errors are pre-existing at HEAD: census digests, plus the evidence digests for `lib.rs`, `native_script_words.rs` and `materialized_footprint.rs`. The runner is `tools/gate_overlay.py`, and its outputs are `tools/gate-a.txt` and `tools/gate-b.txt`.
- **G-1 (Info, pre-existing).** The doc's "Immutable identity and derived memo lifetime" appendix (`:116-124`) is not in the manifest; `replay.limitations` contains none of it. This was already true at HEAD before the patch. Running `update-name-resolution-proof-index.py --render` on this record would delete the appendix, the new V2 paragraph `:122` included. The NOTES claim that the "HEAD render matches HEAD document byte for byte" is true only of the rendered prefix. The gate does not compare whole documents, so this is not a blocker.

## Owner resolution (xtask owner_resolution.rs)

- **Entry-point rule.** Each entry point must be a `pub` item (`exported`, `owner_resolution.rs:2113-2125`) in one of the row's source paths.
  - `NestedBodyCutMemo` is a `pub struct` at `parse_cut.rs:338`.
  - `native_script_words_in_with_purpose_and_memo` is a `pub fn` at `native_script_words.rs:215`.
  - The `pub(crate)` function is correctly not listed.
- **Other row checks.** There are 5 columns, the axis is non-empty, and the drift gate `xtask-segmentation-drift` is unchanged. The source and internal manifests are opt-in and untouched. No rule forbids a source path from appearing in two rows.
- `segmentation_drift.rs` flags only terminator-scan and word-start state-machine shapes, and the new code contains neither.
- **O-1 (Nit).** `native_script_words.rs` is now listed in both the "command / word segmentation" row (`:422`) and the "parse-error cut" row (`:424`). The gate accepts this, and the memo entry is a parse-cut routing entry, so it can stay.

## Other findings

- **T-1 (Low).** No test can observe that a hit actually happened. `HashSet::insert` is idempotent, so every assertion would still pass if `is_clean` always returned `false`.
  - The tests do prove that a hit never changes the answer, and tests 6 and 7 prove that a wrong hit would be caught.
  - The performance claim itself rests on C4 timing.
  - Optional: add a test-only hit counter. This is not required, because correctness is indistinguishability.
- **T-2 (Low).** The tampered half of `…never_records_unavailable_streams` (`parse_cut.rs:1708-1724`) makes a top-level word unavailable. That frame never had a `clean_key`, so this half shows only that a clean sibling is recorded and the `Err` is kept.
  - The strict half does cover an unavailable body, but via a lex refusal, so that body is never pushed.
  - A body whose arena or geometry fails after it is pushed (`content_part_frame` `Err` inside a body) is not covered. By code reading it is correct anyway, because the early return skips `record_clean`.
- **T-3 (Info).** The strict-stream case drives the `pub(crate)` walk with `strict_quoting: true`. The production entry forces `strict_quoting: false` (`native_script_words.rs:248`). The test is valid for the unit, but it is not a production configuration.
- **M-1 (Nit).** The new manifest test binding orders its keys path, selector, assertion, status. The binding right after it orders them path, selector, status, assertion. The record was already mixed, and the order is harmless.
- **Scope check.**
  - The patch's only removed lines are the replaced call and import sites and the old contract and purpose text.
  - No existing assertion, selector, fixture or timeout changed. The deep-wrapper test and its 180 s budget are untouched.
  - No `#[allow]`, `#[expect]`, `#[ignore]`, `should_panic` or `Duration` was added.
