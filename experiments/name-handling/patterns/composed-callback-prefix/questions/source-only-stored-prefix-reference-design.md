Design question: expose readonly document-local references to the original public three-argument
captured prefix through its proven source advice, while keeping native capture admission independent.
No implementation, build or oracle widening is authorised by this design.

Current boundaries (actual APIs):
- `commands/tcl/list_.rs::spec`: `Arity::any()`, `ListArguments { from: 0 }`,
  `BUILDS_COMMAND_PREFIX`; `command_prefix_target.rs` describes `[list my method ...]`.
  No registry descriptor restricts this source prefix to two arguments.
- `commands/tcl/namespace_.rs` code descriptor: `NamespaceCommandPrefix`,
  `WRAPS_COMMAND_PREFIX | DEFERS_BODY`, source Body operand. Capture is not evaluation.
- `command_binding/method_prefix.rs::retain_created_prefix` requires exactly two arguments
  and actual normal result/receiver/method-entry receipts. `SourceCapturedMethodPrefix`
  feeds normal registration inventory. This is a narrower existing native producer, not
  a two-argument registry source grammar; its motivation beyond that code is undocumented.
- `OriginalSourceAuthoredCommandPrefix` already retains this original builder, alias lineage,
  target roles, all written operands including `{a b}`, member issuer and conditional obligations.
  Its public Tcl8.6 original positive/adverse source assertions passed; native capture is separate.

Smallest bounded integration: two existing LSP semantic owner files only.
`references.rs::references_in_program` first routes this original declaration through
`method_symbol::local_candidate` -> `local_reference_spans` (before lexical compatibility).
Add a crate-private readonly advice collector in `references.rs`, called ONLY from
`method_symbol.rs::local_reference_spans`; leave native method-reference/edit kernels intact.
Its input is the existing `OriginalMethodCandidate`, current source and genuine analysis.
`target.declaring_class_in(source, analysis)` selects the canonical source class without
reporting-name lookup. Enumerate that class's `original_members.declarations()` and exact
`OriginalSourceMethodMetadata::body_word()` owners; retain unique metadata/receiver-side joins.

Reuse existing actual same-frame callback machinery (`stored_callback_targets`,
`callback_targets_from_command`, `command_prefix_targets_from_word`) over those original bodies.
The wrapper and actual consumed sink must have current source schemas/registry script roles;
one sole same-frame write, literal variable root, no scope alias, unknown/opaque writer or
nested reassignment, exact written ordinals and assignment-before-consumer remain mandatory.
No native producer refusal may be repaired by this source collector.
For each candidate selector require `source_authored_command_prefix` plus `with_source_schema`
self-dispatch semantics and `definition_member_body()`; compare its original container exactly
with the original metadata `body_word()`, real Callable row/receiver side and current context.
Match the selector's OWN complete original word via `SignatureSourceNameKey::from_original_native_word`
under the target's retained name policy and exact `WordValueRules::from_config`; this is a
readonly source-word naming receipt, not a native invocation/frame. Never donate the declaration key.
Require a unique same-owner declaration match and genuine bare original selector geometry.
Missing wrapper/sink schema or ambiguous source declaration stays a measured refusal.

Append only readonly reference spans to `local_reference_spans`; its current consumers are
references and highlights. Keep `method_references_for_class`, receiver-identity capture,
rename, execution, call-point entry and edit eligibility unchanged. Workspace `source_queries`
needs its own later source-query design; this first integration makes no cross-document claim.

Acceptance: unchanged `probes/positive.tcl` on actual Tcl8.6 analysis yields declaration plus
selector [190,194), and selector alone with declaration excluded. Verify source advice separately.
Original `unknown-writer.tcl` selector [201,205) remains absent; keep opaque/nested writer controls.
Original full-source change, full lexer configuration change and missing resolved input withdraw
both advice and readonly selector ranges. Native channel never acquires Document advice.
All original four assertions and private fixture identity remain unchanged; private builder schema
is a distinct outstanding guard. Later release positives remain unclaimed until executed.
