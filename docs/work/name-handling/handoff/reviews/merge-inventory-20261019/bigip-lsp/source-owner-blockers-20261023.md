# Current compiler source-owner blockers

Observed on `cbfb4771`, Rust1.99.1, default compiler library graph. Exact raw
receipts are beside this file. All original assertions remain unchanged.

## Original iRules event word

Minimal source: `when HTTP_REQUEST {}; when CLIENT_ACCEPTED {}`.
`CompilationUnit::build_with_context_registry`, iRules profile/config/context,
then `Analyser::with_resolved_input` plus `set_cu_override` (the existing
`OriginalIrulesSourceContext::capture` ingress) yields:

- `source_graph::current_analysis` true; six original vendor source names.
- Two Module procedures, two genuine event bodies, current input present.
- `source_graph::event_body_for_procedure` succeeds for each procedure.
- Each event body's `event_word(module.source, config, registry)` is None.
- Both original `when` source vectors have a genuine original argument0.
- Both retained source events exist, but the inventory has zero declarations
  and two unavailable sites; the legacy `when_blocks` route also returns zero.

Compiler must retain/issue each genuine original event operand against its
own current source/Registry without claiming event installation or activation.
`lowering::original_when_event_word` / `SourceIrulesEventBody` is the producer
boundary. BIG-IP must not fabricate that private receipt or weaken its match.
This blocks the CLI explain event inventory and contributes to static trace
selection. The exact original CLI golden/trace assertions remain required.

## Original procedure body method inventory and first operand

Minimal source: `proc f {} {set h [ILX::init p e]; ILX::call $h m}`.
`Analyser::new().analyse(source, "f5-irules")` has current original source
context, but its generic source-vector inventory contains only the outer
`proc`, and `source_remote_first_operand_associations` returns `Some([])`.
The existing compiler test
`first_remote_operand_preserves_supported_true_same_frame_source_body`
requires one authenticated association for the same procedure-body form. The ordinary
consumer cannot create a body invocation owner or infer a handle locally.

The unchanged irules unit
`a_body_that_opens_a_new_frame_does_not_inherit_the_handle` also fails: both
`proc f {} {ILX::call $h m}` and a preceding outer `set h [ILX::init p e]`
produce no method cards at all. The required result is an unresolved literal
method card; the outer handle must never be inherited by the new local frame.
Compiler must supply original invocation geometry and retain conditional
applicability, independently of current handle/cell/observers. No consumer
walker or standalone fallback is permitted on a supplied-analysis route.

## Trait recursion (owned correction, not a Send/Sync defect)

Strict irules Clippy initially reports three `arc_with_non_send_sync` errors
at existing `Arc<CompilationUnit>` sites. A direct Send+Sync assertion instead
reports E0275 at default recursion128, and passes with recursion256 (also512).
The compiler and server already declare256. The owned irules crate now uses
256 too; this is a compiler trait-evaluation limit, with no lint allowance or
changed runtime behavior. Fresh strict irules library+tests verification passes with zero warnings;
`irules-clippy-20261023.json` retains the source hashes and exact command.
