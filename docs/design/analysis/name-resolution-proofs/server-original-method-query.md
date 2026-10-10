# naming.server.original-method-query

Kind: `implementation-contract`

## Problem statement

Definition, references and hover can disagree when their cross-document method fallbacks reconstruct classes or member names from display strings. A valid source candidate can also become stale before its span is converted in another document.

## Question

Do Server definition, reference and hover consumers share one original method selection and independently recheck each owning source before projecting a result?

## Conclusion

Server readonly method consumers use the shared sealed Core query and document-owned candidate. A terminal unavailable or ambiguous original operand returns an empty result before String-based method fallbacks. Definition and references independently check each complete target/consumer image and retained configuration before converting source spans; hover checks its canonical declaration owner before rendering names. The fixed consumer assertion distinguishes opaque sibling routes, checks the provider/reference URI and refuses independent duplicate declarations and stale consumer source. This contract issues no native object dispatch, editable rename plan or provider execution. Shared method selection defers genuine lexical variable reads and retained variable conflicts to the independently current variable provider; an unresolved command or a known external class cannot hijack that source-root query.

## Scope

Server readonly method definition, references/code-lens reference provider and hover integration with C8.6 source fixtures. External class own-table answers are possible source declarations, while independently retained call-point entries remain separate. Class-object makers/delegates and unresolved object receivers are not reconstructed. The named Rust test is a coverage binding, not an execution receipt; all seven native providers are not-tested for this implementation invariant. A real external class provider and separate source-variable controls distinguish unresolved-command and external-class selector positions, preserving each local variable declaration URI and source line. The readonly method-query fixture isolates actual opaque candidate routes, owner URI/config and terminal duplicate/source mismatch before its complete Server request. No reporting label or missing owner can donate Native method identity.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `42be4431c50dbc03d96dca7e69d3eadcfa04d9039fbfe38d2f4f3230a2feb3cf`. Exact current Rust consumer source and linked fixed assertions; no native or Rust execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs). SHA-256 `60ce3a01155c329e41cdae33ce5a3aae7be8d2c7a0f2523d813eefd90c14d4aa`. Exact current Rust consumer source and linked fixed assertions; no native or Rust execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_candidate_at`: Delegate all readonly member lookup to the authentic Core query and URI-owned Workspace candidate.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_locations_at`: Recheck complete target or consumer source/configuration independently before projecting definition/reference spans.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_hover_at`: Recheck canonical owner source before rendering a resolved call or possible declaration summary.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_method_queries_keep_opaque_routes_owners_and_terminal_ambiguity` (linked): Checks typed definition/reference/hover helpers and production definition path, exact opaque route, independent owner URI, duplicate-owner terminal behavior and stale whole consumer source. The readonly method-query fixture isolates actual opaque candidate routes, owner URI/config and terminal duplicate/source mismatch before its complete Server request. No reporting label or missing owner can donate Native method identity.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_method_queries_preserve_lexical_variable_provider_priority` (linked): A genuine lexical source-variable read in an unresolved command or known external class selector keeps method-provider Continue and public Server definition navigation selects its local variable declaration URI/line.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust test is a source coverage binding. No Rust test result or native provider capture is attached to this contract; C Tcl, Jim and BIG-IP have not tested this Rust invariant.
