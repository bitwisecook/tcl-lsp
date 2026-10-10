# Original procedure completion source context

Proof ID: `naming.interprocedural.original-procedure-completion-source-context`. Kind: `implementation-contract`.

## Problem statement

A procedure-completion summary can borrow a reported procedure label, changed header or detached command spelling and then be reused by dead-store elimination despite lacking the original declaration, effective alias argv, selected formal grammar or genuine child lookup horizon. Source completion must remain independent of reached Native execution and the other erasure prerequisites.

## Question

How do interprocedural completion summaries and their deletion consumer share the exact original Logical Module, declaration allocation/header, selected formals and effective child invocations while preserving source grammar and independent runtime/error/edit obligations?

## Conclusion

RetainedSourceModuleBindings authenticates the original Lowerer-produced procedure header, body coordinates/source and statement inventory under the same complete Module owner. The typed original Logical call joins exact target allocations, genuine point metadata and effective captured/written argv to one matching retained declaration. Selected parameter/list grammar determines count and direct local scalar bindings; links/elements and unknown expansion cannot borrow those names. CompletionWalk checks original operands and each child invocation at its actual retained horizon. Only the conditional straight-line scalar-store/return model and independently accepted source calls enter the completion fixed point; missing/foreign/Native-only source, changed headers/body, unknown reads, traces, potential errors and recursive dependencies retain refusal. Definition reach stays a separate call premise. The elimination no-error query uses this same Module/header join, independently of SCCP presence, purity, observers and source edit permission.

## Scope

Eleven marked software controls comprise ten conditional completion models and one actual elimination no-error consumer. Six existing completion examples are explicitly interpreted as Logical source assertions, while the other controls retain alias/default/child, scalar/element, complete-owner and selected Jim formal strategy boundaries. Direct grammar helper assertions are lexical obligations, not original procedure receipts. This question has no executed Rust assertion or external C/Jim/BIG-IP process result. The source model grants no entered Native frame, successful Handler/Normal, argument value, physical cell/read, native formal installation, compiler instruction or executable erasure/inlining equivalence.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: f5-bigip.

No external provider execution answers this original Logical procedure-completion contract. Selected Tcl/Jim grammar fixtures test conditional source ownership/count/read obligations, not actual procedure activation, Native Normal or deletion equivalence.

## Exact evidence

- `naming-interprocedural-original-procedure-completion-source-context-completion-definition` (implementation): [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs). SHA-256 `732155a21ad495e484443e034241b2ffad01c6016a6760303b5ff5b6be649533`. Current Root-ACKed396/397 conditional source walk and ten marked completion definitions; no executed assertion or provider result.
- `naming-interprocedural-original-procedure-completion-source-context-call-definition` (implementation): [rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs](../../../../rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs). SHA-256 `74784b6eeacfb1536d60458b717ec229670abb4be5a9b5261b3f372667a9abb0`. Typed original declaration/effective argv/selected formal ownership query; no Native frame, argument values or completion admission.

## Source inspection

