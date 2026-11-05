# naming.procedure.original-formal-count-shape

Kind: `native-observation`

## Problem statement

Which original formal signatures accept each source argv count under C Tcl 8.4-9.1 and current Jim, including positional defaults, rest anywhere vs final, and Apply alias/rename shapes?

## Question

Which argc0..5 counts do seven exact original procedure formal lists accept, and how do the fixed Apply/rooted/alias/captured-lambda/rename calls preserve or refuse their original count shape?

## Conclusion

All five C releases have the same seven finite procedure acceptance sets: an empty list accepts0; required-after-default and both middle-args spellings accept3; trailing args accepts1..5; defaults-around-middle-args accepts3,4; two optional parameters accept0..2. Jim instead accepts2,3 for required-after-default,2..5 for the middle-rest spellings and1..5 for defaults-around-rest, while the other tested sets agree. Every provider refuses the malformed three-field parameter. Tcl8.4 Apply and current Jim interp alias setup remain unsupported controls; successful setup and actual call results remain separate. The same already-decoded formal count owner serves strict native binder and original lambda source signature. C reserves rest semantics for final args and uses positional default slots; Jim permits rest anywhere and reserves required slots while skipping defaults. An independent expected acceptance matrix drives the actual shared parser and binder for exactly the seven successful original definitions and argc0..5 plus malformed definition refusal observed by all six provider CLI transcripts. Ordinary analyser and signature-scan declarations retain authentic ordered formal bytes and selected C/Jim parameter grammar before projecting count. ProcDef, MethodDef and SignatureProc share SourceFormalCount; computed/malformed/unowned contracts stay Unknown/open. Explicit authored metadata is separate from Original storage. Header consumers can copy only the body-free Eq count/origin/grammar projection, which cannot recreate a source receipt. The new test independently encodes exactly Formal185 seven successful definitions and caught acceptance argc0..5 for all C5/Jim providers, compares analyser and signature records, and keeps malformed/computed refusal controls. An additional marked software count control keeps finite Registry representability separate from an unbounded formal-count shape: unrepresentable finite bounds withdraw rather than become unlimited metadata.

## Scope

Six actual process exits0 and empty stderr retain six queried versions and372 caught observations:42 procedure calls, seven successful definitions, one malformed definition and12 Apply/setup/call controls per provider. Each process uses the same sequential source state; definitions are removed between cases and Apply rename is the final control. Observed acceptance is bounded to argc0..5 and these exact formal lists. Binary-scan result projection is not an original native getter/count/pointer measurement. CLI setup and caught outcomes provide no formal value assignment, runtime cell/frame ownership, alias object identity, compiler admission, successful unrelated body execution, physical header/cache or BIG-IP result. Native caught definition/call status observations belong to independent all-six receipts and original ASCII source. Their finite observed range does not establish counts above five, value assignment, runtime frame/cell ownership, native getter/header identity or actual Rust execution. FormalProof186 selector is published only after exact Root LIVE ACK. Current immutable source declaration counts retain independently selected parameter grammar and original storage separately from explicit authored metadata. Body-free Eq count/origin/grammar projections cannot recreate a source receipt. The finite acceptance expectations are independent of native calls, frames/getters/headers or counts above five. The sentinel and large-count assertions are software metadata limits. The independently retained original Native proof observes only its exact argv count window; it establishes no large-count acceptance, allocation, procedure entry or new assertion outcome.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI, one fresh process per provider with one queried version and 62 sequential caught definition/call controls. Result hex is binary-scan source projection. Version: tcl8.4 8.4.20.

For tested argc0..5, accepted counts: empty=0; required-after-default=3; middle-args=3; middle-rest-name=3; trailing-rest=1,2,3,4,5; defaults-around-rest=3,4; all-optional=0,1,2. All seven definitions succeed and the malformed three-field formal definition is refused. Apply is unavailable; alias creation succeeds without proving its target can run, and the target invocations and rename fail. These controls provide no Apply body or alias-call acceptance.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI, one fresh process per provider with one queried version and 62 sequential caught definition/call controls. Result hex is binary-scan source projection. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

For tested argc0..5, accepted counts: empty=0; required-after-default=3; middle-args=3; middle-rest-name=3; trailing-rest=1,2,3,4,5; defaults-around-rest=3,4; all-optional=0,1,2. All seven definitions succeed and the malformed three-field formal definition is refused. Direct Apply accepts its exact two-argument case and refuses short/surplus, required-after-default and middle-rest controls. Rooted, ordinary alias, captured-lambda alias and renamed short calls retain arity failures after successful setup. Error codes are NONE in C8.5 and TCL WRONGARGS in C8.6–C9.1.

### jim

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI, one fresh process per provider with one queried version and 62 sequential caught definition/call controls. Result hex is binary-scan source projection. Version: jim 0.84-9-g5bac7c9.

For tested argc0..5, accepted counts: empty=0; required-after-default=2,3; middle-args=2,3,4,5; middle-rest-name=2,3,4,5; trailing-rest=1,2,3,4,5; defaults-around-rest=1,2,3,4,5; all-optional=0,1,2. All seven definitions succeed and the malformed three-field formal definition is refused. Direct Apply accepts the exact two-argument case and the two-argument required-after-default and middle-rest cases. Its short/surplus/root-separator and renamed short controls fail. Both interp alias setups are unsupported; following missing alias calls prove no captured-prefix acceptance.

### bigip

Status: `not-tested`.

No appliance observation of these exact original formal-count and call controls.

## Consumer bindings

