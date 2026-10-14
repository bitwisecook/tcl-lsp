# Can list-built/captured callback prefixes supply current source targets?

Current finding: selected original builders and callback roles support conditional readonly source geometry. Prefix words alone establish neither future dispatch nor an editable target. This audit inspects one idiom and existing controls; it executes no Rust tests or providers and changes no canonical answers.

## Which actual sites motivate this question?

- [tcllib `modules/interp/interp.tcl:66–81`](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/interp/interp.tcl#L66), `corpus-snit-alias`: caller-frame `mymethod`, `linsert` and `eval` construct a child-interpreter alias prefix. List boundaries are the reduction; Snit and interpreter identity remain additional premises.
- [tcllib `modules/httpd/build/server.tcl:361–385`](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/httpd/build/server.tcl#L361), `corpus-captured-my`: socket/cron callbacks capture the object namespace for `my`. Namespace capture is the reduction; framework class provenance and callback host remain additional premises.
- [Wiki command prefix](https://wiki.tcl-lang.org/page/command+prefix), retained page index 1, examples 2, 4–7, 17–18: list words, execution form and builder shadowing. This is a retrieved page snapshot, not a verified revision.

[sites.json](../../../../experiments/name-handling/patterns/sites.json) entries 11–12 retain corpus revision, file/range hashes and licence; [wiki-sources.json](../../../../experiments/name-handling/patterns/wiki-sources.json) retains page/example hashes. Corpus entries retain coordinates/hashes and authored reductions, not vendored excerpts. These three contexts supply no prevalence measurement.

## What do the exact existing questions answer?

For the corpus rows, the canonical question is “What lookup completion and result does the exact [named] reduction produce after successful setup on each recorded provider?” The final line of each linked original probe is that lookup. C5 means 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0, with identical outcomes below.

| Existing question and exact source | C5 | Jim `0.84-9-g5bac7c9` | F5 |
| --- | --- | --- | --- |
| [list-prefix](../../../design/analysis/name-resolution-proofs/corpus-pattern-list-prefix.md), [probe](../../../../experiments/name-handling/patterns/probes/list-prefix.tcl): spaced command head and fixed/appended words through `eval [linsert ...]` | lookup `0: {a b} {c d}` | original lookup `0: {a b} {c d}` | not tested |
| [namespace-captured-prefix](../../../design/analysis/name-resolution-proofs/corpus-pattern-namespace-captured-prefix.md), [probe](../../../../experiments/name-handling/patterns/probes/namespace-captured-prefix.tcl): namespace-local `set prefix`, then qualified read and invocation | lookup `0: local {a b} {{c d}}` | original lookup `1: can't read "::nh::prefix": no such variable`; captured invocation is unreached | not tested |
| [string-prefix-near-miss](../../../design/analysis/name-resolution-proofs/corpus-pattern-string-prefix-near-miss.md), [probe](../../../../experiments/name-handling/patterns/probes/string-prefix-near-miss.tcl): string concatenation splits spaced head | lookup `1: invalid command name "::nh::two"` | same original guest error | not tested |
| [wiki-prefix-1](../../../design/analysis/name-resolution-proofs/wiki-prefix-1.md): “Does this retained program establish that list prefix preserves embedded data?” | PASS, expression `{{x; error injected} {a b}}` | inner-script `0: {x; error injected} {a b}` | not tested |
| [wiki-prefix-3](../../../design/analysis/name-resolution-proofs/wiki-prefix-3.md): “Does this retained program establish that builder spelling alone cannot prove list construction?” | PASS, `actual` | inner-script `0: actual` | not tested |
| [wiki-code-3](../../../design/analysis/name-resolution-proofs/wiki-code-3.md): “Does this retained program establish that captured command spelling observes replacement?” | PASS, `replacement` | inner-script `0: replacement` | not tested |

Wiki sources are the exact named cases in [wiki-probes.tcl](../../../../experiments/name-handling/patterns/wiki-probes.tcl); C PASS asserts the retained result expression. Jim Wiki executions are adapted inner scripts, not unchanged Tcltest executions or registered original corpus answers. [Jim captures](../../../../experiments/name-handling/patterns/jim-results-5bac7c9/records.json) retain channels; the separately authored [explicit-variable prefix control](../../../../experiments/name-handling/patterns/jim-probes/namespace-explicit-variable-prefix.tcl) returns `0: local {a b} {{c d}}` under different setup and cannot repair the original Jim error. [F5 contexts](../../../../experiments/name-handling/patterns/f5/questions.json) remain pending; desktop results and adjacent appliance controls supply no answer to these inputs.

## Which shared consumer and guards actually cover this idiom?

- [Compiler `produced_prefix.rs`](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/produced_prefix.rs), `retain_produced_command_prefixes` / `produced_prefix`: selected executable Body/CommandPrefix parent, one complete original builder, actual `BUILDS_COMMAND_PREFIX` plus `ListArguments { from: 0 }`, original target vector and full context. Multiple/dynamic/expanded builders and selected shadows refuse; alias captures preserve lineage without later written anchors.
- [Compiler `source_structure.rs`](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `source_authored_command_prefix` / `source_produced_command_prefix_words`: shared immutable source projection. [Core `references.rs`](../../../../rust/tcl-lsp-core/src/references.rs), `command_prefix_targets_from_word`, consumes the same selected schema/traits and original producer geometry; `my` and external `[self]` require distinct receiver/visibility handling. A literal head span remains syntax until navigation independently resolves it.
- Retained shape is not currency: [Core `CurrentSourceContext::capture`](../../../../rust/tcl-lsp-core/src/original_context.rs) requires the complete source image, complete lexer configuration and matching retained realm/input. `retained_dispatch_context` feeds that receipt into callback scans; nominal dialect or displayed names cannot substitute.
- Stored prefixes are bounded to one same-frame static assignment: `stored_callback_targets` / `collect_stored_callback_writes` in `references.rs` count writes across selected same-frame regions and withdraw for aliases, ambiguous writes, dynamic writers and opaque commands. Unknown mutation must not leave an old prefix eligible.
- [Compiler `OriginalCallbackPrefix`](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs) separately retains source head, baked/appended counts and selected scope. Registration does not donate an unreached trigger frame. [Database `apply_original_callback_arity_with_mode`](../../../../rust/tcl-lsp-db/src/lib.rs) requires independent source callback lookup/current exact headers. Logical consumers use [shared `SignatureCommandInvocation::logical_callback_candidate_keys`](../../../../rust/tcl-compiler/src/signature_scan/types.rs), which refuses original callback/prefix/name records; reporting candidates cannot reopen their retained lookup refusal.

## Which existing controls count, and how far?

- `original_produced_prefix_keeps_logical_context_capture_and_terminal_refusals` in `produced_prefix.rs`: captured argument has no later anchor; inert/lambda parent, dynamic/multiple/expanded builder, old/foreign context and shadows refuse. It is source-role coverage, not callback execution.
- `callback_source_roles_keep_actual_availability_and_original_builder_geometry` and `callback_source_targets_withdraw_known_shadows_and_incomplete_stored_writes` in `references.rs`: direct/stored/wrapped/aliased producer shapes, required availability, missing owner, changed config/context, unknown writer and opaque command barriers.
- `references_for_method_includes_one_hop_stored_callback_prefixes`, `stored_callback_prefixes_count_nested_same_frame_reassignment`, `actual_stored_callback_reference_uses_its_retained_brace_configuration` in `references.rs`: stored `namespace code [list my ...]`, nested writes, grammar mismatch and foreign source; `an_expanded_callback_word_invents_no_reference` covers sink/builder/wrapper expansion.
- `original_callback_missing_future_lookup_declines_reporting_candidates` in database `lib.rs`: retained prefix without independent lookup cannot borrow a conflicting reporting arity map. [Project projection controls](../../../../rust/tcl-lsp-db/src/original_project_callback_projection_tests.rs), `supplied_project_callback_projection_refuses_foreign_images_inputs_and_horizons`, cover separate project currency.
- `the_real_tcllib_callback_shapes_resolve_to_their_methods` in `references.rs` reads `virtchannel_base/cat.tcl` only when installed and may skip. Its comment explicitly excludes httpd's Clay class: no analysed class means no navigable method. It neither pins the selected maintained revision nor covers the selected httpd site.

These are inspected test definitions, not newly executed assertions. The [consumer integration map](../../../design/analysis/name-resolution-proofs/consumer-integrations.md) keeps their implementation contracts separate from native observations.

## What useful held-out pair would challenge the current acceptance boundary?

Proposed new source questions, unexecuted and unregistered: can an aliased builder, namespace capture and one-hop stored private-method callback retain the original selector span together; does an unknown writer withdraw it? These bytes differ from every oracle question above and require their own question/result receipts.

```tcl
interp alias {} ::makePrefix {} ::list
oo::class create C {
    method tick {fixed} {}
    method setup {} {
        set cb [namespace code [::makePrefix my tick {a b}]]
        after idle $cb
    }
}
```

Positive acceptance target: conditional readonly `tick` source reference, with the fixed word preserved and no runtime/edit claim. Adversarial negative: change `setup {}` to `setup {destination}` and insert `set $destination {my unrelated}` immediately before `after`; it may overwrite `cb`, so stored target selection must abstain. Separate controls cover ingredients; this composed pair has no retained execution or static-consumer outcome here.

## What remains open?

Whole Snit child-interpreter aliases, Clay/httpd class provenance, socket/cron invocation scope/argument injection and lifecycle are unmeasured. Core list/capture observations cannot close them. Future callback bindings, broad array/alias/SSA propagation, edit geometry and optimisation need independent owners/controls. No measured precision/recall, current-source Rust execution receipt or F5 runtime answer follows from this audit.
