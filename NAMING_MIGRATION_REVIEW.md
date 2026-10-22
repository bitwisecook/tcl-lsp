# Naming migration review: LSP and compiler

**Reviewed branch:** `work`  
**Exact reviewed commit:** `fcb7927f02b5395753247b322fdd7be87ae5c731`  
**Commit subject:** `tclvm: nested-error errorInfo levels per release; lsp: the first workspace query's scan budget (#2487)`  
**Commit date:** 16 October 2026, 16:50:24 BST  
**Review date:** 22 October 2026  
**Scope:** the checked-out snapshot, including existing migration debt; this is not a claim that every finding was introduced by the latest commit. No implementation, test, configuration, dependency lockfile or Git history changes were made. This report is the sole repository addition.

All repository code links below are pinned to the reviewed commit. Priorities mean **P1: wrong identity, unsafe transformation, or materially inconsistent semantic analysis**; **P2: incorrect/missing editor information, bounded completeness gaps, or significant scaling problems**; **P3: narrower maintenance/API debt**. A priority does not imply that every input triggers the issue.

## Assessment

**The migration is substantial but incomplete. It does not yet meet the requirement that dialect-aware naming be authoritative for every analysis.** Several layers share spelling helpers while independently deciding which entity a spelling denotes. Those decisions demonstrably disagree. The most serious consequences are a compiler optimisation that changes `LOCAL` to `GLOBAL`, source renames that change the called command or break execution, distinct C Tcl commands collapsed into one identity, and JimTcl names incorrectly merged using C Tcl's colon rules. The SpecTcl follow-up also demonstrates a template registering the wrong command and a live hook producing an incorrect optimisation replacement (N17).

There is valuable infrastructure to retain: registry-owned command semantics, explicit lexical dialect axes, recorded invocation candidates, namespace/import lifecycle facts, source reanalysis, a shared source-order graph, incremental reverse-reference indexes, and careful Salsa memory management. The LSP-facing facts need one authoritative naming context and unambiguous identities. VM/runtime internals may keep their existing representation while preserving correct SpecTcl behavior.

The highest-return sequence for the **LSP-first release** is: contain unsafe transformations; establish family-aware identity and occurrence resolution in the LSP's production dependencies; make its providers and package/source views consume those facts; correct semantic invalidation; then address measured release-performance failures. Merely changing the common string canonicaliser will not fix the findings below. The broader runtime/backend migration and optional storage redesign can follow the release; the boundary and acceptance criteria are made explicit next.

## Editor-facing LSP release scope and completion criteria

**Release goal:** ship the **editor-facing LSP** naming work. Its analyses must share dialect-aware naming, and SpecTcl—including the EDA, JimTcl and other supported packs—must function completely for that editor experience. **Completing the VM/runtime naming migration is not a release requirement, even for internal paths SpecTcl executes.** Existing implementations and compatibility paths may remain; fix their behavior only where needed to make the LSP and packs work correctly. Return to the rest of the migration after this release.

This report is a technical worklist and proposed release checklist for that goal. **The reviewed commit is not ready on the evidence here.** The original audit alone was not a complete release checklist: it mixed necessary LSP corrections with longer-term architecture and performance recommendations. This section separates them. No implementation fixes are implied by this follow-up; executed checks and outstanding gates are distinguished below.

### A. Draw the boundary by production use

Include command and variable identity, occurrence recognition, namespace/import/alias/lifecycle resolution, package and source views, registry/pack/stub context, incremental invalidation, and every shipped LSP provider consuming them. Definition/declaration, type definition/implementation, hover, references/highlights, prepare/rename/linked editing, call/type hierarchy, signatures/inlay hints, semantic tokens, symbols, code lenses, diagnostics and refactors must agree where they describe the same occurrence. Completion legitimately returns several candidates; navigation may expose uncertainty; neither is permission for rename or compiler analysis to select an unrelated definite target.

**Compiler code on those paths is part of this release.** Ordinary deep diagnostics call compiler checks through [server lib.rs:5972–5988](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L5972-L5988), [server lib.rs:4563](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L4563) and [DB lib.rs:3888–3935](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L3888-L3935). The analyser also shares the compilation unit ([DB lib.rs:3806–3807](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L3806-L3807)); quick fixes rebuild compiler checks ([server lib.rs:23386–23398](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L23386-L23398)). N08's pre-CFG inliner and N09's type/call/taint decisions therefore cannot be deferred merely because their files are in `tcl-compiler`. Disabling optimiser suggestions alone does not contain a pass already run while constructing the compilation unit.

