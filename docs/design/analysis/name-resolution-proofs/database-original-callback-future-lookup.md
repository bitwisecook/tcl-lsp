# naming.database.original-callback-future-lookup

Kind: `implementation-contract`

## Problem statement

A retained original callback name identifies the actual source producer but does not select the namespace, frame or command table where a later callback runs. Using reporting candidates or the printed tail when its future lookup is missing can report an arity error for a different command. The diagnostic consumer must withdraw until that independent lookup exists.

## Question

Does the database callback arity consumer refuse reporting-name candidates when an original input has no independently selected future lookup?

## Conclusion

SignatureCommandInvocation::logical_callback_candidate_keys returns no compatibility candidate keys for a retained original input or callback prefix. The source callback consumer uses the immutable prefix lookup to demand exact current original declaration headers and typed source-signature incompatibility. Missing independent lookup supplies no diagnostic and cannot be replaced by a printed name, reporting reference or mutable candidate list. Legacy exact/range arithmetic remains in its independently selected compatibility purpose. This establishes source-consumer refusal, not callback execution, registration, future frame selection or a native descriptor. The authentic retained source callback receiver additionally joins canonical local headers or independently authored held external target slots. UnknownSourceTarget alone permits genuine external prefix lookup; known source barriers, unavailable frame/interpreter and ambiguous horizons remain terminal. Separate alias-capture/baked/suffix counts survive mutable reporting changes. No external header supplies registration or future callable occupancy.

## Scope

Current database implementation contract. The refusal regression obtains a genuine original cb invocation from analysis, removes its direct lookup and reporting reference, and supplies an appended arity of two. A separate same-name zero-formal declaration remains available in the project. The invocation has no independently issued callback registration/target lookup: compatibility candidate keys remain empty and project callback advice emits no arity diagnostic. This control does not test a populated callback lookup or the positive original-lookup path. Those paths and shared exact/range calculation remain inspected owners. No Rust execution outcome or C Tcl, Jim or BIG-IP result is claimed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native provider was invoked for this Rust callback-consumer contract.

## Consumer bindings

- [rust/tcl-compiler/src/signature_scan/types.rs](../../../../rust/tcl-compiler/src/signature_scan/types.rs), `SignatureCommandInvocation::logical_callback_candidate_keys`: Withdraw compatibility reporting-name candidates for retained original callback/name inputs; actual Logical DB readers share this projection.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `apply_original_callback_arity_with_mode`: Demand current original source declaration headers through the immutable callback prefix lookup and typed source-signature subject.
- [rust/tcl-compiler/src/analyser/diagnostic_subject/callback.rs](../../../../rust/tcl-compiler/src/analyser/diagnostic_subject/callback.rs), `SourceCallbackAritySubject::from_selected_signatures`: Compose exact finite or at-least callback counts from the retained prefix, baked/captured arguments and selected current declaration signatures, then classify mismatch before presentation.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_callback_missing_future_lookup_declines_reporting_candidates` (linked): A genuine original cb invocation with its direct lookup and reporting reference removed has no compatibility candidate keys. Appended Exactly(2) metadata and a same-name zero-formal project declaration cannot manufacture a project callback arity diagnostic; no positive callback registration/target lookup is supplied.

## Evidence

- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs)