The current Rust source defines the shared original declaration/effective argv/formal join and the conditional completion/no-error consumers. No upstream Tcl source excerpt or external provider experiment establishes this Logical source model. The [inlining source context](inlining-original-frame-source-context.md) and [actual analysis metadata](original-analysis-metadata-context.md) questions retain their independent source geometry, availability and missing-owner boundaries.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `RetainedSourceModuleBindings::matches_original_procedure`: Compare the Lowerer-retained original declaration name/key, header formals, body source/coordinates and direct statement inventory. Each child still requires its own authentic token and selected-operation receipt.
- [rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs](../../../../rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs), `original_logical_procedure_calls_for_module`: Require positive complete Logical Module/point metadata, exact original target allocation and one matching retained procedure before forming effective captured/written argv. Equal reported names or offsets cannot select a declaration.
- [rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs](../../../../rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs), `original_procedure_formal_count_shape`: Retain selected formal/list grammar and the same raw/parsed original declaration count shape. Descriptive accepted count supplies no actual values, local cells, activation or completion.
- [rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs](../../../../rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs), `original_procedure_scalar_bindings`: Intersect direct local scalar names across every accepted selected formal-binding branch. Jim defaults/rest strategy is retained; caller links and empty/nonlocal/element destinations decline the scalar source model.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `CompletionWalk::for_module`: Require complete current retained Module source/Registry correspondence and positive Logical metadata before the conditional word/child invocation walk; Native-only, missing or foreign owners issue no source completion.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `CompletionWalk::word_in`: Authenticate the genuine parent operand and selected original child inventory under full retained source grammar. A detached spelling cannot donate original lookup or procedure ownership.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `procedures_complete`: Compute only the conditional straight-line Logical completion fixed point under exact original declaration/formal/read/child receipts and independent callee-definition reach. Potential errors, traces, unresolved calls and recursive dependency cycles remain refusal.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `RaiseProof::calls_complete`: Join possible call completion to the same original Module/header and checked child source walk. This no-error query alone supplies no presence, purity, observer, erasure or Native execution licence.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::a_callee_counts_as_defined_only_where_its_definition_surely_ran` (linked): Conditional source completion retains the callee-definition reach/order premise, including genuine early calls, conditional or nested declarations and transitive call dependencies. Source callback ordering is not observed runtime dispatch.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::a_word_the_release_rejects_never_completes` (linked): The Logical source model refuses selected malformed whole words and unsupported expansion syntax. Its clean selected source counterpart remains separate from a native compiler or successful invocation receipt.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::a_body_of_commands_that_complete_completes` (linked): Straight-line original scalar stores/returns and accepted calls form the conditional completion model only under genuine retained declarations, bound source names and child invocation selection; no entered procedure or Native Normal is asserted.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::a_body_that_may_raise_does_not_complete` (linked): Expressions, unknown reads, element/nonlocal destinations, expansion/control/link effects, rejected call counts, raising callees and recursive cycles retain potential failure instead of stock completion.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::a_rebound_or_declared_head_states_no_completion` (linked): Known source replacement/rename and document stub declarations cannot borrow builtin completion; a source purity flag supplies no successful-return premise.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::the_ir_only_summary_states_no_completion` (linked): IR-only lowering with reported profile lacks genuine source command ownership and states no completion. A positively retained Logical source unit supplies its separate conditional model.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::original_completion_keeps_alias_prefixes_defaults_and_child_horizons` (linked): Authentic original procedure/default formals and effective alias prefixes retain exact accepted counts and checked child invocation horizons. Wrong count, selected raising/unknown targets and child failure refuse completion.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::original_completion_keeps_selected_scalar_reference_and_element_obligations` (linked): Bound scalar references retain Unicode, literal dollars and unmatched scalar parentheses under the original source grammar. Elements/dynamic indices and unbound names refuse. Direct FirstClose/Tcl9 lexical helper cases test grammar obligations only and are not fabricated procedure declarations.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::original_completion_declines_missing_foreign_native_and_mutated_headers` (linked): The complete Module/Registry owner and exact procedure key/name, raw/parsed formals, body coordinates/source and statement inventory are required. Missing/foreign/stale or changed header/body and Native-only source units refuse conditional completion.
- [rust/tcl-compiler/src/interprocedural/completion.rs](../../../../rust/tcl-compiler/src/interprocedural/completion.rs), `interprocedural::completion::tests::original_completion_retains_selected_jim_formal_binding_strategy` (linked): An explicitly Logical Jim source model retains selected defaults, rest-position/named-rest count strategy and intersection of direct scalar bindings. Caller-link and array destinations cannot borrow those local scalar obligations; no Jim activation is observed.
- [rust/tcl-compiler/src/optimiser/elimination.rs](../../../../rust/tcl-compiler/src/optimiser/elimination.rs), `optimiser::elimination::tests::original_completion_deletion_consumer_uses_same_module_and_header` (linked): The elimination call no-error query joins its summary to the same actual Module, original header and Registry. Missing Module, foreign registry or changed formals withdraw it; SCCP presence, observers, purity and edit permission remain independently required.

A coverage binding records a defined assertion, not an executed result.

## Replay

Named controls bind current authored source/API assertions and do not claim execution. Exact compiled-artifact/test receipts retain their actual outcomes separately. Native formal, frame, Handler/Normal and erasure obligations remain independent of this Logical source model.
