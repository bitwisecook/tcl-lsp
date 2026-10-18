# Luna-12 review: Sol-5 `same-document-order-v6-delta.patch`

**Verdict: HOLD.** Two items block this patch.

1. The `nominal` predicate is unsound. It reopens the exact class of hijack that V5's `Selected` requirement refused.
2. The guard-test END change weakens an assertion that was correct.

The source-horizon change (fix 2) is relation-identical wherever both paths answer, and the code itself is fine. But its soundness rests entirely on sequence admission, so it can only be admitted together with a sound admission rule.

## Basis

- **Patch:** sha256 `7dfe7acc3f6c506da68fbae09acb5842586ace24fd2e54ee0eab8f6bde4b7a0e`.
- **Baseline:** `a/` matches `git show e46e88617:` for `source_graph.rs`.
- **Working tree:** `sol-5/b/` is byte-identical to the current working-tree `rust/tcl-lsp-core/src/source_graph.rs` and `docs/design/analysis/import-order-source-graph.md`. The orchestrator has already applied it, so the line numbers below are working-tree lines.
- **Unrelated working-tree edits:** `name-handling.md`, `body_effects.rs` and `source_class.rs` are also modified. They are not part of this patch and I did not review them.
- **Method:** static review only. tclsh probes are in `luna-12/probe/` (`run.tcl` sources a document and reports whether `::h` / `::a::h` exist at the end and whether the last statement ran). Results are in `probe/results.txt`, identical on 8.6 and 9.0.

## (a) `nominal` is unsound — HOLD

### What the admitted obligations mean

`SequenceCapture::nominal` (`rust/tcl-lsp-core/src/source_graph.rs:1050-1068`) accepts two kinds of non-`Selected` words:

- **`Conditional` words whose obligations are only `UnknownCurrentLookup`, `UnknownLookupCoordinates` or `UnretainedHolder`.** By their own definitions (`rust/tcl-compiler/src/command_binding/source_catalogue_advice.rs:19-30`), these mean:
  - "Current command-table contents are not closed";
  - "The source coordinate path is conditional on unavailable lookup state".

  The codebase's existing consumer of this exact obligation set treats it as terminal: `collection_lookup_is_available` (`rust/tcl-compiler/src/object_types/original_collection_sources.rs:567-583`, "Keep unknown lookup/earlier mutation terminal on the actual schema").
- **`SourceTransitions` words with empty lineage.** This arm ignores **every** obligation and every uncertain operation. That includes `UnknownEarlierMutation`, defined as "A prior source operation can mutate commands or namespace lookup" (`rust/tcl-compiler/src/command_binding/source_transition_advice.rs:300-301`). It also includes `uncertain_operations()`, the prior vectors that left transitions unknown (`:535-540`).

  Sol-5's own observed `namespace forget` source carries `UnknownEarlierMutation`. So this arm is looser than the `Conditional` arm, and looser than `collection_lookup_is_available`, which requires `uncertain_operations().is_empty() && !UnknownEarlierMutation`.

### Why the predicate cannot tell the benign cause from a rebinding

`UnknownCurrentLookup` is raised whenever `opaque_domain` is set (`rust/tcl-compiler/src/command_binding/original_command_table.rs:2251-2257`). `opaque_domain` is set by:

- `mark_opaque_resolution` and `mark_opaque_binding_mutation` (`rust/tcl-compiler/src/command_binding.rs:14445-14461`), whose doc comment says "an unbounded command-binding effect may occur";
- `mark_dynamic_proc_binding` and `mark_unnameable_rebinding_subject` (`:14464-14474`), which a dynamic `proc $n`, `rename $a $b`, or `interp alias {} $n …` reaches (`:18675-18760`).

So the obligation Sol-5 sees after an unmodelled `namespace export` is the same flag set by an in-document `proc $n …` that redefines `eval`. The words carry nothing that tells the two causes apart.

The barrier does not close this gap either. `blocks_registry_source` (`source_transition_advice.rs:1751-1777`) only refuses a head whose literal key has a Shadowed, Deleted, Alias-chain, procedure or Declared cell. A dynamic or unmodelled rebinding goes through `widen`, which creates no cell for the hijacked name (`:1731-1750`).

### Counterexamples

All of these are same-document, and none uses `interp alias` with a literal name or a cross-document definition. tclsh 8.6 and 9.0 agree on every row.