Source-producing commands also count. `optimiseDocument` calls the optimiser ([server lib.rs:17219–17220](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L17219-L17220)). `minifyDocument` accepts compact/aggressive/isolated options and returns replacement source ([server lib.rs:16952–17034](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L16952-L17034)); both [desktop extension.ts:1099–1118](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/editors/vscode/src/extension.ts#L1099-L1118) and [browser extensionBrowser.ts:743–756](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/editors/vscode/src/extensionBrowser.ts#L743-L756) expose isolated name compaction. N15 is consequently an LSP release issue. A temporary restriction must be enforced by the server, since removing an editor menu does not remove `workspace/executeCommand` access.

**VM/runtime migration can wait, including internal paths used by SpecTcl. Functional defects cannot.** N17 is a blocker because it silently registers the wrong command and produces a wrong hook result, not because its code has an old naming design. A targeted correctness fix is sufficient; it need not complete that subsystem's migration. The editor's compiler-explorer commands also expose backend output ([server lib.rs:18692–18718](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L18692-L18718)); retain correct behavior there without turning their call graph into a demand to migrate every backend component. The release boundary is user-visible behavior and the LSP's shared semantic facts.

### B. Disposition of every finding

“Required” below means necessary for the requested complete LSP migration. A narrow, explicitly documented restriction can make an interim release safe, but does not turn an unsupported ordinary case into completed functionality. For dynamic or genuinely ambiguous Tcl, a shared unknown result and refusal to edit are correct behavior. A blanket refusal of supported static namespaces/packages/renames is not a substitute for migrating them.

| Finding | LSP-first disposition | Concrete completion condition |
|---|---|---|
| **N01: colliding/display identities** | Required; wrong-target blocker. | Distinct command slots remain distinct end to end, empty tails survive indexing, and declaration/addressability rules respect the selected release. Display text is never reparsed as identity. |
| **N02: Jim policy** | Required for the supported Jim profile. | Shared naming preserves Jim's interior-colon distinctions, operation-specific rename lookup and import behavior; unavailable Tcl features are not silently enabled. Jim remains a separate oracle family. |
| **N03: variable recognition/profile** | Required; edit-safety blocker. | All providers use the selected profile's occurrence spans and variable rules, including Unicode, Tcl 9 braces, literal suffix boundaries and scalar parentheses. No fallback to a default profile when context is available. |
| **N04: imported source rename** | Required; unsafe-edit blocker. | Source transformation updates all derived import/export/call spellings or refuses the entire edit when completeness cannot be proven; edited fixtures execute correctly in the matching oracle. |
| **N05: forced-import selection** | Required; unsafe-edit blocker. | Reference enumeration and rename consume the same binding and lifetime; renaming a shadowed command does not retarget a call. |
| **N06: collision checks** | Required; unsafe-edit blocker. | Destination occupancy/capture checks cover command kinds, methods and relevant workspace views, independently of which occurrence starts the request. |
| **N07: speculative proc-variable binding** | Required; unsafe-edit blocker. | Speculative navigation never becomes proven binding for references/rename; an unbound proc local cannot rename an unrelated global. |
| **N08: inliner and nested lowering** | Required through ordinary compilation-unit use. | Use the complete existence context before testing inline eligibility; preserve dialect/declaration context in nested lowering, or decline that expansion. Assert original/transformed behavior and downstream analysis. |
| **N09: type/call/taint resolution** | Required through diagnostics and edits. | Compiler facts resolve the same occurrence/view as the analyser. Missing context yields conservative uncertainty, not a different definite class/callee. |
| **N10: OO relation resolution** | Required for supported OO profiles. | Definition, MRO and method consumers use relation-site identity and proper shadowing; no arbitrary unique-tail rescue for a missing qualified class. |
| **N11: package selection/autoload** | Required for cross-package correctness. | Eligibility, version selection and provider provenance agree across diagnostics/navigation/edits; request order cannot leak another application's selected release; Unicode survives qualification. The multi-request scenario still needs integration proof. |
| **N12: lifecycle invalidation** | Required; stale-target blocker. | After moving a declaration across a retirement, the first answer for the committed revision matches a fresh index/server, including open-buffer and closed-file updates. |
| **N13: source environment/convergence** | Required for supported static source graphs. | Source views carry inherited naming state; literal chains converge without silent fixed-depth truncation; all consumers see the same complete or explicitly incomplete view. The server's four-round path still needs integration proof. |
| **N14: callback and semantic roles** | Required for full provider migration. | Arity and argument-role decisions use resolved command identities; unrelated same-tail commands cannot supply facts. This is lower immediate risk than a source-corrupting edit, but still violates the shared-source goal. |
| **N15: minifier renaming** | Required wherever the shipped command permits affected modes. | Fix isolated procedure compaction or explicitly refuse it server-side in both compact and aggressive modes. Aggressive mode must separately satisfy N08/N09. Plain whitespace minification is not implicated by N15's two proofs; other modes still need their own regressions. |
| **N16: provider fallbacks** | Correctness portion required; bounded coverage gaps may follow. | Remove wrong alias/type-definition targets and unavailable-dialect signatures. Extra abbreviation support and richer completion candidate presentation can be scheduled separately if no incorrect identity is asserted and limits are documented. |
| **N17: backend variable scanner** | **Functional fix required through SpecTcl; migration deferred.** | Correct the wrong template/hook result and add the concrete regressions below. Existing bytecode/VM architecture may remain, including this execution path. No whole-subsystem naming migration is required. |

The additional seams in sections 3.2–3.3 also need a disposition: workspace naming must preserve pack/stub environment; workspace-class reanalysis must retain the same context; method-barrier selection must be proven conservative or the affected rewrite must abstain. The method barrier is reached by optimiser propagation ([propagation.rs:87](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/optimiser/propagation.rs#L87), [propagation.rs:1135–1161](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/optimiser/propagation.rs#L1135-L1161)). Linked editing also has a literal self-name fallback when settlement is absent ([linked_editing_range.rs:93–104](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/linked_editing_range.rs#L93-L104)); prove its conditions safe or abstain. It is an editing surface, not a presentation exception. Extract/inline refactors likewise remain in scope ([code_actions.rs:1703–1726](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/code_actions.rs#L1703-L1726)). These static risks require targeted proof; they must not be reported as additional executed failures. Unused bridge plumbing and a helper whose complete projection repairs its result can remain recorded migration debt where they do not introduce an independent LSP naming decision.

### C. Minimum implementation shape; what can wait

The release needs **one semantic owner and consistent inputs**, not necessarily the exact new API or storage layout suggested in section 3.4. Existing analysis/query/index infrastructure can provide that owner. A typed or injectively encoded identity carried across compatibility adapters is acceptable if adapters neither reinterpret its spelling nor choose another target. Per-occurrence results need enough context to distinguish family/release, frame/namespace, path/import/lifecycle state, package/source view, source revision and registry/pack generation. Consumers may project those facts differently; they may not independently settle the same occurrence using bare-tail or global-first fallbacks.

For packages, a narrowly scoped solution can retain provider provenance and filter materialised facts by the requesting document's eligible provider view before an existing-command shortcut. Share file analysis and keep provider membership separate. For `source`, extend existing view inputs to carry the required inherited namespace/path and environment facts, and use dependency-driven convergence. Neither repair requires a general interpreter simulator. Unknown `eval`, custom C resolvers, dynamic package loading and ambiguous execution contexts may remain unresolved, provided that uncertainty propagates consistently to analysis and edits.

The following can follow the LSP release when the preceding invariants hold:

- VM/runtime/native naming migration, **including paths used by SpecTcl**, provided the supported pack behavior and editor results are correct. A demonstrated defect requires a functional fix; being on the execution path does not by itself require migration.
- Full variable-storage/place migration beyond the facts needed by LSP-facing analysis; an unwired `place_bridge` need not be completed just for its own sake.
- A universal interning arena, comprehensive clone elimination, parent/LCA source-graph storage and other layout redesigns if shipping-build latency/memory evidence does not require them now. Introducing structured identities must still avoid obvious per-occurrence path duplication.
- Consolidating every name-producing tool behind a new transaction planner. Until then, each enabled LSP transformation must use authoritative naming facts and independently meet the same safety contract; a second incompatible resolver is not an acceptable bridge.
- Bounded presentation/coverage improvements and unrelated compiler cleanups, recorded explicitly rather than included in a blanket claim that every migration is finished.

### C.1 SpecTcl and its packs must work for the editor release

SpecTcl is both an authoring dialect and the extension mechanism supplying editor analysis. Its valid pack programs and hooks must run, their facts must reach the intended analyses, and editing a pack must replace those facts consistently. EDA, JimTcl and the other supported packs are explicit acceptance cases. This is a functional dependency of the editor release, not another migration project: even the VM/runtime paths executing these packs can stay unmigrated. Disabling valid trusted hooks or silently falling back to an empty/base registry would fail the requirement.

The concrete production dependency is `evaluate_pack_with`/`evaluate_pack_in` → `run_pack_program` → `TclVmEngine` → `BytecodeCompileService` → VM execution ([loader/eval.rs:1640–1693](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/src/loader/eval.rs#L1640-L1693), [pack_eval.rs:395–461](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spec-hooks/src/pack_eval.rs#L395-L461), [engine lib.rs:460](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-engine-tclvm/src/lib.rs#L460)). Live hook plans are published once and installed into each worker's host, with evaluator-generation changes on reload ([hooks.rs:515–596](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/src/hooks.rs#L515-L596)). Keep that shared ownership; verify that every LSP analysis path receives the appropriate environment and host generation.

| Required SpecTcl behavior | Acceptance evidence |
|---|---|
| **Pack evaluation and compatibility** | Supported legacy 1.x packs and 2.x vocabulary retain their documented meaning; upgrading shipped packs preserves facts. Static fast-path and evaluated loading agree for valid templates, helper procedures, namespaces, variable substitutions and included fragments. N17's wrong-name template must register `good`, with its braced control agreeing. |
| **Live hooks actually execute** | Valid supported hook families are installed and exercised through the production registry/worker path, including variable/argument roles, command-prefix timing and constant-fold/evaluator behavior where declared. Assert meaningful expected answers, not merely absence of crashes. Existing budget/error/trust-based abstention remains valid; unexpected abstention for a valid enabled hook is a functional regression. |
| **Correct dialect and naming context** | The pack DSL version, host execution policy, and analysed document's language family/release are distinct inputs. Release-sensitive derived bodies must use the document's effective context and produce the matching C Tcl/Jim result where supported; otherwise decline a speculative optimisation. A Tcl-hosted hook is not evidence that Jim and C naming are interchangeable. Pack-defined commands, aliases/imports and roles must obey the same identity source as ordinary commands. |
| **All analysis paths see the pack** | Ordinary and incremental analysis, signatures/tokens, source-seeded documents, workspace-class reanalysis, workspace existence/import checks and compiler diagnostics/edits consume the same applicable pack/stub facts. A correct hover with a base-registry compiler answer is not functional integration. |
| **Reload, generations and cache coherence** | Add/change/remove a pack or included fragment, change its target/trust configuration, and query from several workers and open/closed files without editing document text. Retire old registry, hook and memoised answers; preserve each document's effective dialect/environment. Compare settled answers with a fresh server. Respect the documented pack scope: current folder-relative discovery contributes to a workspace-wide pack set, so per-folder pack isolation is not an invented release requirement. |
| **SpecTcl authoring and shipped hosts** | `.tclspec` detection, scoped DSL vocabulary, completion/signatures, hover and diagnostics remain usable. Body-local vocabulary must not shadow unrelated Tcl commands outside that body. Native filesystem and browser VFS loading/reload both work where shipped. |
| **Existing containment and cost contracts** | Preserve execution budgets, call confinement, error/quarantine reporting and the intended trust policy while ordinary supported packs remain functional. Measure cold load, warm reuse, first worker hook installation and repeated reload memory; do not let a naming change invalidate shape caches on every request or retain every obsolete host generation. |

The functional boundary is the **supported SpecTcl execution contract**. The pack evaluator deliberately permits deterministic helpers such as `proc`, `namespace`, `global`, `variable` and arrays while excluding `rename`, `upvar`, `interp` and arbitrary ambient I/O ([pack_eval.rs:58–92](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spec-hooks/src/pack_eval.rs#L58-L92)); DSL includes are a separate loader facility. There is no requirement here to enable those excluded operations. Likewise, declared implementation/`Evaluate` hooks request release pinning ([hooks.rs:130–134](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/src/hooks.rs#L130-L134)); the engine explicitly refuses Jim or an unknown/unpinnable environment rather than evaluating under its default C Tcl profile ([engine lib.rs:650–675](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-engine-tclvm/src/lib.rs#L650-L675)). Preserve that conservative refusal while keeping supported Jim pack surfaces and LSP naming functional. This does not require implementing a Jim VM before release. Other hook families run under the pack host's policy and receive document context; a DSL version number must not substitute for either policy.

Two integration seams deserve explicit regression coverage beyond ordinary pack hover: source rehoming constructs a fresh analyser without the normal pack overlay inputs ([server lib.rs:22257–22314](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L22257-L22314)); workspace-class reanalysis has a separately constructed context/cache (section 3.2). Workspace import checks use the plain dialect registry path identified in section 3.2. These are **static context-loss risks**, not additional executed SpecTcl failures. Extend section D's environment-change scenario with a pack-defined naming/alias/role operation in a sourced closed file and a class consumer, plus a live hook whose changed answer distinguishes generations. Verify the effective environment and result rather than assuming registration implies consumption. Preserve the existing workspace-wide pack publication scope ([server lib.rs:11086–11121](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L11086-L11121), [server lib.rs:21272–21288](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L21272-L21288)) while testing each folder's applicable dialect against it.

`make test-spectcl-compat` is a mandatory functional gate for the implementation, not a deferred VM-migration gate. It selects `eval_loader`, `golden_packs`, `pack_source_e2e`, `spec_corpus` and `pack_is_real_tcl`, requiring the exact Tcl 9.0 reference ([Makefile:1172–1183](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/Makefile#L1172-L1183)). The last target is a Tcl syntax/parse acceptance check, **not general behavioral equivalence** of hooks or templates; preserve the executable oracle regressions as well. Existing authoring, environment/roster, trust, live-pack and cross-worker tests complement that gate. Add the newly demonstrated wrong-registration regression rather than treating the existing green suite as exhaustive proof. Section 6 records the compatibility tests actually run during this follow-up and their toolchain limits.

### C.2 Explicit EDA, JimTcl and other pack acceptance

The editor's production pack inventory includes **eight discoverable EDA packs and two unconditional Jim core packs**. The current EDA command counts are asserted in [eda_loadables.rs:72–109](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/tests/eda_loadables.rs#L72-L109); use those assertions rather than older counts in comments or corpus-baseline prose.

| Pack | Editor environment/role | Declared commands |
|---|---|---:|
| [eda_cadence.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_cadence.tclspec#L20) | `cadence-eda-tcl` | 77 |
| [eda_mentor.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_mentor.tclspec#L29) | `mentor-eda-tcl` | 69 |
| [eda_microchip.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_microchip.tclspec#L25) | `microchip-libero-eda-tcl` | 257 |
| [eda_quartus.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_quartus.tclspec#L44) | `intel-quartus-eda-tcl` | 77 |
| [eda_synopsys.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_synopsys.tclspec#L20) | `synopsys-eda-tcl` | 68 |
| [eda_xilinx.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/eda_xilinx.tclspec#L29) | `xilinx-eda-tcl` | 788 |
| [sdc_base.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/sdc_base.tclspec#L20) | Shared SDC facts across EDA environments | 86 |
| [upf.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/upf.tclspec#L28) | Shared UPF facts and executable editor hooks | 67 |
| [jim.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/core-surfaces/jim.tclspec#L58) | Jim inherited-command roster and release windows | Roster, no own commands |
| [jim-own-surface.tclspec](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/core-surfaces/jim-own-surface.tclspec#L55) | Jim's own commands, procedure forms and class semantics | 60 |

The eight EDA packs total **1,489 declared commands**. Their environment selection, shared SDC/UPF availability, exclusion of rival-vendor commands, arity, variable/body roles and native semantic descriptors must survive loading and reach the editor's providers. Jim's inherited and own surfaces must be ready on first document open, before any discovered pack reload, and must survive later publication/removal of discovered packs. Jim is loaded through the same SpecTcl loader but registered as a built-in family surface, independently of discovery ([core_surfaces.rs:19–99](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/src/core_surfaces.rs#L19-L99)). Keeping those facts loaded is necessary alongside fixing N02/N03's naming mismatches.

**The compatibility inventory does not include Jim's core packs.** Its manifest lists `specs` and two documentation/example directories ([shipped-pack-dirs.txt:1–8](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/data/shipped-pack-dirs.txt#L1-L8)), covering the eight EDA and sixteen example packs at this hash. Therefore a green `test-spectcl-compat` alone cannot establish Jim editor support. Preserve the dedicated `jim_document`, `jim_own_surface`, `jim_surface_roster`, `jim_generation_zero` and `jim_seed_after_publish` tests, plus `eda_loadables` and `eda_environment_packs`. Section 6.3 records their executed results.

“Packs run” has concrete meaning even for declarative surfaces. A current-loader inventory probe found no executable Tcl hook bodies in the two Jim core packs or the six vendor/SDC packs; their loaded facts and native semantic descriptors still drive editor analysis and must work. UPF has **17 executable bodies**: sixteen option-arity hooks and the `define_power_model` argument-role resolver (examples at [upf.tclspec:350–358](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/upf.tclspec#L350-L358), [upf.tclspec:815–827](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/specs/upf.tclspec#L815-L827)). None of these ten production packs declares a target-dependent `Evaluate` body, so the existing refusal to execute a Jim-pinned implementation does **not** itself disable the shipped Jim packs. It must not be used as an excuse to disable Jim surface loading, body analysis or naming.

For release, require observable results from the UPF option and body-role hooks. The current corpus gate requires at least one successful invocation per hooked **pack**, and rejects script errors; it does not require successful coverage of every body ([spec_corpus.rs:563–580](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/tests/spec_corpus.rs#L563-L580)). Add deterministic cases for the distinct hook behaviors and ensure every declared slot is exercised, with expected optional-value consumption/body traversal. A bound slot or a silent no-op is not proof that the feature works.

Use the existing editor protocol fixtures as the final acceptance layer: [Jim startup/analysis tests](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/jim.rs#L99-L207) and [bundled EDA command smoke](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/spec_packs.rs#L931). Extend the latter beyond Vivado to all six EDA environments, and apply section D's cross-file/source/rename/reload checks to pack-defined roles and naming. Exercise installed/embedded pack delivery and browser VFS where shipped. This does not require running proprietary vendor EDA tools or completing a standalone Tcl VM.

Other supported workspace, package and user packs must retain all supported declared facts and live hooks through the same path. The sixteen documentation/example packs have enumerated degradation notices, including an unbindable method hook and unloaded schema rows ([spec_corpus_baseline.txt:17–65](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-spectcl/tests/spec_corpus_baseline.txt#L17-L65)). A green baseline is not a claim that those drafts are fully functional. Any such feature offered as supported editor functionality needs its behavior completed; documented draft-only schema work may follow the release. The production EDA packs are held to clean loading, and their requirements must not be weakened by adding tolerated degradation notices.

### D. Missing integrated release proofs

Retain the reproduced lower-level cases as deterministic regressions in their existing owner crates. Add the following focused server scenarios; a Cartesian product of every LSP method and every runtime is unnecessary. Share fixtures and assert identity, locations, relevant diagnostics, references and complete rename/refusal outcomes. The full shared oracle matrix must still cover C Tcl 8.4–9.1 and the pinned latest Jim, with their expected differences.

| Scenario | Required proof and why existing coverage is insufficient |
|---|---|
| **Package selection in one server process** | Place two releases outside the workspace scan root; two entry points require different releases. Query A→B→A and B→A→B, include an unavailable higher registration and Unicode autoload name, then change the requirement/library path. Each document keeps its eligible provider and old providers disappear. The existing package-definition test creates a fresh backend for each requirement, so it cannot prove isolation ([server lib.rs:35624–35735](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L35624-L35735)). |
| **Inherited source state and deep chains** | A root's namespace path resolves calls in an unopened child; extend a literal chain beyond both limits (for example 7–12 files), including a source-directory constant after hop five and namespace composition after hop four. Source one child under two known namespaces, then remove/change/rename an edge. The first completed reconciliation after independent readiness must expose all expected views and remove obsolete ones. Basic closed-source rename coverage exists ([tests/e2e/rename.rs:813](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/rename.rs#L813)); it does not prove these inherited/deep cases. |
| **Warm versus fresh lifecycle answers** | Use N12's valid before/after scripts through `didChange`, then repeat with a watched change to a closed provider. Warm definitions/references/diagnostics before the edit and compare the first new-revision answer with a fresh server. Extend existing closed-file reindex coverage ([server lib.rs:37424](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L37424)). |
| **Environment changes without text edits** | Warm command/import/callback results under a folder dialect and custom pack/stub; change/remove/restore the environment while text stays fixed, including an unopened provider and source-seeded consumer. All providers and workspace gates must use the new generation, with no late old-state publication. Existing live pack tests ([tests/e2e/spec_packs.rs:263](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/spec_packs.rs#L263), [tests/e2e/spec_packs.rs:873](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/spec_packs.rs#L873)) are a foundation, not proof of the workspace registry seam. |
| **Actual protocol edits and provider agreement** | Exercise declaration-origin and call-origin rename, open and closed files, imports/aliases, class/method collisions and Unicode/UTF-16 ranges through the server. Apply the returned workspace edit to scratch source and run the matching interpreter. A correct core edit set alone does not validate URI/range lifting, merging, document versions or the server's added cross-file edits. Check compiler diagnostics/quick fixes and enabled optimise/minify commands against the same identities. |
| **Shipped transports and editor hosts** | Cover native JSON-RPC plus browser/WASM and WASI when included in the release artifact. Verify VFS/file updates, dialect/pack propagation and edit application in the relevant host. Shared Rust code is valuable but does not prove transport parity. Reuse the existing native `e2e` target and host suites, rather than creating an unrelated test framework. |

**Do not let retrying a query repair the state being tested.** `await_query_settled` repeats position requests while startup work finishes ([tests/e2e/common/mod.rs:1338–1383](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/tests/e2e/common/mod.rs#L1338-L1383)). That is useful for startup readiness, but a depth-limit regression must await readiness independently and assert one completed reconciliation; repeated naming requests could advance more rounds and hide truncation. Similarly, compare results for the same committed semantic revision, not an intentionally provisional fast tier against a settled fresh analysis.

The existing repository gates remain the release mechanism: prescribed-toolchain `make rust-check`, `make prep-pr` for implementation changes, and the applicable deep CI aggregates on the exact shipping commit, including `rust-tests`, `lsp-e2e`, `test-ext`, `test-ext-web` and **`spectcl-compat`** ([Makefile:1159–1183](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/Makefile#L1159-L1183), [test-tiers-and-ci-gates.md:7–32](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/docs/design/contracts/test-tiers-and-ci-gates.md#L7-L32)). Browser and WASI server harnesses are provided in [Makefile:2133–2156](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/Makefile#L2133-L2156). The prescribed-toolchain and full server/host gates were **not** run as part of this report-only scope update; `prep-pr` includes formatting/code generation and would exceed that scope. Section 6 distinguishes the 81 naming tests and the additional SpecTcl tests actually run from the remaining release proof.

### E. Performance and memory are measured exit criteria

Section 4 identifies real scaling costs, but its debug CPU timings and helper allocation counts are not shipping-server latency or RSS measurements. Before release, measure the actual release builds on representative small/large projects and the targeted scaling fixtures, cold and warm. Record open-to-first-tokens, edit-to-response, full diagnostics convergence, references/rename and code-lens latency; include cancellation during edits and overlapping requests. Measure peak and retained memory across repeated open/edit/close, package/environment changes and source views, including the browser host where shipped.

Use the repository's existing latency contract: typical first tokens target single-digit milliseconds, and regression beyond run noise blocks a change ([engineering-guide.md:36–60](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/docs/design/rust/engineering-guide.md#L36-L60)). The 40 ms fast-tier budget is a delivery mechanism, not a universal request SLA or proof that background work is cheap ([lsp-performance.md:79–140](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/docs/design/rust/lsp-performance.md#L79-L140)). Establish and record acceptable workspace sizes and memory/long-request bounds for the shipping targets; this review did not measure them and does not invent a numeric memory limit.

Prioritise the repeated scope/body scans in code lenses and invocation indexing, repeated workspace-existence scans, reference/edit deduplication and import-heavy updates. Exercise source-depth storage separately so an unusual stress graph is not confused with a typical project. Fix demonstrated user-scale stalls or memory failures before shipment; not every quadratic stress path mandates its proposed replacement before this phase can finish. Conversely, passing the current baseline is insufficient if that baseline itself stalls on supported representative workloads. Deferring an optimisation needs measurements and a bounded remaining risk, not only a low finding priority.

### F. Release sign-off checklist

All boxes below are **pending implementation/validation**, not completed by this review:

- [ ] Each row in section B has a recorded fix, tested conservative restriction, or justified out-of-scope disposition. No known wrong-target/unsafe-edit case remains enabled in the claimed feature set.
- [ ] Supported static LSP cases use authoritative dialect/environment-aware identities end to end; compatibility adapters do not introduce independent settlement. Dynamic/incomplete cases share explicit uncertainty.
- [ ] SpecTcl meets sections C.1–C.2 for EDA, JimTcl and other supported packs: correct evaluation, working live hooks, authoring and consistent LSP facts/reloads. Production pack coverage includes Jim's separate core inventory and UPF's executable hooks. N17's functional defect is fixed and regressions pass. Existing VM/runtime paths may remain unmigrated; functionality is the criterion.
- [ ] The six integrated scenarios in section D pass, alongside owner-level regressions and family/release oracle vectors. Removed limitations are demonstrated through the server, not only through direct helper calls.
- [ ] All enabled source transformations preserve behavior in the matched oracle fixtures; collision checks, import-derived spellings, edit ranges and revision handling are covered.
- [ ] Shipping-build latency, cancellation and memory results meet the recorded contract and expose no unresolved representative-workload failure.
- [ ] Required repository/host gates pass for the **new shipping commit**, using the prescribed toolchain. Record its exact hash separately from this report's reviewed `fcb7927f02b5395753247b322fdd7be87ae5c731`.
- [ ] Release notes/contracts state any temporary restrictions and the deferred runtime/backend scope. “LSP migration complete” is reserved for the supported LSP feature set actually meeting the shared-naming contract.

This makes the report usable for the staged goal: the work needed by LSP and SpecTcl users is explicit, the remaining compiler/runtime migration is bounded, and missing proofs are visible. It is not an assertion that static review discovered every possible defect, or that the current branch has passed release sign-off.

## 1. Ground truth and validation method

### 1.1 Oracle versions

The reference is executable C Tcl across the supported release families and the latest upstream JimTcl commit observed during this review, supplemented by the corresponding C source. JimTcl is a separate language family; its reported `0.84` alone does not identify the exact source.

| Interpreter | Upstream tag/reference | Exact source commit |
|---|---|---|
| C Tcl 8.4.20 | `core-8-4-20` | `9ccfe9d1b35741ff7323837f6485ffe48b06fad9` |
| C Tcl 8.5.19 | `core-8-5-19` | `160d612a6b2b1c2c0db27236d648b7bc1364570c` |
| C Tcl 8.6.18 | `core-8-6-18` | `515167c897a7c8066abfbd71e8fe7bd8bf3569de` |
| C Tcl 9.0.4 | `core-9-0-4` | `c655b4770b1d6d32a8cbffd6cef59db6029fe19e` |
| C Tcl 9.1.0 | `core-9-1-0` | `0c6bad08d91fa2ad51f05a50a41f8c1ea82ce985` |
| Latest JimTcl, reports 0.84 | upstream `master`/`HEAD` on review date | `5bac7c99ad65864c87da513e22e2f01703fa4e03` |

The five Tcl patchlevels match [rust/tcl-dialect/data/reference-toolchains.tsv:1–6](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-dialect/data/reference-toolchains.tsv#L1-L6). Jim was built from [the pinned upstream commit](https://github.com/msteveb/jimtcl/tree/5bac7c99ad65864c87da513e22e2f01703fa4e03), with the default build including namespace support. Features compiled out of a different Jim build need their own capability context. Initial exploratory checks used system C Tcl 8.6.16; the release-matrix results below supersede that narrower evidence where applicable.

The observed semantic differences relevant to this review are:

| Probe | C 8.4 | C 8.5 | C 8.6 | C 9.0 | C 9.1 | Latest Jim |
|---|---|---|---|---|---|---|
| `a::p` versus `a:::p` command keys | Same | Same | Same | Same | Same | Distinct |
| Create non-global procedure `:p` | Rejected | Rejected | Accepted | Accepted | Accepted | Accepted |
| `a:`/`p` versus `a`/`:p` fixture | Second declaration rejected | Second rejected | Distinct commands | Distinct commands | Distinct commands | One command key |
| `x::`, `x:::`, `::x::` for namespace's empty-tail command | Same | Same | Same | Same | Same | `x:::` is distinct/misses |
| Bare `$café` | Reads `caf` + literal `é` | Same | Same | Same | Same | Reads `café` |
| `${a{b}}` with both candidate variables defined | Reads `a{b` + literal `}` | Same | Same | Reads `a{b}` | Same | Reads `a{b` + literal `}` |
| Missing bare variable in namespace frame falls back globally | Yes | Yes | Yes | No | No | No |
| Missing bare variable in proc frame falls back globally | No | No | No | No | No | No |
| `namespace path` / unique `pat` abbreviation | Unavailable | Available | Available | Available | Available | Unavailable |
| `rename p q` inside `a`, only global `p` exists | Finds global `p` | Same | Same | Same | Same | Fails looking for `a::p` |
| Existing import remains callable after live source-command rename | Yes | Yes | Yes | Yes | Yes | No; old target name is missing |
| Native TclOO fixtures | Unavailable | Unavailable | Available | Available | Available | Unavailable in tested build |

“Unavailable” is a feature distinction, not an oracle failure. Source-refactoring failures below were checked separately from live table mutation. The shared scalar-parenthesis fixtures `a(b` and `a(b)c` are legal on all five C releases, and Unicode `auto_qualify` preserves its spelling on all five. Source inherits an existing namespace path on C Tcl 8.5–9.1. Byte/encoding details of astral source characters differ on older Tcl releases; the basic Unicode comparison above deliberately uses `é` rather than assuming identical supplementary-plane encoding.

### 1.2 Evidence levels

- **Executed current-source evidence:** compiler/LSP libraries were freshly built from the reviewed snapshot into `/tmp`, and small external harnesses called their public APIs. Returned source edits were applied to scratch files and evaluated in fresh interpreters. No test source was added to the repository.
- **Executed isolated-source evidence:** a few pure helpers were extracted verbatim and compiled in temporary harnesses. These isolate an algorithm or parser; they are not full-server tests.
- **Static integration evidence:** call paths, missing context or dependency projections were inspected. Such findings are labelled where the full server scenario was not executed.
- **Design limitations:** unsupported dynamic cases or incomplete context contracts are separated from reproduced defects.

The installed Rust compiler was **1.98.1**, while the workspace requests **1.99.1**. Local Cargo validation used `--ignore-rust-version`, preserving `Cargo.lock` with `--locked`. The relevant libraries compiled successfully; this is not a claim that the prescribed 1.99.1 CI gates were run. Repository `target/debug` artifacts predated the reviewed commit and were excluded as proof.

## 2. Findings requiring correction

### N01 — P1: static identities still use a non-injective display string

**Executed current-source defect; C Tcl 8.6–9.1 example.** `StaticCommandSlot` and an injective length-framed encoding exist in [rust/tcl-runtime-api/src/command_identity.rs:15–67](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-runtime-api/src/command_identity.rs#L15-L67), including a test explicitly showing that display strings collide. The compiler/LSP do not consume these types. `AnalysisResult` still keys procedures/classes by strings ([rust/tcl-compiler/src/analyser/types.rs:2113–2150](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/types.rs#L2113-L2150)); procedure intake constructs a string, derives its simple name by parsing that string, and treats a matching key as redefinition ([rust/tcl-compiler/src/analyser/handlers.rs:2091–2092](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/handlers.rs#L2091-L2092), [rust/tcl-compiler/src/analyser/handlers.rs:2272–2285](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/handlers.rs#L2272-L2285)).

```tcl
namespace eval a: {proc p {} {return LEFT}}
namespace eval a  {proc :p {} {return RIGHT}}
puts [namespace eval a: {p}]
puts [namespace eval a  {:p}]
```

C Tcl 8.6+ has two distinct slots and prints `LEFT`, `RIGHT`. Both render as `::a:::p`. The current analyser retains **one** `all_procs` entry, derives the simple name `p` for the `:p` declaration, and emits a false unknown-command diagnostic for `:p`. This affects every downstream table using that key, including navigation, references, rename and call analysis. Tcl 8.4/8.5 reject the non-global `:p` procedure declaration; accepting it there is a separate missing release check. Jim's flat command-name rules produce a different equivalence relation, addressed in N02.

The workspace also re-parses already-constructed identities as written namespace names. `index_command_links` normalises alias/rename names and targets ([rust/tcl-lsp-core/src/workspace_index.rs:1738–1770](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1738-L1770)); destruction and variable-reference intake do likewise ([rust/tcl-lsp-core/src/workspace_index.rs:1583–1632](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1583-L1632)). Namespace normalisation intentionally drops trailing separators ([rust/tcl-syntax/src/naming.rs:961–976](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L961-L976)), so a legal empty-tail command is changed:

```tcl
namespace eval n {}
proc n:: {} {return EMPTY}
interp alias {} a {} n::
puts [a]
```

The fresh analyser records `::n::`; the index changes the alias target to `::n`. The runtime command and the indexed target are different entities.

**Change:** carry typed namespace/command identity from declaration intake through signatures, scopes, invocation facts, links and workspace indexes. Keep written spelling, semantic identity and rendered display separate. A compatibility encoding must never be passed to a written-name parser. Intern namespace paths and simple names rather than copying a `Vec<String>` into every occurrence. Correct the present-tense claim that this migration is already complete in [docs/design/analysis/name-resolution.md:107–128](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/docs/design/analysis/name-resolution.md#L107-L128).

**Addressability follow-up:** `is_absolutely_addressable` and W314 only check all-colon components ([rust/tcl-syntax/src/naming.rs:381–383](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L381-L383), [rust/tcl-compiler/src/analyser/handlers.rs:1866–1874](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/handlers.rs#L1866-L1874), [rust/tcl-compiler/src/analyser/handlers.rs:1968–1972](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/handlers.rs#L1968-L1972)). In C Tcl, `:p` and contents of a namespace ending in a literal `:` also lack the expected absolute spelling; `p:` and an interior `a:b` must remain distinguishable. Derive addressability from the structured identity under its family/release policy, and use the same result for diagnostics and edits.

### N02 — P1: the shared command-name algorithm has no JimTcl policy

**Executed against latest Jim and the fresh analyser.** `canonical_written_command`, `qualify` and `command_resolution_candidates` assume C Tcl's colon-run grammar and accept no family context ([rust/tcl-syntax/src/naming.rs:168–212](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L168-L212), [rust/tcl-syntax/src/naming.rs:1114–1201](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L1114-L1201)).

```tcl
namespace eval a {}
proc a::p  {} {return TWO}
proc a:::p {} {return THREE}
puts [list [a::p] [a:::p]]
```

C Tcl collapses the internal separator run and prints `THREE THREE`; latest Jim preserves the internal spelling and prints `TWO THREE`. Analysing as `jim` still produces one `::a::p` entry and sends both calls to the second declaration. This is a wrong identity, not just a missing Jim command spec.

Jim's [command hash/equality strips a leading run of two or more colons but preserves interior bytes](https://github.com/msteveb/jimtcl/blob/5bac7c99ad65864c87da513e22e2f01703fa4e03/jim.c#L4021-L4074). Its [namespace canonicaliser](https://github.com/msteveb/jimtcl/blob/5bac7c99ad65864c87da513e22e2f01703fa4e03/jim-namespace.c#L61-L98) and [rename implementation](https://github.com/msteveb/jimtcl/blob/5bac7c99ad65864c87da513e22e2f01703fa4e03/jim.c#L4462-L4523) also differ from C Tcl. For example, renaming an otherwise globally callable `p` from namespace `a` fails in Jim if `a::p` is absent; C Tcl source lookup can fall back globally. Latest Jim has no `namespace path` subcommand.

**Change:** add a mandatory naming policy derived from the existing resolved environment, covering family, release and relevant capabilities. Share the owner/API and facts, but let that owner implement distinct canonicalisation, equality and operation-specific resolution rules. Do not impose C Tcl's structured namespace-slot equivalence on Jim's flat command table. Distinguish command invocation, command creation, rename-source lookup, rename-destination creation, alias targeting and variable lookup. A single optional Tcl version number cannot express these differences.

### N03 — P1: variable cursor lookup bypasses the dialect-aware lexical owner

**Executed current-source defect.** The shared byte scanner already distinguishes C Tcl's ASCII bare variable characters from Jim's high-byte rule ([rust/tcl-core-types/src/naming.rs:1–50](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-core-types/src/naming.rs#L1-L50)). LSP cursor discovery instead has its own Unicode `char::is_alphanumeric` scanner and a braced-name finder without a dialect argument ([rust/tcl-lsp-core/src/hover.rs:107–123](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/hover.rs#L107-L123), [rust/tcl-lsp-core/src/hover.rs:2949–3093](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/hover.rs#L2949-L3093)).

```tcl
set caf A
set café B
puts $café
```

C Tcl reads `caf` and prints `Aé`; `${café}` would name the second variable. Current definition/references select the `café` declaration. Rename from that read edits only the unrelated declaration and leaves the clicked occurrence unchanged. Latest Jim really does read `café`, but it also accepts non-alphanumeric high-byte characters such as emoji, so the LSP's Unicode-alphanumeric approximation is not a correct Jim parser either.

The same cursor helper truncates Tcl 9 `${a{b}}` to `a{b` or fails to recognise it, although Tcl 9 names `a{b}`. It can return `foo` for a cursor on the literal `bar` in `$foo-bar`, because it does not require the cursor to fall within the parsed variable extent. Rename additionally manufactures `profile_for_dialect("")` instead of threading the resolved document profile through ([rust/tcl-lsp-core/src/rename.rs:240–254](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L240-L254), [rust/tcl-lsp-core/src/rename.rs:904](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L904)).

**Change:** resolve the cursor to a dialect-tokenised occurrence containing its exact span, source spelling, parsed scalar/element form and cell identity. All providers should consume that occurrence. Require the cursor to belong to its span. Preserve UTF-16 position conversion, C Tcl release-specific braced closing, Jim high bytes, nested indices and expression sugar. Do not merely replace `is_alphanumeric` with ASCII without threading the family.

**Related parser debt:** `is_braced_indirect_array_ref` hand-scans with Tcl 9-style nesting and no dialect ([rust/tcl-syntax/src/naming.rs:908–945](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L908-L945)). It causes false W212 for valid 8.x/Jim `${a{b}(status)` usage via [rust/tcl-compiler/src/analyser/diagnostics/usage.rs:979](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/diagnostics/usage.rs#L979). `normalise_var_name_for_style` strips at the first `(` even without a final `)`, disagreeing with `split_element_ref` for legal scalars such as `a(b` ([rust/tcl-syntax/src/naming.rs:534–552](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L534-L552), [rust/tcl-syntax/src/naming.rs:1235–1254](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L1235-L1254)). The helper inconsistency was executed; no additional false diagnostic is claimed for the small scalar fixture.

### N04 — P1: imported-command source renames do not preserve re-execution

**Actual returned edits applied and executed.** The model applies a correct observation about live **C Tcl** imports to the wrong operation. A C Tcl runtime `rename` preserves an existing imported command's local name. On re-execution after a source edit, an import either installs the newly exported tail or no longer imports the command; it does not automatically preserve the old live alias. Latest Jim differs even for live source-command rename, so that behavior also needs an explicit family policy.

```tcl
namespace eval Foo {
    proc bar {} {return 1}
    namespace export bar
}
namespace import ::Foo::*
puts [bar]
```

Renaming the declaration to `baz` returns a declaration-only edit. The edited script fails with `invalid command name "bar"`. An exact import with `namespace export *` also receives only the declaration edit in the same-document probe, leaving both import target and call stale. Existing tests explicitly require this incomplete edit policy ([rust/tcl-lsp-core/src/rename.rs:2672–2699](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L2672-L2699), [rust/tcl-lsp-core/src/rename.rs:2869–2898](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L2869-L2898)). Relevant implementation: [rust/tcl-lsp-core/src/rename.rs:1514–1608](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1514-L1608), [rust/tcl-lsp-core/src/rename.rs:1864–1908](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1864-L1908), [rust/tcl-lsp-core/src/references.rs:328–348](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L328-L348).

**Change:** distinguish independent alias spellings from bindings whose spelling derives from the source. Exact/wildcard imports, affected export patterns and derived local uses must participate in one source rename plan; `interp alias` and a runtime rename destination have independent spellings and need a different edit policy. When an affected glob cannot be changed safely, refuse the whole rename. Test by executing the edited source in a fresh interpreter, not by mutating a live command table.

### N05 — P1: rename edits a forced-import call that references correctly excludes

**Actual returned edits change output `A` to `B`.**

```tcl
namespace eval a {namespace export *; proc foo {} {return A}}
namespace eval b {
    proc foo {} {return B}
    namespace import -force ::a::*
    puts [foo]
}
```

Rename the declaration of `b::foo` to `baz`. Find References correctly returns only that declaration. Rename also edits the final `foo` call to `baz`, switching it from imported `a::foo` to local `b::baz`.

The references predicate includes `forced_shadow_takes_the_call` ([rust/tcl-lsp-core/src/references.rs:166–199](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L166-L199)); rename uses only a narrower invocation matcher ([rust/tcl-lsp-core/src/rename.rs:1580–1604](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1580-L1604)). Sharing a helper name is not sharing the full decision.

**Change:** settle each occurrence once with the complete import/export/lifetime context. Reference enumeration and edit planning should project from that answer, with an additional editability policy. Add an invariant test that rename never edits an occurrence resolved to a different binding, including before/after `-force`, `forget`, re-import and cross-file exports.

### N06 — P1: rename collision protection depends on target kind and starting location

**Executed current-source defects.** `rename_proc` checks only the local procedure map; `rename_class` has no equivalent destination gate ([rust/tcl-lsp-core/src/rename.rs:1550–1559](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1550-L1559), [rust/tcl-lsp-core/src/rename.rs:1611–1695](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1611-L1695)). Method rename does not reject an ordinary same-table destination. Declaration-origin server augmentation lacks the destination check present in the consumer-origin workspace path ([rust/tcl-lsp-server/src/lib.rs:16551–16591](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L16551-L16591), [rust/tcl-lsp-core/src/rename.rs:1941–1951](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1941-L1951)).

| Requested rename | Current consequence |
|---|---|
| Class `A` to existing class `B` | Returns edits; fresh execution fails creating `B`. |
| Method `old` to existing method `dest` | Returns edits; `OLD DEST` becomes `DEST DEST`. |
| Proc `P` to class command `C` | Returns edits; class creation fails. |
| Local declaration `p` to sibling-file `q` | Core returns edits and cross-document collector does not reject them; consumer-origin workspace rename does reject the occupied name. |

**Change:** one transaction planner must validate the destination in the actual command/member namespace across all applicable definitions, aliases, objects, builtins and workspace views. Include capture/shadowing at edited uses, not just exact declaration collisions. Use the same planner regardless of cursor origin. Return a typed refusal rather than an empty edit result that permits a different fallback path.

### N07 — P1 for edits: speculative variable navigation is treated as a proven binding

**Executed current-source discrepancy.** For `set g 1; proc p {} {puts $g}`, Tcl requires an explicit `global`, `variable` or `upvar` link. The analyser reports the unbound read, but `lookup_var_in_scope_chain` deliberately walks outward to global `g` ([rust/tcl-lsp-core/src/definition.rs:2076–2163](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/definition.rs#L2076-L2163)). Rename from the invalid proc-local `$g` changes the global declaration only.

This lenience is documented ([docs/design/analysis/name-resolution.md:397–407](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/docs/design/analysis/name-resolution.md#L397-L407)), but its use as an editable identity is inappropriate. **Change:** represent a speculative navigation suggestion separately from a resolved cell; edit-producing providers must require a proven occurrence-to-cell edge. Procedure/method frame boundaries must stop bare variable lookup unless a recorded link authorises the hop. Preserve the real 8.x namespace-frame global fallback, which is a different rule and disappears in Tcl 9.

### N08 — P1: the uplevel inliner resolves against only inlineable procedures

**Executed compiler misrewrite.** `inline_uplevel` calls the shared resolver with an existence oracle containing only passthrough candidates ([rust/tcl-compiler/src/inline_uplevel.rs:403–410](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/inline_uplevel.rs#L403-L410), [rust/tcl-compiler/src/inline_uplevel.rs:498–501](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/inline_uplevel.rs#L498-L501)). A nearer non-inlineable command is treated as nonexistent, allowing a global candidate to win.

```tcl
proc p {} {uplevel 1 {set result GLOBAL}}
namespace eval n {
    proc p {} {uplevel 1 {set result LOCAL}; return}
    proc c {} {set result INIT; p; return $result}
}
puts [n::c]
```

The analyser resolves the call to `::n::p`; the inliner substitutes `::p`'s body. The fresh compilation unit infers `GLOBAL`, and applying the actual optimiser edits changes the output from `LOCAL` to `GLOBAL`. The pass runs before CFG construction ([rust/tcl-compiler/src/compilation_unit.rs:1408–1425](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/compilation_unit.rs#L1408-L1425)), so this contaminates LSP compiler analyses even without a user requesting optimisation.

**Change:** resolve against the full command surface first, then test whether the selected target is inlineable. Include lifetime/mutation/path/import evidence or conservatively decline. Never use an optimisation eligibility subset as an existence oracle. Nested lowering must borrow the full semantic context: `lower_literal_script` currently reconstructs a lowerer with lexer config alone, losing dialect/declared-command inputs ([rust/tcl-compiler/src/inline_uplevel.rs:631–637](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/inline_uplevel.rs#L631-L637), [rust/tcl-compiler/src/lowering/mod.rs:1098–1107](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/lowering/mod.rs#L1098-L1107)).

### N09 — P1/P2: compiler type, call-graph and taint consumers still choose their own names

**Executed current-source disagreement; effect/taint consequences follow the call path.** Constructor inference checks global before current namespace ([rust/tcl-compiler/src/type_infer.rs:214–240](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/type_infer.rs#L214-L240), [rust/tcl-compiler/src/type_infer.rs:927–957](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/type_infer.rs#L927-L957)). With both `::Foo` and `::n::Foo`, `Foo new` inside `::n` resolves to `::n::Foo` in the analyser but produces SSA type `Object(::Foo)`. A nearer non-class command must also block a global class; changing only the order of class-map probes is insufficient.

Interprocedural resolution reconstructs a namespace from the caller name and passes an empty path ([rust/tcl-compiler/src/interprocedural.rs:545–563](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/interprocedural.rs#L545-L563), [rust/tcl-compiler/src/interprocedural.rs:2064–2089](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/interprocedural.rs#L2064-L2089)). The taint solver uses this path-free answer ([rust/tcl-compiler/src/taint_interproc.rs:606–622](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/taint_interproc.rs#L606-L622), [rust/tcl-compiler/src/taint_interproc.rs:685–742](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/taint_interproc.rs#L685-L742)).

```tcl
proc p {} {return GLOBAL}
namespace eval lib {proc p {} {return LIB}}
namespace eval n {
    namespace path ::lib
    proc c {} {return [p]}
}
puts [n::c]
```

C Tcl 8.5+ selects `::lib::p`. The analyser records it, while the current compilation unit's direct-call list contains `::p`. Some rewrite trust gates decline when paths change; that does not repair the wrong definite call-graph fact. A different callee can have different side effects, return types or taint behavior.

**Change:** pass one site-level resolution result to constructor inference, interprocedural summaries, taint and optimisation. Missing path/alias/context must widen to unknown or an explicit possible-target set, not silently select global. Extend conformance to these consumers: the existing compiler command-resolution test primarily checks analyser settlement ([rust/tcl-compiler/tests/command_resolution_conformance.rs:37–99](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/tests/command_resolution_conformance.rs#L37-L99)).

### N10 — P1/P2: class resolution loses invocation and relation-site context

**Executed current-source defects.** Class selection excludes `namespace path` ([rust/tcl-lsp-core/src/definition.rs:4138–4166](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/definition.rs#L4138-L4166)). Thus `C new` inside a namespace whose path contains the namespace defining `C` succeeds in C Tcl 8.6+ but gives no class hover/type hierarchy in the probe.

Superclass/mixin facts retain raw strings, then rebuild their identity relative to the class owner rather than the `oo::define` occurrence ([rust/tcl-compiler/src/analyser/oo.rs:3240–3247](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/oo.rs#L3240-L3247), [rust/tcl-compiler/src/analyser/oo.rs:3292–3296](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/oo.rs#L3292-L3296), [rust/tcl-compiler/src/analyser/class_hierarchy.rs:530–655](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/class_hierarchy.rs#L530-L655)):

```tcl
namespace eval A {oo::class create Base {}}
namespace eval B {oo::class create Base {}; oo::class create Child {}}
namespace eval A {oo::define ::B::Child superclass Base}
```

C Tcl sets `::A::Base` as the superclass. Current MRO contains `::B::Base`. The globally unique-tail fallback also resolves nonexistent `::missing::Base` to unrelated `::unrelated::Base`, although the runtime rejects the explicit missing target. These edges feed inherited-method diagnostics and rename families, so they cannot be treated as authoritative guesses.

**Change:** direct class commands should use ordinary command settlement and then a kind check. Store each superclass/mixin relation's resolved identity or reference occurrence with its definition-time context. Remove qualified-miss tail guessing. Keep command invocation and TclOO relation lookup as distinct, oracle-tested modes over the same state. TclOO cases apply to native C Tcl 8.6+, not automatically to 8.4/8.5 or Jim.

### N11 — P1/P2: package navigation ignores availability and loses provider context

**Availability defect executed through production APIs.** Package scanning already records conditional availability. `select_providers` and `resolve_require` do not take the target environment or filter those facts ([rust/tcl-lsp-core/src/package_resolver.rs:966–1052](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/package_resolver.rs#L966-L1052)). `reachable_files`, used by diagnostic recovery, does filter them ([rust/tcl-lsp-core/src/package_resolver.rs:1180–1198](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/package_resolver.rs#L1180-L1198)). The server's library navigation calls the former ([rust/tcl-lsp-server/src/lib.rs:14137–14152](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L14137-L14152)).

With P 1.0 registered unconditionally and P 2.0 inside `if {0}`, the current API reports P2 `Unavailable` yet selects its file for `resolve_require`; diagnostic reachability selects P1. A version guard such as Tcl 9-only registration has the same missing-context problem.

**Static integration finding:** library merges also lose the requesting package domain. `merge_library_files` returns early when any candidate already exists in the shared workspace index ([rust/tcl-lsp-server/src/lib.rs:14183–14190](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L14183-L14190)). If one application requires exact P1 and another exact P2, both defining `::P::run`, a library loaded for the first can satisfy the second query before its selected provider is considered. Indexed definitions do not identify the requesting interpreter/package selection ([rust/tcl-lsp-core/src/workspace_index.rs:107–151](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L107-L151), [rust/tcl-lsp-core/src/workspace_index.rs:1095–1132](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1095-L1132)). The full multi-request server scenario was not executed; the context loss and early return are directly present.

**Change:** use a `PackageResolutionContext` containing semantic environment, version constraints, preference, search-path generation and execution domain. Filter unavailable registrations before version selection. Navigation, diagnostics and rename must read the same selected provider view. Share physical file analysis, but keep provider/domain membership explicit. A genuine workspace override is different from a library materialised for another application. Conditional alternatives should yield ambiguity/provenance, not whichever file was queried first.

**Additional executed defect:** package `collapse_colons` appends each UTF-8 byte as a Unicode scalar ([rust/tcl-lsp-core/src/package_resolver.rs:652–711](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/package_resolver.rs#L652-L711)). `auto_qualify("café", "::")` returns `cafÃ©`; C Tcl preserves `café`. Reuse the family-appropriate spelling owner and copy unchanged UTF-8 runs. This remains linear-time and removes an independent parser.

### N12 — P1: incremental name settlement can disagree with a fresh index

**Executed with valid before/after Tcl sources.** `SettlementDependencies` stores procedure `(qualified_name, nested)` and class names, but omits declaration-order facts ([rust/tcl-lsp-core/src/workspace_index.rs:1146–1188](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1146-L1188)). Liveness uses each declaration's offset relative to retirements ([rust/tcl-lsp-core/src/workspace_index.rs:2145–2156](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2145-L2156), [rust/tcl-lsp-core/src/workspace_index.rs:3135–3143](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L3135-L3143)). Replacement consequently preserves a stale name cache ([rust/tcl-lsp-core/src/workspace_index.rs:2783–2792](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2783-L2792), [rust/tcl-lsp-core/src/workspace_index.rs:2582–2602](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2582-L2602)).

Start both versions with `proc p {} {return initial}`. In the old text, put another `proc p {} {}` before `rename p q` and padding after; in the new text, put the padding before that same rename and the second declaration after. Keep the rename offset unchanged. Both programs execute successfully: old has no `p`, new has `p`.

After warming the old index, replacing the file reports `workspace_command_exists("::p") == false`; a new index over the same new source reports `true`. The live-procedure iterator already sees `p`, proving disagreement within the same index.

**Change:** compare/cache the semantic live-binding result per slot, or include every relevant ordering fact. Do not invalidate on every body offset change unnecessarily; crossing a lifecycle event is the important distinction. Add incremental-versus-fresh equivalence for procedures/classes, aliases, imports, redefinition and unchanged retirement offsets. This must precede expanding caches for performance.

### N13 — P1/P2: source views do not inherit the full naming environment

**Executed through current analyser/index APIs and C Tcl.** Source rehoming carries a namespace string but not its path/environment ([rust/tcl-lsp-server/src/lib.rs:22295–22314](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L22295-L22314), [rust/tcl-lsp-core/src/workspace_index.rs:3021–3058](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L3021-L3058)).

```tcl
# parent.tcl
namespace eval lib {proc helper {} {return good}}
namespace eval app {namespace path ::lib; source child.tcl}
puts [app::run]

# child.tcl
proc run {} {helper}
```

C Tcl 8.5+ returns `good`. Seeded analysis of the child in `::app` records only `::app::helper`, `::helper`; the combined index returns no call reference to `::lib::helper`. A namespace name alone is not a namespace environment.

There are two additional completeness bounds:

- `imported_constants` has five inner/outer rounds and restarts propagation from empty ([rust/tcl-lsp-core/src/workspace_index.rs:2387–2478](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2387-L2478)). A current-API chain propagates a root directory constant through hops 1–5 and loses it at hops 6–7. The next outer iteration can mistake the same truncated answer for convergence.
- Server rehoming has a four-round cap ([rust/tcl-lsp-server/src/lib.rs:22223–22332](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L22223-L22332)). Static inspection shows a deeper composed source chain can publish a partial view without an explicit incomplete outcome. This server path was not exercised end-to-end.

**Change:** a source view needs `(physical revision, execution domain, namespace identity, semantic environment/binding snapshot)`, including inherited paths and applicable import/package state. Keep one parsed syntax tree and multiple semantic views. Use dependency worklists and SCC/cycle handling; finite acyclic depth should not be silently limited to four/five. If a resource budget is exhausted, record an incomplete result and block edits that require completeness.

### N14 — P2: callback arity and semantic-token roles still collapse namespace identity

**Callback path: static active-production evidence.** `project_command_arities` buckets by tail, and both callback diagnostic paths repeat that projection ([rust/tcl-lsp-db/src/lib.rs:1290–1329](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L1290-L1329), [rust/tcl-lsp-db/src/lib.rs:1550–1588](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L1550-L1588), [rust/tcl-lsp-db/src/lib.rs:1679–1705](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L1679-L1705)). The server uses these queries ([rust/tcl-lsp-server/src/lib.rs:4495–4499](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L4495-L4499), [rust/tcl-lsp-server/src/lib.rs:20636](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L20636)). `::a::cmp {x y}` can supply an accepted two-argument arity for `lsort -command ::b::cmp` even when `::b::cmp` accepts only one. A class/alias with the same tail can disable validation for the explicit procedure. This is not just an unused legacy API.

**Semantic tokens: executed defect.** `VarNameArgRoles` expands each procedure into bare/FQN/unrooted aliases, ignores empty role sets, and looks up a raw head without call-site namespace ([rust/tcl-lsp-core/src/semantic_tokens.rs:595–611](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/semantic_tokens.rs#L595-L611), [rust/tcl-lsp-core/src/semantic_tokens.rs:678–689](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/semantic_tokens.rs#L678-L689), [rust/tcl-lsp-core/src/semantic_tokens.rs:3493–3509](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/semantic_tokens.rs#L3493-L3509)).

```tcl
namespace eval A {proc touch {v} {upvar 1 $v x; set x 1}}
namespace eval B {proc touch {v} {return $v}; touch plain}
```

The `plain` argument is data passed to `B::touch`. The token provider marks it as a variable declaration using `A::touch`'s write-through role. An empty role set is semantic information, not absence of a candidate.

**Change:** key summaries by settled symbol identity and apply them through occurrence IDs. Keep callback-prefix bound arguments as separate facts. Unresolved calls should remain unresolved; pooling unrelated tail signatures is not conservative validation. Retain Salsa's per-symbol cutoff benefits with an identity/context key.

### N15 — P1: compact minification has its own unsafe naming transformation

**Actual current-source outputs executed.** The compact-name allocator begins with an empty used-name set and skips existing one-character procedures without reserving them ([rust/tcl-lsp-core/src/minify.rs:1293–1318](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/minify.rs#L1293-L1318)):

```tcl
proc a {} {return OLD}
proc longName {} {return NEW}
puts [a]
puts [longName]
```

Isolated compaction generates two `proc a` definitions and changes `OLD NEW` to `NEW NEW`. Isolation does not make this internal collision valid.

The emitter separately requires `call_text == proc_def.name` ([rust/tcl-lsp-core/src/minify.rs:1227–1245](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/minify.rs#L1227-L1245), [rust/tcl-lsp-core/src/minify.rs:1326–1333](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/minify.rs#L1326-L1333)). `proc longName ...; puts [::longName]` becomes `proc a ...; puts [::longName]`, which fails. Qualified declarations can exhibit the inverse omission. The symbol map also uses bare names, losing homonyms.

**Change:** use the shared transactional rename planner, reserve all occupied command slots, and preserve each occurrence's qualification/quoting. Refuse a whole symbol transformation if a required occurrence cannot be rewritten. Reuse the existing semantic snapshot instead of making a separately configured analyser.

### N16 — P2: remaining LSP fallbacks contradict the common naming source

Each of these was exercised against the freshly built libraries:

| Consumer | Mismatch | Code and suggested change |
|---|---|---|
| Signature help | Offers `namespace path` under Tcl 8.4; exact spelling lookup also misses the valid 8.5+ `namespace pat` abbreviation. | [rust/tcl-lsp-core/src/signature_help.rs:187–216](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/signature_help.rs#L187-L216) uses dialect-agnostic `registry.get` and direct subcommand equality. Use the resolved surface/subcommand query. `pa` is ambiguous between `parent`/`path` in 8.5+; it is not a valid test for `path`. |
| Alias hover | A global alias `a` is shown for a call to a local `::n::a` procedure. | [rust/tcl-lsp-core/src/hover.rs:964–969](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/hover.rs#L964-L969), [rust/tcl-lsp-core/src/hover.rs:2673–2689](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/hover.rs#L2673-L2689). Remove the early spelling scan; display alias provenance from the actual target. |
| Type definition | Plain data `greet` in `puts greet` inside a class is treated as its method and targets the class. | [rust/tcl-lsp-core/src/type_definition.rs:68–76](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/type_definition.rs#L68-L76). Use declaration/dispatch roles, not membership of a word in the method table. A bare TclOO command is also not implicitly `my`. |
| Completion | With workspace `::A::f` and `::B::f`, only the first bare-label `f` survives, despite different qualified insertion texts. | [rust/tcl-lsp-core/src/completion.rs:780–804](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/completion.rs#L780-L804). Deduplicate by identity/insertion spelling; rank visible names and show namespace details. |

These are smaller than the destructive findings, but collectively disprove the claim that every provider already consumes one naming answer.

### N17 — P1: a second variable grammar corrupts SpecTcl registration and live hook results

**Executed current-source defect in required SpecTcl functionality, including an unsafe optimiser result.** `codegen::is_bare_var_name` accepts every colon, including a lone one ([rust/tcl-compiler/src/codegen/values.rs:41–46](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/codegen/values.rs#L41-L46)). `push_array_key` treats that as proof of a whole variable reference ([rust/tcl-compiler/src/codegen/values.rs:412–414](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/codegen/values.rs#L412-L414)); native lowering shares the predicate ([rust/tcl-compiler/src/native_lowering/cells.rs:116–124](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/native_lowering/cells.rs#L116-L124)).

For `$a:b`, C Tcl substitutes `a` and appends literal `:b`. The current public codegen helper emits a single scalar load of variable `a:b`. A runtime fixture distinguishes them with `a=x`, `a:b=BAD`, and array entries `arr(x:b)=good`, `arr(BAD)=bad`. The runtime result is `good`; the emitted operand identifies the wrong cell.

The original audit classified this as adjacent backend debt. The explicit requirement for functional SpecTcl prompted a production-path probe, which shows that it **cannot be deferred from this release**:

```tcl
speclib edge 2.0 {
    set a x
    set a:b BAD
    set arr(x:b) good
    set arr(BAD) bad
    command $arr($a:b) {arity 0}
}
```

Current `evaluate_pack_with` registers **`bad` with no notices**, both with `static_fast_path=true` and with it disabled. Moving the substitution into a helper `proc` also registers `bad`; using `$arr(${a}:b)` is a control that correctly registers `good`. A minimal capture driver implementing `speclib`/`command` around the body produces **`good` on all five C Tcl references and latest Jim**. Direct and helper forms under both `speclib 1` and `speclib 2.0` give 24 matching oracle cases. This driver checks Tcl evaluation of the template; it is not a claim that the external interpreters implement the repository's SpecTcl DSL.

The same bug affects live analysis hooks, not just loading:

```tcl
speclib edge 2.0 {
    command edge::test {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set a x
            set a:b BAD
            set arr(x:b) good
            set arr(BAD) bad
            fold $arr($a:b)
        }
    }
}
```

Load the pack, build/install its production hook plan and TclVM host, install the `tcl8.6` registry, then optimise `proc ::f {} {set y [edge::test abcde]}`. The current API emits **O129 replacement `bad`**, although the hook's Tcl substitution computes `good`. The hook installs without a decline and the pack has no notices. This was reproduced for both DSL `1` and `2.0` through `pack::load` → `hooks::plan_for` → `tclvm_host` → `registry_for_dialect_with_packs` → `optimise_raw`; it is a direct production-API probe, not an executed JSON-RPC test. The server's compiler/optimisation paths described in release section A consume that machinery.

**Change:** correct whole-reference recognition and require the parsed extent to cover the complete reference. Reusing the dialect-aware owner is the preferred implementation; a correct, tested compatibility fix is sufficient for this release and may leave the surrounding VM/runtime design unmigrated. Keep braced decoding separate. A documented looser helper is not appropriate at a call site requiring exact lexical identity. Retain the template and live-hook regressions above, including lone colons, separator runs and dialect-sensitive boundaries. Disabling ordinary supported SpecTcl evaluation is not an acceptable substitute for fixing its behavior.

## 3. Migration completeness and API suitability

### 3.1 What is migrated, and what is still independent

| Area | Existing shared foundation | Remaining migration boundary |
|---|---|---|
| Written C Tcl names | Shared colon scanner, command versus namespace canonicalisers, candidate ordering. | Structured static identity is not used; Jim command policy absent; display strings parsed back into identities. |
| Variable grammar | Shared byte scanner, Tcl 8/9 closer, Jim lexical axes. | Cursor helpers, indirect-array predicate, normaliser and codegen exact-reference test use competing rules. |
| Procedure navigation | Most user-procedure providers call `resolve_called_proc` and use analyser facts. | Forced-shadow policy differs between references/rename; alias hover and minifier bypass full settlement. |
| Classes/methods | Shared hierarchy/MRO implementation, dispatch metadata and rename safety facts. | Wrong relation-site context, unique-tail guesses, path-free direct class selection and independent SSA constructor lookup. |
| Workspace references | Per-document ownership, revision checks, lazy reverse indexes and exact dependency comparisons. | Dependency projection is incomplete; some occurrence checks scan the entire workspace; provider/execution-domain identity is absent. |
| Source/packages | Seeded reanalysis, source graph, guarded package registration, version constraints. | Seeds omit inherited environment; bounded propagation appears complete; navigation ignores availability and merged libraries lose provider context. |
| Compiler analyses | Common spelling helpers, registry transition descriptors, command-binding trust, shared compilation units. | Inliner subset oracle; path-free call graph/taint; class typing and nested lowering context differ. |
| Presentation summaries | Shared registry, semantic-token realm bindings, Salsa query graph. | Callback tail buckets, namespace-blind role aliases, dialect-agnostic signature fallback and completion tail deduplication. |

These are seam failures: the individual models are often sophisticated, but their joins discard exactly the context needed to choose the right identity.

### 3.2 APIs currently allow contradictory inputs

`CallResolution` permits both registry and program export oracle to be absent ([rust/tcl-lsp-core/src/definition.rs:143–211](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/definition.rs#L143-L211)). `ProgramExports` usefully pairs a URI with its oracle, but there is no equivalent enforced pairing for source revision, analysed profile, registry/pack generation, package selection, source view and invocation facts. Providers accept these as independent parameters, and an `Option` miss often permits another spelling-based fallback.

Other concrete context seams:

- `AnalysisResult::ns_var_global_fallback` re-finds a catalogue profile from a string ([rust/tcl-compiler/src/analyser/types.rs:2597–2606](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/types.rs#L2597-L2606)) rather than reading the already-resolved environment. Jim can be a model-only environment outside that catalogue. The observed Jim no-global-fallback answer happens to agree here; the construction still cannot express all model-only profiles or overlays reliably.
- `WorkspaceIndex::registry_for` reconstructs a static registry from a dialect name ([rust/tcl-lsp-core/src/workspace_index.rs:2689–2693](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2689-L2693)). A document's custom pack/stub surface is richer than that string. This is a verified missing context input; no specific custom-pack false result is asserted without an integration fixture.
- `analyse_with_workspace_classes` constructs another analyser, with a cache keyed by text/profile/class facts rather than the complete main analysis context ([rust/tcl-lsp-server/src/lib.rs:15027–15092](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L15027-L15092)). It can lose registry generation, source seed and file/stub inputs used elsewhere. Treat this as an integration risk requiring a parity test, not as a separately reproduced failure.
- Nested passthrough lowering loses semantic inputs. A current-source iRules probe gives `Binary(Contains)` for a direct `$x contains "a"` condition and `Raw` for the same expression through the passthrough inliner. This verifies the context-loss consequence noted in N08; ordinary Tcl oracles cannot adjudicate an F5-specific operator.
- The safe rename-name validator intentionally accepts only ASCII identifiers ([rust/tcl-lsp-core/src/rename.rs:100–119](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L100-L119)). This is a conservative product limitation, not Tcl's legal-name grammar. It should remain safe until a spelling-aware writer handles punctuation, Unicode, empty names and quoting.

### 3.3 Remaining debt that should not be overstated

`insert_rebound_candidates` treats any `::` as absolute ([rust/tcl-compiler/src/command_binding.rs:4730–4735](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/command_binding.rs#L4730-L4735)). That helper is wrong for relative-qualified `inner::p` under `::n`. However, both tested complete mutation summaries correctly distrusted `::n::inner::p` because a richer binding projection repairs the result elsewhere. This is **migration debt and redundant conflicting logic**, not a demonstrated additional miscompilation. Remove/replace it through the shared operation-specific resolver rather than changing one string test and leaving rename-source/destination semantics conflated.

`place_bridge` explicitly says consumers are not yet wired ([rust/tcl-compiler/src/place_bridge.rs:20–30](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/place_bridge.rs#L20-L30)). Its existence and its own tests do not establish an authoritative shared variable place model in LSP/SSA. `var_resolve` needs frame and dialect semantics before being promoted to that role ([rust/tcl-compiler/src/var_resolve.rs:115–140](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/var_resolve.rs#L115-L140)). Distinct navigation, analysis and runtime storage types are reasonable; duplicating their semantic decisions is the problem.

`optimiser::method_barrier` still has global-first and first-tail class lookup ([rust/tcl-compiler/src/optimiser/method_barrier.rs:574–595](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/optimiser/method_barrier.rs#L574-L595)). It needs the same audit as N09/N10 before being trusted as a conservative barrier. Selecting one arbitrary matching tail is not the same as a may-target union. No separate bad rewrite from this path was demonstrated here.

### 3.4 What “one naming source” should mean

Use a single immutable **resolution snapshot** per semantic revision. It should combine:

1. Physical source revision and parsed syntax/occurrence IDs.
2. Resolved language environment: family, release, lexical grammar, builtin surface, registry/pack generation and document declarations.
3. Execution domain and source view: interpreter/entry point, package-provider selection, namespace state and inherited path/import environment.
4. Command/cell identities, binding lifetimes, declaration provenance and dependency edges.
5. Per-occurrence resolution outcomes and spelling/editability facts.

An illustrative interface shape is:

```rust
struct SemanticViewId { /* document revision + domain + environment generation */ }
struct OccurrenceId { /* source identity, span and syntactic role */ }
enum Resolution {
    Resolved { target: SymbolId, binding: BindingId, evidence: EvidenceId },
    Ambiguous { candidates: CandidateSetId },
    Dynamic,
    UnavailableForEnvironment,
    Missing,
    Incomplete,
}
// Resolution is queried by occurrence/view, not reconstructed from displayed text.
fn resolve(view: SemanticViewId, occurrence: OccurrenceId) -> Resolution;
```

This is a design suggestion, not a patch. The important properties are stronger than the exact names:

- Separate **slot** from **entity**: C Tcl rename moves a command object between slots; imports may follow that object, while a source-refactoring dependency may derive a local spelling from it. Jim may have different link semantics. A single `qualified_name: String` cannot encode all three.
- Distinguish written names from constructed identity and UI display. For C Tcl, interned parent-linked namespaces plus simple-name IDs remove ambiguity and repeated parsing. For Jim, preserve its command-key equivalence. The interface can be shared without forcing identical internal equality.
- Keep variable cells, frame-local bindings, array elements and explicit aliases typed separately. An array element's identity includes its base cell and index form; scalar punctuation must not accidentally manufacture an element.
- Return uncertainty explicitly. A definition suggestion, a possible target, and a proven editable occurrence must not share an indistinguishable `Option<String>` path.
- Treat source/package views as members of a domain. One shared store of syntax and facts is compatible with multiple application entry points and package versions; one global namespace union is not always a runtime world.
- Store provenance sufficient to explain a target and to invalidate it. Provider caches should read snapshot IDs, not independently reconstruct a dialect from a string.

### 3.5 Rename should be a transaction over those facts

Select the symbol(s) from a proven occurrence; enumerate binding and spelling dependencies; compute destination slots; check all collisions/capture; render each occurrence using Tcl-aware quoting and qualification; then validate that the affected references still resolve to their intended entities. Apply all edits against the source versions used to plan them, or refuse/recompute. Namespace-seeded declarations can deliberately denote multiple entities, but every view must approve the same physical edit.

The same planner should serve LSP rename, compact minification and other name-producing refactors. It should preserve independent aliases, update derived import spellings, and refuse when dynamic/incomplete evidence prevents complete edits. This consolidates safety while allowing read-only providers to display useful ambiguity.

## 4. Algorithmic performance and memory

### 4.1 Cost model

Let `F` be files, `P` procedures, `C` classes, `I` invocation records, `U` unresolved calls, `S` scope nodes, `K` candidates per call, `ℓ` mean candidate bytes, `L` links, `T` retirement records in a contributing file, `M` import/export/lifecycle records, `D` source depth, `E` source/package graph edges, `A` alias-chain length, `R_v` references to variable `v`, and `e_d` rename edits in document `d`. Hash-table bounds are expected bounds; hashing/comparing an unbounded string still costs its byte length. Outputting `r` references necessarily costs at least `Ω(r)`.

The most significant avoidable work is repeated whole-definition/scope scans and duplicated graph/name state, not the linear cost of returning real results.

| Operation | Current worst-case/important scaling | Code | Better alternative and trade-off |
|---|---|---|---|
| Candidate construction | `O(K²ℓ)` from `Vec::contains`; `O(Kℓ)` owned result. Even a first local hit builds the complete list. | [rust/tcl-syntax/src/naming.rs:1114–1201](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L1114-L1201) | Pre-normalise/deduplicate path IDs per environment and iterate lazily. If materialising, ordered vector + membership set gives expected linear work while preserving first-match order. Tiny-path fast path avoids unnecessary hash allocation. |
| Invocation settlement | Materialises candidates twice per ordinary relative call. | [rust/tcl-compiler/src/analyser/scope.rs:1276–1283](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L1276-L1283) | Resolve the already-retained candidates, or return winner and witness from one pass. Low-risk allocation improvement. |
| Deep key parsing | `key_tail` on valid deep empty-tail keys and `key_segments` can be `O(D·name_bytes)`, quadratic for fixed-size segments; repeated prefix allocations. | [rust/tcl-syntax/src/naming.rs:219–363](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-syntax/src/naming.rs#L219-L363) | Typed slots make tail constant-time and parent traversal `O(D)`; emit strings only when requested. |
| Invocation body/scope recovery | `O(I(P+C))` body scans and `O(IS)` child-scope visits in wide files. Tree depth alone is not the bound. | [rust/tcl-compiler/src/analyser/scope.rs:1032–1096](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L1032-L1096), [rust/tcl-compiler/src/analyser/scope.rs:174–185](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L174-L185), [rust/tcl-compiler/src/analyser/types.rs:2620–2627](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/types.rs#L2620-L2627) | Record scope/body IDs when walking. For offset-only input, reuse one interval sweep/index, accounting for overlapping synthetic scopes. |
| Earliest reachable call offsets | Repeated complete edge passes; reverse-ordered chains need `Θ(P)` rounds, `Θ(P·I)` edge visits. | [rust/tcl-compiler/src/analyser/scope.rs:1032–1081](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L1032-L1081) | Adjacency/worklist; priority propagation of decreasing earliest offsets can use `O((P+I) log P)`-style work, preserving top-level base facts. |
| Procedure nesting at index intake | `Θ(P(P+C))` for top-level declarations. | [rust/tcl-lsp-core/src/workspace_index.rs:1389–1399](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1389-L1399) | Add declaration offsets to the existing body sweep at [rust/tcl-lsp-core/src/workspace_index.rs:2175–2206](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2175-L2206). |
| Qualified variable reference attachment | `Θ(Σ R_v²)` from repeated `Vec::contains`, then sorting. | [rust/tcl-compiler/src/analyser/scope.rs:393–409](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L393-L409), [rust/tcl-compiler/src/analyser/scope.rs:1809–1822](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L1809-L1822) | Append/sort/deduplicate once, `O(R_v log R_v)`; linear merge if ordering is guaranteed, or hash set with extra memory. |
| Workspace existence per candidate | `O((P+C)(T+1)+L)` scans, instead of a slot lookup. | [rust/tcl-lsp-core/src/workspace_index.rs:2145–2156](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2145-L2156), [rust/tcl-lsp-core/src/workspace_index.rs:4794–4809](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L4794-L4809) | Per-slot live-binding summary including nested/unconditional status; lifecycle events indexed by slot/offset. Preserve builtin-shadow rules. |
| Cross-file diagnostic settlement | Approximately `O(U·K·((P+C)(T+1)+L))`, plus arity rescans and import decisions. | [rust/tcl-lsp-server/src/lib.rs:29143–29318](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L29143-L29318) | Resolve once using indexed binding/arity summaries. Cache by complete semantic generation, not bare tail. |
| Local procedure code lenses | Procedure × invocation × namespace-scope scan: worst-case `O(P·I·S)`, cubic when all grow together. | [rust/tcl-lsp-core/src/code_lens.rs:152–165](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/code_lens.rs#L152-L165), [rust/tcl-lsp-core/src/references.rs:166–199](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L166-L199), [rust/tcl-lsp-core/src/references.rs:423–435](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L423-L435), [rust/tcl-compiler/src/analyser/scope.rs:174–185](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/analyser/scope.rs#L174-L185) | Settle occurrences once and retain reverse counts/locations. The namespace lookup currently occurs before rejecting mismatched procedure names. Avoid allocating reference vectors just to count them. |
| Method code lenses | Repeated document/body scans, at least `O(methods × source_bytes)` before hierarchy work. | [rust/tcl-lsp-core/src/code_lens.rs:290–325](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/code_lens.rs#L290-L325), [rust/tcl-lsp-core/src/references.rs:1520–1545](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L1520-L1545), [rust/tcl-lsp-core/src/references.rs:2041–2065](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/references.rs#L2041-L2065) | Shared member-occurrence and realm-binding indexes; preserve uncertainty and inheritance semantics. |
| Rename edit merge | `O(Σ e_d²)` duplicate checks, repeated document acquisition per intent. | [rust/tcl-lsp-server/src/lib.rs:16695–16727](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L16695-L16727), [rust/tcl-lsp-server/src/lib.rs:27115–27143](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L27115-L27143) | Group by URI, acquire one versioned snapshot, hash or sort/merge ranges. Reject conflicting text/overlap rather than silently retaining the first edit. |
| Alias chase | `O(Aℓ)` plus new visited set/cloned strings per chase. | [rust/tcl-lsp-core/src/workspace_index.rs:4663–4676](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L4663-L4676) | SCC/cycle results and terminal IDs memoised per semantic generation; do not compress across lifetime/domain boundaries. |
| Source order graph | `O(E+F·D)` construction and `Θ(F·D)` stored path entries; chain yields quadratic time and memory; comparisons scan common prefixes. | [rust/tcl-lsp-core/src/source_graph.rs:613–657](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/source_graph.rs#L613-L657), [rust/tcl-lsp-core/src/source_graph.rs:861–893](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/source_graph.rs#L861-L893) | Parent forest/root/depth in linear space; LCA/binary lifting gives `O(F log F)` space/build and `O(log F)` comparisons. More complex but avoids full root-path copies; preserve ambiguity and package ordering. |
| Body-only reference repair | Reuses name caches, but rebuilds workspace-wide wildcard import groupings on the next dirty-mode query. | [rust/tcl-lsp-core/src/workspace_index.rs:4544–4547](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L4544-L4547), [rust/tcl-lsp-core/src/workspace_index.rs:5225–5363](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L5225-L5363) | Immutable import index keyed by resolution-input generation, using stable record IDs rather than self-borrows. Additional retained memory buys removal of repeated `O(M)` work. |

Additional costs worth addressing after the main paths:

- `override_family` has repeated fixed-point and linear family/MRO membership tests; a conservative bound is `O(H³Q)` for `H` candidate definers and MRO length `Q` ([rust/tcl-lsp-core/src/rename.rs:1336–1388](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename.rs#L1336-L1388)). Compute connected family components using adjacency/visited sets and cache by hierarchy generation.
- `ambiguous_object_command` scans bindings inside a binding scan, `O(B²)` ([rust/tcl-lsp-core/src/rename_safety.rs:403–445](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/rename_safety.rs#L403-L445)). Build `qualified slot → class identity set` once.
- Package directory deduplication uses vector membership ([rust/tcl-lsp-core/src/package_resolver.rs:730](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/package_resolver.rs#L730), [rust/tcl-lsp-core/src/package_resolver.rs:881–884](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/package_resolver.rs#L881-L884)); ordered vector plus set avoids quadratic duplicate checks. Source-file unions have the same smaller-scale pattern.
- `ModuleCommandMutations` hashing constructs and sorts a cloned snapshot, `O(m log m + name_bytes)` with allocations ([rust/tcl-compiler/src/command_binding.rs:4266–4268](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/command_binding.rs#L4266-L4268), [rust/tcl-compiler/src/command_binding.rs:4393–4414](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/command_binding.rs#L4393-L4414)). Intern an immutable canonical snapshot or cache its digest while retaining collision-safe equality.

### 4.2 Measurements against this snapshot

These are diagnostic measurements, **not release-server latency claims**. Fresh libraries were built in the unoptimised test profile using Rust 1.98.1. Other review work ran concurrently; process-CPU medians are used for the production-library measurements below. Analysis is excluded where indicated. No result from the repository's stale binary is attributed to this hash.

| Synthetic size | Index flat procedure metadata, CPU ms | 100 missing call-aware workspace-existence queries, CPU ms | Build source-order chain of that many files, CPU ms |
|---:|---:|---:|---:|
| 250 | 5.718 | 11.803 | 45.118 |
| 500 | 10.655 | 21.921 | 180.366 |
| 1,000 | 38.346 | 44.956 | 690.050 |
| 2,000 | 147.930 | 89.862 | 2,877.410 |

Flat indexing and source-chain construction show the quadratic shapes predicted by the code. Each group of 100 existence misses scans linearly with workspace size; invoking that operation for every call multiplies those dimensions.

For reproducibility, the indexing fixture constructs `P` top-level `ProcDef` records with distinct `::p00000000`-style names and non-overlapping body spans in an otherwise empty analysis, then calls `WorkspaceIndex::from_documents`; its lookup phase performs 100 `workspace_command_exists_for_call("::missing", false)` calls. The source-order fixture is a literal-source chain of `F−1` edges, with unique file URIs and no package alternatives. Three runs per size produced the medians above.

Local code-lens collection over pre-analysed flat files measured CPU medians of **263.545 ms at 125 procedures**, **918.139 ms at 250**, and **12,528.994 ms at 500**. Its source repeats `proc p00000000 {} {}\np00000000\n` with unique names, producing two invocation records per procedure; each pre-analysed input was queried three times. The larger run was stopped and no result claimed. These noisy debug-profile values are evidence of a serious scaling problem, not a precise exponent fit; the nested loops establish the worst-case bound.

An optimised standalone extraction of the current naming helpers measured:

| Workload | 400 levels/entries | 800 | 1,600 |
|---|---:|---:|---:|
| `key_tail` on deep empty-tail key, ms | 0.664 | 2.788 | 10.743 |
| Cumulative allocation bytes for that operation | 240,200 | 960,400 | 3,840,800 |
| `key_segments`, ms | 0.985 | 3.091 | 11.261 |
| Cumulative allocation bytes | 504,483 | 1,969,059 | 7,778,211 |
| Candidate-list construction, ms | 0.526 | 1.469 | 5.876 |
| Resolver with immediate first-candidate hit, ms | 0.459 | 1.380 | 5.435 |

The last row shows that successful early lookup still pays for complete eager construction. Cumulative allocated bytes are allocator traffic, not peak live memory or RSS. These depths/path lengths are stress inputs; benchmark ordinary depths as well before choosing small-vector thresholds.

For the source-order chain, the strongest memory evidence is the exact path-element count, independent of noisy timing:

| Files | Stored path elements `F(F−1)/2` | Populated path bytes | Path-vector capacity bytes |
|---:|---:|---:|---:|
| 500 | 124,750 | 2,495,000 | 3,362,240 |
| 1,000 | 499,500 | 9,990,000 | 13,469,120 |
| 2,000 | 1,999,000 | 39,980,000 | 53,917,120 |
| 4,000 | 7,998,000 | 159,960,000 | 215,750,080 |

`Placed` is 20 bytes on the measured 64-bit layout. This isolated extraction used standard-library hashers in supporting scaffolding instead of the production Fx aliases; the path counts/layout remain the relevant evidence. URI maps, allocator overhead, package data and all other analysis state are additional. This is not a claim that a typical workspace has a 4,000-deep chain.

### 4.3 Retained memory and ownership

1. **Repeated occurrence context:** `SignatureCommandInvocation` owns `Vec<String>` candidates ([rust/tcl-compiler/src/signature_scan/types.rs:519–525](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/signature_scan/types.rs#L519-L525)), cloned into workspace rows ([rust/tcl-lsp-core/src/workspace_index.rs:1420–1434](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1420-L1434)). Rows also own URI/written name strings ([rust/tcl-lsp-core/src/workspace_index.rs:462–477](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L462-L477)). Candidate storage is `O(IKℓ)` in each retained copy even when many calls share one namespace/path context. Use document IDs, namespace-context IDs and interned name IDs.
2. **Repeated settled names:** per-document `(String, occurrence)` contributions and reverse-map target keys duplicate names; direct and link-following caches retain separate projections ([rust/tcl-lsp-core/src/workspace_index.rs:1911–1953](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1911-L1953), [rust/tcl-lsp-core/src/workspace_index.rs:4559–4571](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L4559-L4571)). Preserve the distinct semantics but share IDs/storage.
3. **Multiple derived name sets:** command names, defined-name sets, link maps and settlement indexes retain overlapping strings ([rust/tcl-lsp-core/src/workspace_index.rs:1790–1845](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1790-L1845)). One authoritative arena can back these useful projections without independent string identity.
4. **Transient deep clones:** document replacement clones old/new settlement surfaces ([rust/tcl-lsp-core/src/workspace_index.rs:1177–1210](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1177-L1210), [rust/tcl-lsp-core/src/workspace_index.rs:2783–2791](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L2783-L2791)); server rename clones an entire `AnalysisResult` before a worker ([rust/tcl-lsp-server/src/lib.rs:15641](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-server/src/lib.rs#L15641)); optimiser pass construction clones interprocedural analysis ([rust/tcl-compiler/src/optimiser/manager.rs:135](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-compiler/src/optimiser/manager.rs#L135)). Prefer immutable `Arc` facts/snapshots where ownership permits. Do not weaken equality/invalidation to save copying.
5. **Peak-capacity retention:** cleared document records intentionally retain vector capacity ([rust/tcl-lsp-core/src/workspace_index.rs:1213–1220](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-core/src/workspace_index.rs#L1213-L1220)). Memory follows peak slot/table capacity, not just current records. A byte budget or shrink-on-permanent-eviction policy can reclaim exceptionally large one-off files while preserving fast ordinary replacement.
6. **Source-view multiplication:** multiple namespace/environment seeds can retain/reanalyse multiple semantic views of the same syntax. Account for view count, not only file count. Share syntax and immutable environment data; do not merge legitimately different identities to save memory.
7. **Salsa is not simply an unbounded leak:** deep-memo LRU caps, uncached closed-file analysis and interned-key garbage collection are explicitly implemented and documented ([rust/tcl-lsp-db/src/lib.rs:76–181](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/tcl-lsp-db/src/lib.rs#L76-L181)). Preserve low-durability/tracked-query GC invariants. Count limits can still retain very different byte totals for differently sized files, and light workspace tiers intentionally remain live. Measure bytes, retained generations and open/close/edit cycles separately.

For scale, one million occurrence records each owning an 80-byte URI contain 80 MB of repeated URI payload plus 24 MB of `String` headers on a 64-bit target, before allocator overhead. This is a transparent size calculation, not measured process memory. Interning fixes repetition only if arenas have a bounded lifecycle; replacing every string with a freshly cloned structured path could increase memory despite fixing correctness.

## 5. Conformance and regression protection

### 5.1 Current gates are useful but insufficient

The command vectors and per-release C oracle are strong protections for the rule they exercise. They do not establish that every consumer supplies the same namespace, dialect, existence oracle or lifetime. In particular:

- Definitions/winners represented only as flat FQN strings cannot distinguish N01's colliding C Tcl slots. Make definitions return unique markers and encode holder segments/simple names independently from display.
- The compiler conformance runner principally checks analyser invocation settlement, not inliner, SSA typing, interprocedural or taint results. Add independent projections from the same fixtures.
- `resolution-drift` searches `.name ==` near `all_procs`/`all_classes` within a 240-byte lexical window and exempts sanctioned files ([rust/xtask/src/resolution_drift.rs:36–54](https://github.com/bitwisecook/tcl-lsp/blob/fcb7927f02b5395753247b322fdd7be87ae5c731/rust/xtask/src/resolution_drift.rs#L36-L54)). It cannot catch a wrong existence subset, missing context, tail bucket, raw registry lookup, display collision or duplicated scanner. Passing it is not proof of ownership.
- Several tests assert incomplete source-renaming behavior. Tests must compare both expected identities and edited-program behavior; a green test can preserve the wrong contract.
- A C Tcl version matrix does not cover Jim. Family-specific expected results must be explicit; do not count Jim disagreements with C Tcl as Jim failures.

### 5.2 Recommended acceptance matrix

| Axis | Minimum regression coverage |
|---|---|
| Environment | Every C Tcl 8.4/8.5/8.6/9.0/9.1 profile, latest pinned Jim, vendor embedded-core rules, custom packs/stubs and environment changes. |
| Command spelling | Absolute/relative, no implicit ancestor lookup, lone/interior/edge colons, separator runs, empty simple names, Unicode, escaped/braced/quoted words, embedded NUL where the representation supports it. |
| Variables | Bare versus braced Unicode, literal suffixes, cursor boundaries, Tcl 9 nested braces, legal scalar parentheses, empty array base/index, qualified/relative cells, `global`/`variable`/`upvar`/`namespace upvar`, Jim expression/index forms. |
| Binding lifetime | `rename` source and destination, deletion, recreation, aliases, exact/wildcard imports, `-force`, export changes, `forget`, calls before/after events and deferred proc bodies. |
| Namespace/source | Path order, relative path roots, paths unavailable in 8.4/Jim, source-site inherited path/import state, depth beyond five, one file under multiple seeds, cycles and ambiguous roots. |
| Packages | Exact/ranged version requirements, unavailable/conditional providers, stable/latest preference, separate entry points requiring different releases, autoload Unicode, query-order independence, configuration rebuilds. |
| OO | Direct class calls versus relation operands, reopening from another namespace, local nonclass shadowing, missing qualified classes, methods/objects/override families, inferred versus unknown receivers. |
| Provider parity | Definition, hover, references, rename/prepare, hierarchy, signature/inlay, semantic roles, callback arity, code lenses, diagnostics, compiler call/taint/type facts and minification agree on identity where applicable. |
| Incrementality | Cold versus warm, body-only edit, declaration moved across fixed-offset retirement, provider query-order permutations, source/provider environment change without source-text change, multi-view invalidation. |
| Scaling | Flat procs, many reads of one qualified variable, wide namespace path, deep source chain, long alias chains, import-heavy body edit, large rename and repeated open/close sessions. |

Do not require every dynamic Tcl construct to become statically decidable. Computed names, custom C resolvers, unknown interpreter targets and unprovable code generation legitimately require uncertainty. The requirement is that uncertainty be shared and never silently converted into a confident unrelated target or an incomplete edit.

### 5.3 Proposed implementation order for the staged release

1. **Contain incorrect transformations.** Fix N08's subset oracle; refuse collision cases and unproved proc-variable bindings; correct forced-import selection; make imported source rename/minification complete or refuse. These can land with focused behavior regressions before the full identity migration.
2. **Establish semantic identity and environment ownership in the release boundary.** Family-aware written-name parsing/equality; typed identity/display/spelling; operation-specific policies; occurrence facts tied to a resolved environment and domain. Include latest Jim's differing rules from the start. Existing owners and safe compatibility adapters are acceptable; section 3.4 is not a mandatory wholesale rewrite.
3. **Migrate LSP consumers and source/package views; preserve functional packs.** Replace tail/spelling fallbacks, carry inherited source state, use one provider selection and one relation-site identity. Include compiler facts used by LSP diagnostics/edits. Correct SpecTcl behavior where needed through targeted fixes; its VM/runtime implementation need not migrate. Preserve explicit unknown outcomes for incomplete analyses without disabling supported EDA/Jim/other pack behavior.
4. **Repair incremental dependencies.** Cold/warm equality and domain/provider/source-state invalidation are release criteria. Cache correctness must precede more caching.
5. **Meet measured shipping-build performance and memory criteria.** Scope/body IDs and reverse occurrence indexes are the first optimisation candidates; indexed live bindings/arities follow. Source parent/LCA representation, shared import snapshots, interning and clone reductions can follow the release where measurements justify deferral. Preserve bounded SpecTcl load/hook costs and generation retirement.
6. **Complete the integrated release checklist and update contracts.** Remove completed-migration claims until the relevant consumers are wired. Replace misleading source-refactoring expectations. Keep narrow textual drift guards, but add behavioral cross-consumer/family checks and deterministic scaling budgets. Record the exact shipping hash and explicit residual scope, then return to the remaining compiler/VM/runtime migration.

## 6. Verification record and limits

### 6.1 Original naming review

The focused naming run passed **81 tests**:

| Target | Result | Scope |
|---|---:|---|
| `tcl-compiler --test command_resolution_conformance` | 1 passed | Existing analyser-settlement vectors. |
| `tcl-lsp-core --test name_resolution` | 75 passed | Existing LSP/analysis name-resolution fixtures. |
| `tcl-syntax --test command_resolution_conformance` | 3 passed | Canonical helper, release tagging, and real C Tcl vector execution across all five configured releases. |
| `tcl-syntax --test variable_resolution_conformance` | 2 passed | Release tagging and real C Tcl variable vectors across all five configured releases. |

The command, with the installed toolchain/cache on `PATH` and each `TCL_LSP_TCLSH84`, `85`, `86`, `90`, `91` variable pointing to its verified wrapper, was:

```sh
CARGO_TARGET_DIR=/tmp/naming-review-target cargo test --locked --offline \
  --ignore-rust-version \
  -p tcl-syntax --test command_resolution_conformance --test variable_resolution_conformance \
  -p tcl-compiler --test command_resolution_conformance \
  -p tcl-lsp-core --test name_resolution
```

The wrappers set the matching `TCL_LIBRARY` per interpreter. C Tcl 8.4/8.5 needed C89-compatible compiler flags with the installed modern C compiler; their upstream source was not patched. Jim was cloned at the exact commit above and built with its default configure options. Initial offline dependency resolution lacked newer locked crates; these were fetched before the final offline test run. An initial oracle-discovery failure was resolved by building the exact five references, rather than substituting the older system Tcl.

Additional verification included:

- A **110-execution C Tcl matrix**: 22 naming/source/variable cases on each of the five releases, plus the principal cases on latest Jim.
- **78 LSP-focused interpreter executions**: 13 fixtures across six engines, distinguishing unavailable TclOO/path features from failures. Both minifier before/after failures reproduce on every engine.
- **174 compiler/rename replay executions**: 29 original/transformed/live-import scripts per engine. The optimiser's concrete `LOCAL → GLOBAL` rewrite and the forced-import `A → B` edit reproduce on all six engines. Class/method collision outcomes apply to C Tcl 8.6–9.1. These replays use actual current-API transformations generated under the stated fixture profile, and do not by themselves prove every provider was invoked under every profile.
- Fresh-library API probes for colliding identities, Jim colon-run handling, Unicode cursor selection, defaulted/nested brace scans, compiler target/type disagreement, unavailable package selection, empty-tail links, valid-source incremental cache mismatch, source inheritance/propagation and LSP presentation fallbacks.
- Optimised isolated-helper allocation measurements and current-library debug CPU benchmarks as described in section 4. Neither full CI nor release-performance gates were claimed.

To close the profile-versus-replay distinction for the most important rename cases, a second harness independently generated edits under **each of `tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1` and `jim`**, then ran the original/edited source on its matching oracle. All **24 case/profile pairs** confirmed N04, N05 and N07: wildcard and exact imports received declaration-only edits and failed; forced import changed `A` to `B`; unbound proc-variable rename changed only the unrelated global while W210 remained. There was no family-specific rescue of those edit paths.

The report's reproduced snippets and expected outcomes are included above so the findings do not depend on preserving scratch files. Temporary harnesses, oracle builds and raw logs were confined to `/tmp`; they are not additional repository deliverables. Full server protocol end-to-end tests, release-optimised latency/RSS measurements, every vendor dialect, dynamically loaded C extensions and all Tcl runtime behaviors were not exhaustively tested. Package cross-request contamination and the server's four-round rehoming limit are identified as static integration findings rather than completed end-to-end reproductions.

### 6.2 LSP-first and SpecTcl follow-up

The release-scope follow-up remained at the same reviewed hash and made no implementation changes. It traced the production dependencies described in the release checklist, executed the new external N17 probes and ran **54 distinct existing SpecTcl tests**, all passing:

| `tcl-spectcl` test target | Passed | What the result establishes |
|---|---:|---|
| `eval_loader` | 27 | Existing evaluated/static-load equivalence, template/include/vocabulary and failure-policy fixtures. |
| `golden_packs` | 3 | Shipped pack snapshots, legacy-to-2.0 upgrade equivalence and snapshot inventory. |
| `pack_is_real_tcl` | 1 | Shipped pack parse acceptance by required pinned C Tcl 9.0.4. |
| `pack_source_e2e` | 6 | Existing load/install/live-hook and derived-body fixtures; rerun with all five exact C Tcl oracles required. |
| `spec_corpus` | 5 | Existing shipped corpus loading/install/analysis, shared-analysis and hostile-hook fixtures. |
| `roster_reload_static_view` | 2 | Existing surface-roster/static-view reload fixtures. |
| `spectcl_dialect_analysis` | 3 | Existing scoped authoring-vocabulary and loader/analysis agreement fixtures. |
| `workspace_trust` | 7 | Existing provenance/trust-dependent installation and execution behavior. |

These used the freshly built reviewed libraries, `/tmp/naming-review-target`, `--locked --offline --ignore-rust-version`, and the same installed Rust **1.98.1** limitation described above. The first command selected exactly the five compatibility targets:

```sh
cargo test --locked --offline --ignore-rust-version -p tcl-spectcl \
  --test eval_loader --test golden_packs --test pack_source_e2e \
  --test spec_corpus --test pack_is_real_tcl
```

It ran with `TCL_LSP_TCLSH90` pointing to the exact 9.0.4 wrapper and `TCL_REQUIRE_SPECTCL_COMPAT=1`; corpus regeneration/optional corpus expansion variables were unset. Its derived-body test reported skipped external-oracle rows for the other four releases. To remove that gap, the following command reran the six `pack_source_e2e` tests and added the remaining twelve tests:

```sh
cargo test --locked --offline --ignore-rust-version -p tcl-spectcl \
  --test pack_source_e2e --test spectcl_dialect_analysis \
  --test workspace_trust --test roster_reload_static_view
```

The second command set **all five** `TCL_LSP_TCLSH84/85/86/90/91` variables to their verified wrappers and `TCL_LSP_REQUIRE_TCLSH=1`. It completed without skipped oracle rows. Thus the table counts 54 distinct tests, not 60 distinct tests by double-counting the repeated six. Logs are `/tmp/naming-review-spectcl-compat.log` and `/tmp/naming-review-spectcl-focused.log`; no snapshots/baselines were regenerated.

The external SpecTcl probes independently demonstrate N17 despite those green existing tests: wrong command registration through both loader modes, the helper/braced control comparison, and wrong live-hook O129 results under both DSL 1 and 2.0. The template capture driver yielded `good` in all **24 combinations** of six interpreter references, two DSL wrappers and two body shapes. The live-hook probe installed the actual production plan/host/registry; it was not a mock hook. These results expose a missing regression, not a failing test hidden in the reported pass count.

This follow-up did **not** run the complete native/browser/WASI protocol suites, the prescribed Rust 1.99.1 gates or release-profile latency/RSS checks. It did not establish every SpecTcl feature or every hook family's oracle coverage. Required server context/reload proofs remain listed in sections C.1–C.2 and D of the release checklist. A green compatibility suite is necessary for release, but the reproduced wrong registration and unsafe hook result mean it is not sufficient at this hash.

### 6.3 EDA and Jim editor-pack checks

After identifying the separate Jim core-pack inventory, the following **57 additional existing tests** passed at the same hash:

| `tcl-spectcl` test target | Passed | Focus |
|---|---:|---|
| `eda_environment_packs` | 5 | Pack-declared environments, generated shell agreement, aliases and ownership. |
| `eda_loadables` | 8 | All eight packs/1,489 commands, vendor and shared surfaces, roles and dialect routing. |
| `jim_document` | 26 | Jim procedure/statics/class/method analysis and separation from C Tcl. |
| `jim_generation_zero` | 3 | Jim surface availability before discovered-pack publication. |
| `jim_own_surface` | 10 | Jim's own commands, release windows and family separation. |
| `jim_seed_after_publish` | 1 | Seeding preserves already-published rosters. |
| `jim_surface_roster` | 4 | Inherited-command inclusion/exclusion, export/reload and ancestor isolation. |

```sh
cargo test --locked --offline --ignore-rust-version -p tcl-spectcl \
  --test eda_loadables --test eda_environment_packs \
  --test jim_document --test jim_own_surface --test jim_surface_roster \
  --test jim_generation_zero --test jim_seed_after_publish
```

The run used the same installed toolchain and `/tmp` target directory; its log is `/tmp/naming-review-editor-packs.log`. Together with section 6.2, **111 distinct existing SpecTcl/pack tests passed**, in addition to the original 81 naming tests. These are owner/library tests, not a claim that the remaining server/host or per-hook acceptance matrix ran. Existing broad Jim tests also do not invalidate N02's narrower, independently reproduced colon-rule defect.

The current-loader inventory probe separately checked the eight EDA and two embedded Jim sources to distinguish declarative facts from executable `HookDecl` bodies. It found UPF's seventeen executable bodies and no such bodies in the other nine production pack files. Jim's own-surface pack has an informational presentation notice for an intentional native lowering hook; that notice must not be confused with a missing or failed hook. These observations guide editor-facing verification; they do not expand release scope to completing VM/runtime migration.

The review conclusion does not depend on the unexecuted scenarios: the current-source identity collisions, cross-provider disagreements, unsafe edits, wrong compiler rewrite, stale incremental answer, source-path loss, SpecTcl evaluation defect and measured scaling problems establish the editor-facing gaps. Preserve the existing registry, lifecycle and incremental infrastructure while making naming authoritative for LSP analysis and keeping its EDA/Jim/other SpecTcl packs fully functional. VM/runtime paths may retain their current design, including paths used by SpecTcl. The remaining migration follows the editor release.