- [rust/tcl-syntax/src/formal_params.rs](../../../../rust/tcl-syntax/src/formal_params.rs), `FormalArgumentCountShape`: Share independently selected formal count/value topology and original effective count/operand ownership; descriptive or binder API coverage grants no entered lambda frame, Native handler or native object/header correspondence.
- [rust/tcl-syntax/src/formal_params.rs](../../../../rust/tcl-syntax/src/formal_params.rs), `formal_argument_count_shape`: Share independently selected formal count/value topology and original effective count/operand ownership; descriptive or binder API coverage grants no entered lambda frame, Native handler or native object/header correspondence.
- [rust/tcl-syntax/src/formal_params/bytes.rs](../../../../rust/tcl-syntax/src/formal_params/bytes.rs), `bind_formal_argument_bytes`: Share independently selected formal count/value topology and original effective count/operand ownership; descriptive or binder API coverage grants no entered lambda frame, Native handler or native object/header correspondence.
- [rust/tcl-compiler/src/signature_scan/formal_parameters.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_parameters.rs), `SignatureSourceFormalParameterValue::argument_count_shape`: Share independently selected formal count/value topology and original effective count/operand ownership; descriptive or binder API coverage grants no entered lambda frame, Native handler or native object/header correspondence.
- [rust/tcl-compiler/src/signature_scan/arity.rs](../../../../rust/tcl-compiler/src/signature_scan/arity.rs), `arity_of_in`: Share independently selected formal count/value topology and original effective count/operand ownership; descriptive or binder API coverage grants no entered lambda frame, Native handler or native object/header correspondence.
- [rust/tcl-compiler/src/signature_scan/formal_count.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_count.rs), `SourceFormalCount`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/signature_scan/formal_count.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_count.rs), `SourceFormalCountProjection`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/signature_scan/formal_count.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_count.rs), `SourceFormalCount::projection`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/signature_scan/formal_parameters.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_parameters.rs), `SignatureSourceFormalParameters::parameter_grammar`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/signature_scan/arity.rs](../../../../rust/tcl-compiler/src/signature_scan/arity.rs), `arity_from_count_shape`: Check the shared FormalArgumentCountShape before narrowing into Registry Arity; a finite bound cannot borrow the unlimited sentinel.
- [rust/tcl-compiler/src/signature_scan/types.rs](../../../../rust/tcl-compiler/src/signature_scan/types.rs), `SignatureProc::formal_count_projection`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `ProcDef::formal_count_projection`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `MethodDef::formal_count_projection`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `Analyser::declared_procedure_formal_count`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/signature_scan/ctx.rs](../../../../rust/tcl-compiler/src/signature_scan/ctx.rs), `ScanCtx::formal_count`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-compiler/src/analyser/original_members.rs](../../../../rust/tcl-compiler/src/analyser/original_members.rs), `member_formal_count`: Retain original formal/value topology and exact selected source correspondence separately from written call-site operands, binding success and native object/frame facts.
- [rust/tcl-syntax/src/formal_params.rs](../../../../rust/tcl-syntax/src/formal_params.rs), `formal_params::tests::formal_count_shapes_preserve_c_and_jim_default_and_rest_grammars` (linked): The same already-decoded formal count owner serves strict native binder and original lambda source signature. C reserves rest semantics for final args and uses positional default slots; Jim permits rest anywhere and reserves required slots while skipping defaults. An independent expected acceptance matrix drives the actual shared parser and binder for exactly the seven successful original definitions and argc0..5 plus malformed definition refusal observed by all six provider CLI transcripts.
- [rust/tcl-syntax/src/formal_params.rs](../../../../rust/tcl-syntax/src/formal_params.rs), `formal_params::tests::native_formal_acceptance_matrix_keeps_provider_count_answers` (linked): The same already-decoded formal count owner serves strict native binder and original lambda source signature. C reserves rest semantics for final args and uses positional default slots; Jim permits rest anywhere and reserves required slots while skipping defaults. An independent expected acceptance matrix drives the actual shared parser and binder for exactly the seven successful original definitions and argc0..5 plus malformed definition refusal observed by all six provider CLI transcripts.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `analyser::handlers::original_registry_symbol_tests::original_formal_counts_survive_procedure_and_signature_records` (linked): Seven independently encoded finite successful formal definitions and argc0..5 C5/Jim acceptance expectations survive authentic analyser/signature records; original ordered storage stays separate from authored metadata, with malformed/computed refusal and no actual call/frame or above-five conclusion.
- [rust/tcl-compiler/src/signature_scan/arity.rs](../../../../rust/tcl-compiler/src/signature_scan/arity.rs), `signature_scan::arity::tests::finite_source_count_bounds_cannot_borrow_unlimited_sentinel` (linked): Checked source metadata conversion rejects finite minimum or maximum bounds at or beyond the u16 unlimited sentinel, while representable exact and independently unbounded shapes remain distinct.

## Evidence

- [rust/tcl-registry/tests/data/native_original_formal_count](../../../../rust/tcl-registry/tests/data/native_original_formal_count)
- [rust/tcl-registry/tests/data/native_original_formal_count/8.6.18](../../../../rust/tcl-registry/tests/data/native_original_formal_count/8.6.18)
- [rust/tcl-registry/tests/data/native_original_formal_count/9.1.0](../../../../rust/tcl-registry/tests/data/native_original_formal_count/9.1.0)
- [rust/tcl-registry/tests/data/native_original_formal_count/8.4.20](../../../../rust/tcl-registry/tests/data/native_original_formal_count/8.4.20)
- [rust/tcl-registry/tests/data/native_original_formal_count/9.0.4](../../../../rust/tcl-registry/tests/data/native_original_formal_count/9.0.4)
- [rust/tcl-registry/tests/data/native_original_formal_count/8.5.19](../../../../rust/tcl-registry/tests/data/native_original_formal_count/8.5.19)
- [rust/tcl-registry/tests/data/native_original_formal_count/jim](../../../../rust/tcl-registry/tests/data/native_original_formal_count/jim)