| probe | runtime fact | what v6 admits (statically expected) | V5 |
|---|---|---|---|
| `n1_import_hijacks_namespace.tcl` — Sol-5's fixture shape, except `::m` exports a command named `namespace` made by `rename helper namespace` | The forget inside `namespace eval ::a {namespace forget ::m::h}` resolves to the imported no-op. `::a::h` is **alive** at the end, and the document completes normally (after=1). | The root `namespace eval` is entered (Conditional[UCL,…], the same source Sol-5 observed). The forget's pattern word is admitted (SourceTransitions, lineage 0, the same as Sol-5's observation). The end is admitted. Result: the forget revokes `::a::h` at the end. **Wrong.** | refused |
| `n2_dynamic_eval.tcl` — `set n eval; proc $n args {}; eval {namespace forget ::m::h}` | The forget never runs, and `::h` is alive. | `eval` is Conditional[UCL] because of the opaque domain, so its body is entered, the forget is ordered before `set after` and the end, and it revokes. **Wrong.** | refused (not Selected) |
| `n3_dynamic_end.tcl` — `set n incr; proc $n args {return -code return}; incr x; namespace forget ::m::h` | The document completes normally at `incr x` (after=0), and `::h` is alive. | `incr x` is nominal, so the end is not withheld, and the forget revokes at the end. **Wrong.** | end withheld |
| `n4_namespace_path_end.tcl` — `namespace eval ::a {namespace path ::p}` with `::p::set` (a renamed proc doing `return -code return`), then `namespace eval ::a {set x 1}` | Early normal completion; `::h` is alive. | The `set` in `::a` is nominal and the end is admitted. **Wrong** (if the path is not marked as unknown paths, `original_command_table.rs:2265-2273`). | end withheld |
| `n6_import_hijack_set_end.tcl` — `namespace eval ::a {namespace import ::q::*}` where `::q` exports `set` (made by rename), then `namespace eval ::a {set x 1}` | Early normal completion; `::h` is alive. | The `set` in `::a` is Conditional[UCL, ULC] (the `::a::set` slot has no binding and no declaration, then `::set` is the catalogue candidate, `original_command_table.rs:2251-2281` and `2186-2215`). The end is admitted. **Wrong.** | end withheld |
| `n5_import_force_root.tcl` — `namespace eval :: {namespace import -force ::q::eval}`, then `eval {namespace forget …}` | The forget never runs. | Admitted if a `namespace eval ::` body import is unmodelled. Needs confirming as a control. | refused |
| `n7_namespace_unknown_end.tcl` | Early completion through `namespace unknown`. | An unresolved head has no words, so the end is withheld. **Correctly refused**; listed for completeness. | — |

Rows n1, n4 and n6 also refute the §8.1 sentence that an unmodelled `namespace import`/`forget`/`path` in a body "rebinds no command word of the document" (`docs/design/analysis/import-order-source-graph.md:457`). An import into `::a` rebinds every bare head later resolved in `::a`, including the `namespace` head of the very forget that test 404 depends on.

Rows n2 and n3 refute the "Known limits" claim that nominal-but-not-Selected words carry "a rebinding the compiler does not model — and nothing more" (`:465`). The compiler does model these: the obligation is its record that something unnameable was rebound.

The `interp alias {} namespace {} after 0` sentence in "Known limits" is used to justify the change, but Sol-5's evidence says only that the next `namespace eval` "is still Selected". It does not say whether the sequence **enters** the body. If the Selected effective words are those of `after` (the binding prefix), the flow is `Deferred` and nothing is wrong. If the sequence does enter the body, that is a same-document literal hole in V5 that should be fixed, not documented. Either way it is not evidence for `nominal`.

### Required

Drop `nominal`, and keep `Selected` for:
- entering a body;
- non-head positions;
- the end.

The 404, 022, 024, 028 and 029 shapes need the compiler to stop leaving the table globally unclosed after an unmodelled body statement. Two ways to do that:
- scope the unknownness to the body's namespace (`unknown_lookup_namespaces ∋ ::a`) instead of the global execution-unknown, so root heads stay `Selected`;
- model the import or forget when the source namespace's commands are document-closed.

With either, a head inside `::a` bodies correctly stays non-Selected after an unmodelled import into `::a` (n1, n6).

If a word-level predicate is ever reconsidered, its `SourceTransitions` arm must at least match `collection_lookup_is_available`. Then add n1, n2, n3, n4, n5 and n6 as refusal controls in `document_sequence_refuses_shadowed_or_namespace_local_entering_commands` or `document_end_is_after_normal_completion_only`.

Note that Sol-5's emulation ("13 fixed, 0 broken") ran only existing tests. None of them contains a hijack shape, so it is no evidence of soundness.

## (b) Guard-test END fixture — HOLD: the original assertion was right

The original fixture (`source_graph.rs:3392`) is:

```
set before 1
namespace eval ::n {
    set inner 2
}
if {$x} {set branch 3}
proc q {} {set body 4; return}
set after 6
```

It asserted `set after` → END is `KNOWN`.

**That is the correct Tcl answer.** Under §8.1, "END" means after normal completion, and nothing in this fixture can end normal completion early:
- `{$x}` is a braced expression with no call;
- a read trace's non-OK code becomes an error, as Luna-7 v4 showed with probes;
- the branch body `set branch 3` is retained and has no exit;
- `proc q` is deferred, and its `return` is not a document exit;
- an unset `x` is an error path, which never reaches END.

The relation is also reach-conditioned, so it holds vacuously whenever END is reached. This was the designated fast check in NOTES-v3:78 ("`set after`→END must be `KNOWN` in the guard test"). Luna-7 v4 and v5 re-confirmed it as well.

The implementation withholds END because, per Sol-5, `set after 6` gets **no words at all**. That is an imprecision upstream of the sequence, not a soundness refusal. Since `nominal` does not change no-words, v6 does not address it.

Moving the END assertion to a branch-free fixture (`:3396-3403`) therefore changes the test to fit the code. It also drops the only END coverage for "a non-exiting conditional branch before the end does not withhold it".

The new comment states an unverified cause ("unresolved behind the condition's variable read"). Sol-5 showed only that `if {1}` restores `Selected` and that `if {$x}` *plus* `proc q` loses the words. It never isolated the variable read. A tape fork/join barrier (`body_effects.rs:1534-1542`: differing advice at one site inserts a registry barrier) is an equally plausible cause.

This matters beyond the test. If any `if {$var}` makes every later root command wordless, then every `namespace eval` after it goes unentered, and the end is withheld in most real files.

**Required:**
1. Restore `assert_eq!(purpose("set after", "END"), KNOWN)` on the original fixture. Keeping the straight fixture as an *additional* assertion is fine.
2. Bisect the cause with these four variants:
   - `if {$x}` alone;
   - `proc q` alone;
   - `set x 1` followed by `if {$x}`;
   - `if {[gets stdin]}`.
3. Fix the cause, or file it with that evidence as a tracked limitation before any assertion changes.

## (c) Source-horizon advice (`source_graph.rs:1913-1990`) — sound by construction, gated on (a)

**Where both paths answer, the relations are identical.**
- For one document, `common_original_frame` returns `own(a)`, `own(b)` (`:2211-2215`).
- Sequence positions are never inside definition bodies (`contains`, `:1345`), and `site.body` is exactly `original_definition_body_span`. So `enclosing_body` is `None` on both paths.
- `in_effect_within(a, b, None)` and `load_order(_, false)` are the same functions the site path uses.
- `source_advice_after_document` builds the same end query (the image length) and answers through the sequence only when the end is admitted (`contains`, `:1339-1341`). Otherwise it falls back unchanged.
- Positions after a cutoff, cross-document pairs and body pairs also fall back unchanged.
- For SourceHorizon comparison, `navigation_comparison(a, b)?` still runs first (`rust/tcl-lsp-core/src/namespace_import.rs:180-190`), and it has the same sequence front path.

**Pairs that only the sequence answers** are positions inside an entered body, which is caller-context-unavailable. 1b45c540e excluded those sites because "none establishes an entered or new frame" (`rust/tcl-lsp-core/src/source_structure.rs:48`): a body with no site was being read as load-level. The sequence supplies exactly that missing premise, through the entering command's Registry flow (`enters_once`, no frame-level word, not nested in a caller-context-unavailable body: `entered_bodies`, `source_graph.rs:906-931`). So this does not contradict 1b45c540e's rationale, provided the entering proof is sound. With `Selected` it is; with `nominal` it is not (n1, n2, n5).

**Error direction.** SourceHorizon already abstains toward "in effect" (`namespace_import.rs:193-199`), so wrong removals mostly coincide with the old abstention. The exception is `after()` (`:220-225`), which needs `Some(false)`. Under an unsound entry, the new `after(forget, install) == true` makes a forget that never ran revoke an install (n1), where before the forget revoked nothing.

**Verdict:** admit fix 2 together with a `Selected`-sound admission rule, not with `nominal`.

## (d) Compile, format and docs

**Compile risk is low.** These paths are public:
- `tcl_compiler::command_binding::OriginalCatalogueSourceObligation` (`pub use`, `rust/tcl-compiler/src/command_binding.rs:145-147`);
- `OriginalConditionalRegistryMetadata::obligations` (`pub`, `rust/tcl-compiler/src/registry_invocation/conditional_metadata.rs:97`);
- `OriginalSourceCommandTransitionAdvice::lineage` (`pub`, `source_transition_advice.rs:527`).

The closure binding `&Obligation` under `matches!` is fine, as are the `is_some_and(SequenceCapture::nominal)` and `filter` signatures.

**Format and style:**
- `rustfmt --edition 2024 --check` is clean on `b/source_graph.rs` (my run).
- No `#[allow]`.
- No name matching.
- UK spelling is fine.

**Docs (§8.1):**
- Lines 457 and 465 make the two false claims above.
- The "Controls" admission bullet lost its separating comma before `document_sequence_refuses_…` (patch hunk line 41).
- The new sentence that the end "is read by end-of-document queries (`u32::MAX`)" is accurate (`namespace_import.rs:171`).

**Not run:** no cargo, tests or clippy.

## Summary of required actions

1. **HOLD-1:** replace `nominal` with `Selected`, and take the root-cause route in the compiler (scope unmodelled body statements to their namespace, or model them). Add n1–n6 as refusal controls.
2. **HOLD-2:** restore the original guard END assertion and bisect why `set after 6` has no words.
3. Fix 2 may land with item 1.
4. Correct §8.1 lines 457 and 465. Either establish whether the `interp alias {} namespace …` case is entered, or drop it as justification.
