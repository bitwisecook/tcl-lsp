# Current compiler source-owner blockers

Initial observations used `cbfb4771`. The three source gaps below are confirmed
again on `73c43bdb`, including the published `d1a0fe02` event-geometry repair.
Rust1.99.1, production native library graph; no original assertion changed.

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
A second production-library probe passes the complete input through
`CompilationUnit::build_with_analysis_input` instead of the context builder.
Both genuine event bodies still return no event word: zero declarations and
two unavailable sites. Changing the consumer ingress does not repair this;
raw `source-input-owner-probe-20261023.txt.gz`.

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

## Original remote call nested in setter substitution

Current production-library probe and ILX16 expose a further API gap. Same
actual `Analyser::new().analyse(source, "f5-irules")` input, with constructor
`set h [ILX::init p e]` and later `ILX::notify $h m` in one handler:

| First remote source | Compiler associations | Consumer source target |
| --- | --- | --- |
| `ILX::call $h m` | 1 current, exact vector join | p/e |
| `set reply [ILX::call $h m]` | 0 | unavailable |
| `ILX::call $h m [HTTP::uri]` | 1 current, exact vector join | p/e |
| `set reply [ILX::call $h m [HTTP::uri]]` | 0 | unavailable |

The genuine invocation/method vectors exist in all four cases. The missing
relation is in the compiler producer, not the consumer join. ILX16's standard
`rule_source` uses the fourth form; first-site navigation assertions remain
unchanged. Raw exact command/source/output:
`nested-ilx-owner-probe-20261023.txt.gz`.

Compiler must supply the genuine nested invocation's constructor/setter/read
association under its own analysis, preserving all obligations and withdrawal
for the later notify. The outer reply assignment must not erase a source read
inside its RHS before that read is considered. No consumer syntax walk,
variable-name inference, recreated argument receipt or runtime handle target.

## Same-frame if positive after a procedure declaration

The original ILX16 frame test's positive also fails independently of setter-RHS
nesting. An isolated handler with `set own [ILX::init p e]` then
`if {1} {ILX::call $own m}` produces one compiler association and p/e candidate.
Prepending `set handle [ILX::init p e]` still produces that candidate. Prepending
`proc helper {} {ILX::call $handle m}` instead removes all produced method
cards and associations, including the later handler's own constructor/read.
The full original frame fixture likewise produces none. This is not an outer
constructor barrier interpretation; the proc declaration is the differentiator.
Raw `if-prefix-owner-probe-20261023.txt.gz` contains all four exact sources.

The compiler's procedure/source-vector owner must retain genuine later
conditional method syntax and the independently supported same-frame source
relation, while refusing inheritance of the outer handle in a new procedure
or handler frame. The original first two negatives and final if positive all
remain required; no fixture, depth, expected location or assertion changed.

## Confirmation on the published repair snapshot

`73c43bdb` still has zero event words for both genuine event bodies in all12
combinations: profile versus environment-default context; body versus file
lexer configuration; context builder, complete-input builder and explicit
standalone-input builder. Each produces2events and no native entry. This
includes the exact constructor shape used by the new compiler source-graph
positive. Raw `latest-event-ingress-matrix-20261023.txt.gz`. No consumer ingress
variant repairs it; retain the original geometry-without-activation contract.

Latest production probes also confirm flat remote1/current/exact vector join,
setter-RHS remote0, procedure-body constructor/read0, and loss of the later
handler's cards after a procedure prefix. Raw `latest-{nested-ilx,if-prefix,
source-input,procedure}-probe-20261023.txt.gz`. These are observations, not
passing semantic regressions. The latest affected native tests still have
CLI15/17, source-event0/1, trace0/2; full raw/summary are in
`published-event-repair-20261023.{txt.gz,json}`. All byte-fidelity tests pass.
