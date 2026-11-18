# Nested-body analysis constant: measured shares and proposed changes

Analysis-performance, 18 November 2026. Proposal only: every change below is
in another owner's code, so nothing here is landed (coordinator ruling,
18 Nov: write the change with its measured share and do not change other
owners' code without it).

## Where it stands

- Shape: a `when` body holding N nested `if {1} {` bodies.
  `find_enclosing_when_event`, opt-level 1, per call:

  | depth | before be6f1362 | after be6f1362 |
  | --- | --- | --- |
  | 200 | 3.3 s | 2.8 s |
  | 400 | 9.2 s | 5.9 s |
  | 800 | 26 s | 14.4 s |

- The cubic part, in which every enclosing body re-asked the vendor barrier
  capture for every head below it, is removed by an exact memo (be6f1362).
- The counted barrier work holds the ruled quadratic ceiling
  (`nested_if_heads_take_barrier_work_at_most_quadratic_in_depth`).
- What remains is quadratic, as tclsh is (4x per doubling, depth 1000 in
  15-19 ms). The constant is about 300x tclsh's: depth 400 costs 5.9 s.

## Measured shares

The profile is gdb sampling of the probe at depth 800 after be6f1362: 150
samples, all inside `Analyser::analyse_body` →
`retain_original_static_source_names` except 16.

| Share | Path | Why it is quadratic |
| --- | --- | --- |
| 39% (58) | `registry_invocation::conditional_vendor_metadata::original_vendor_occurrence_registry_metadata` | see the three rows below |
| 19% (28) | … → `native_compilation_source::original_native_compiler_words` → `OriginalSourceCommandProjection::capture_with_nested_cuts` | Each head re-segments its own nested body text (`segment_commands_image_with_offset_and_config`, `native_script_words_in_with_purpose`). An outer `if`'s body spans the rest of the nest, so the total is O(depth²) bytes, re-lexed per head. |
| 4% (6) | … → `vendor_source_barriers::capture_in_script_body` | Each head's receipt lists every enclosing region (O(depth) per head). This is now memoised per distinct question. |
| 4% (6) | … → `vendor_advice::vendor_registry_invocation_shape` | |
| 11% (17) | `signature_scan::original_name::SignatureSourceNameKey::from_original_native_word` → `tcl_registry::native_compiler_words::NativeCompilerWords::capture` | Re-derives a braced word's bytes for every occurrence (`native_source_braced_word_bytes`, `collapse_continuations`, `native_document_ingress`). This is O(word length) each time the same word is named. |
| 9% (14) | `segmenter::segment_commands_local` | `build_document` over the body text on every call, including `build_line_starts` for that text. |
| 5% (8) | `tcl_lexer::native_script_words::native_script_words_in` | Plans for regions outside the barrier cache. |
| 5% (8) | `signature_scan::vendor_name::VendorSourceNameOccurrence::merge_source_origin` | Clones `ResolvedContext` (axis floors, version sets) per occurrence. |
| 4% (6) | `realm::CommandBindingRealm::stamp_original_tokens` | |

## Proposed changes, largest share first

1. **Segment a body's text once.** This targets the 19% under
   `capture_with_nested_cuts`, plus the 9% in `segment_commands_local` and
   the 5% in `native_script_words_in`.
   - Key the segmentation of a body's text by (source image, span, lexer
     configuration), as `vendor_source_barriers::region_plan` already does
     for word plans. Bound the cache per thread and clear it when full.
   - The segmentation is a pure function of those three, so results are
     unchanged.
   - A head nested at depth k then costs O(its own words) instead of
     O(text below it).
   - Owner: whoever holds `registry_invocation::native_compilation_source`
     and `segmenter`.
2. **Derive a braced word's bytes once.** This targets the 11% under
   `NativeCompilerWords::capture`.
   - Cache the decoded bytes on the `NativeWord` (a `OnceLock`), or key
     them by (image, span, config).
   - Repeated naming of the same word then costs O(1).
   - Owner: tcl-lexer / tcl-registry (`native_compiler_words`).
3. **Share the resolved context** in `merge_source_origin`. This targets
   the 5%.
   - Hold `ResolvedContext` behind an `Arc` in the occurrence, or compare by
     the generation's identity.
   - Owner: signature_scan.

Together these account for about 50% of the samples. Expected effect: the
quadratic term remains, as it must, but its constant falls by roughly half.
The O(depth²) body text itself is inherent; tclsh pays it too.

## How to verify

- The count-based witness is
  `registry_invocation::vendor_source_barriers::scaling_tests::nested_if_heads_take_barrier_work_at_most_quadratic_in_depth`.
  A change under items 1–3 should add its own step counter and a ≤ 4×
  bound in the same way.
- Check results are unchanged with a full tcl-compiler run with and
  without the change (13160 tests, same failures).
- tcl-lsp-core `deeply_nested_bodies_survive_when_scanning` takes minutes at
  depth 2000 today. It asserts survival only (ledger REWRITE), and its
  depth exceeds tclsh's limit on purpose.
