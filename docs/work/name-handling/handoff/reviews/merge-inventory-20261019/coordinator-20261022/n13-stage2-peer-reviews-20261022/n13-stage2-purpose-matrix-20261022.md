# N13 stage-2 source-origin and purpose audit

Read-only mechanical/static audit of the frozen off-tree candidate.

- Patch SHA-256: `33bdf6c763aeed82d35071d59eb91681b3167cb8ddbb276856a6aa3c7f24d3f2` (`/tmp/compiler-names-n13-stage2.patch`).
- Base HEAD in receipt: `93a5a341462bf646c6eeb16875d6f5f12197c798`.
- Candidate shadow: `/tmp/compiler-names-n13-shadow`.
- Verified each of the 16 touched files in that shadow against its receipt `after_sha256`; all 16 matched.
- The receipt labels the candidate “OFF-TREE PARTIAL”; this note is not typecheck/test evidence or whole-scope approval.

## Purpose-selection matrix

| Path and lines (candidate shadow unless noted) | Purpose path | Audit result for inherited point |
|---|---|---|
| `rust/tcl-compiler/src/command_binding/origin_inventory.rs:1644–1687` | State-retained purpose checks registry/input ownership; then rejects `baseline.source_point_entry`; only afterward can a real `native_entry` yield `Execution`, logical entry yield `Logical`, retained exact AuthoredSimulation policy yield `AuthoredSimulation`, or explicit standalone metadata yield `Standalone`. | `InheritedPoint` returns `None`; no inherited `Execution`/standalone/policy inference. |
| `rust/tcl-compiler/src/command_binding/origin_inventory.rs:1695–1735` | Module-retained purpose validates module metadata; then rejects `source_entry.source_point_entry`; subsequent selection is Native entry, unknown refusal, exact Logical entry, exact AuthoredSimulation policy, or explicit Standalone. | `InheritedPoint` returns `None` before all purpose grants. |
| `rust/tcl-compiler/src/source_graph.rs:268–317` | Direct original procedure-body caller obtains purpose via `...?`; then exhaustively dispatches Logical, AuthoredSimulation, Execution, Standalone. | Missing purpose refuses at `?`; no string/printed-name fallback. Source graph body result is documented read-only reachability, not Native dispatch/completion/frame permission (`:264–267`). |
| `rust/tcl-compiler/src/registry_invocation/execution_trace.rs:28–80,86–104,110–171,176–211` | Trace-key and trace-absence readers branch by exact purpose. AuthoredSimulation refuses/returns unknown (`:35–36,117–121,189–194`); Execution requires actual Native target/observer evidence (`:134–146,203–211`); Logical/Standalone use exact authored/direct builtin routes. | Missing purpose refuses (`Option` propagation); inherited source cannot establish absence. |
| `rust/tcl-compiler/src/dispatch_proof.rs:1153–1199` | Original ledger subject accepts only `Logical | Standalone` and validates recorded original words, binding, facts, direct builtin slot and call site. | Execution/AuthoredSimulation/inherited `None` cannot become a subject. |
| `rust/tcl-compiler/src/optimiser/method_barrier.rs:529–559` | Logical uses logical operation; Execution/Standalone/AuthoredSimulation use selected handler invocation facts. Missing purpose/invocation marks `facts.anywhere` and continues conservatively. | This is a conservative barrier query, not a native compiler admission; inherited `None` falls to unknown. |
| `rust/tcl-compiler/src/type_infer.rs:502–539` | Explicit standalone no-token path uses the standalone scope-alias helper; otherwise purpose is required and selects Logical or Execution/Standalone/AuthoredSimulation handler facts. | Inherited view has source binding but purpose projection refuses; no fallback to the no-token standalone branch. |

## InheritedPoint consumer matrix

| Path and lines | Consumer | Classification |
|---|---|---|
| `rust/tcl-compiler/src/command_binding.rs:500–537,583–600,3171–3191` | New `InheritedPoint { parent, source }`; source-image accessor includes its exact bytes; declaration matching explicitly returns `false` for InheritedPoint, Derived and ConditionalField. | Source identity/data only; declaration-authorship refusal. (Declaration match implementation is at lines 3171–3191.) |
| `rust/tcl-compiler/src/command_binding.rs:7336–7342` | Origin constructed only when `options.source_point_entry` is present; otherwise origin is Authored. | Explicit inherited-origin construction, no source-name heuristic. |
| `rust/tcl-compiler/src/command_binding/source_point_entry.rs:218–255,257–321` | Produces child source-analysis entry; sets `source_point_entry`, clears native entry, compiled-variable provider and native compilation; child-state validation rejects Native/hosted context and nondefault compilation/provider. | Explicit source-only context, no Native/compiler grant. |
| `rust/tcl-compiler/src/command_binding/source_point_entry.rs:472–542` | Issues point receipt only from same original image/config and complete non-expanded word vector; requires exact unanimous post-argv dispatch point, normal argument completion, no Native/hosted/opaque/source-step/observer state, and registry-backed target. | Refusal on incomplete or nonunique entry evidence. |
| `rust/tcl-compiler/src/command_binding/source_point_entry.rs:400–415`; `rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs:63–85` | Join inherited child origin to exact retained point token, then require actual ancestor analysis for parent declaration. | Exact-owner navigation only; no equal-bytes parent fallback. |
| `rust/tcl-compiler/src/analyser/types.rs:4266–4309,4315–4363` | Inherited procedure/class lookup checks exact source bytes, matched inherited owner, then requires a unique retained metadata record and returns. | Inherited special path returns `None` before Authored token/name fallback. |
| `rust/tcl-compiler/src/registry_invocation.rs:7157–7161` | Converts any retained origin variant to its source image for original-word access. | Byte/source access only; does not select a binding or purpose. |
| `rust/tcl-compiler/src/command_binding/source_transition_advice/source_point.rs:17–50,63–112,115–143` | Derived source-only advice graph; copies parent namespace/path/import facts while clearing child declarations, body, handles and steps; rejects Native baseline, uncertainty/debt, wrong namespace, non-current body interpreter, or mismatched source-load path. | Source advice only, not a Native/runtime table or compiler admission. |
| `rust/tcl-compiler/src/registry_invocation/source_structure/source_load.rs:295–379,382–412,415–438,440–476` | Static controls cover exact ancestor and non-inherited-source refusals, and assert inherited child CU has no Native entry/provider/compilation/body units. | Test assertions only; not executed in this audit. |

AuthoredSimulation is selected only from a retained native recipe whose authority equals `AuthoredSimulation`, after owner/input checks (`origin_inventory.rs:44–50,1674–1682,1717–1731`). Its source-graph path is separate (`source_graph.rs:278–294`) and its trace consumers refuse to infer absence (`execution_trace.rs:35–36,117–121,189–194`). Standalone remains explicit owner-based selection (`origin_inventory.rs:1684–1687,1733–1735`).

## Finding and limits

I found no additional missing branch-string/name fallback or implicit Native/compiler grant in this bounded source-origin/purpose audit. `InheritedPoint` is explicitly source-only and purpose-ineligible. Exact Logical checks in other consumers fail closed when purpose is absent; `SourceOriginKind::Authored`-only checks reject the inherited variant. This does not close N13. I did not duplicate the separately reported VM findings: (1) inherited-child validation omits requested context axes; (2) procedure-target lowering can retain the point while stamping a Native procedure frame.

This is static source review only: no Cargo, typecheck, tests, Clippy, source edits or execution. It does not establish runtime availability, parent-publication currency, source-effect depth/convergence, host integration, or complete N13/release approval. The stage-2 receipt itself lists combined typecheck/controls, source-effect availability/convergence, host publication/currentness, and strict Clippy/release gates as remaining.
