# naming.server.original-method-query

Kind: `implementation-contract`

## Problem statement

Definition, references and hover can disagree when their cross-document method fallbacks reconstruct classes or member names from display strings. A valid source candidate can also become stale before its span is converted in another document.

## Question

Do Server definition, reference and hover consumers share one original method selection and independently recheck each owning source before projecting a result?

## Conclusion

Server readonly method consumers use the shared sealed Core query and document-owned candidate. A terminal unavailable or ambiguous original operand returns an empty result before String-based method fallbacks. Definition and references independently check each complete target/consumer image and retained configuration before converting source spans; hover checks its canonical declaration owner before rendering names. The fixed consumer assertion distinguishes opaque sibling routes, checks the provider/reference URI and refuses independent duplicate declarations and stale consumer source. This contract issues no native object dispatch, editable rename plan or provider execution. Shared method selection defers genuine lexical variable reads and retained variable conflicts to the independently current variable provider; an unresolved command or a known external class cannot hijack that source-root query.

Hover may independently render the sealed Registry pattern or format operand when the local method selector abstains before issuing a query. An authentic method query rejected by workspace occupation, ambiguity or target currency remains terminal. The source advice keeps its original schema, source image, configuration and input; Server rejoins the actual caller publication, saved bytes, generation and configuration epoch before returning it. The same conditional catalogue supplies its counted source-coordinate paths and Registry operand; any indexed command, class or procedure publication in any retained path with the same naming policy withdraws the card. Missing path coverage is terminal. All original handler, holder and lookup obligations remain unresolved and the card identifies itself as conditional source-pattern advice. An empty indexed match establishes no runtime absence. This is embedded-language source advice, not a method or runtime binding.

## Scope

Server readonly method definition, references/code-lens reference provider and hover integration with C8.6 source fixtures. External class own-table answers are possible source declarations, while independently retained call-point entries remain separate. Class-object makers/delegates and unresolved object receivers are not reconstructed. The named Rust test is a coverage binding, not an execution receipt; all seven native providers are not-tested for this implementation invariant. A real external class provider and separate source-variable controls distinguish unresolved-command and external-class selector positions, preserving each local variable declaration URI and source line. The readonly method-query fixture isolates actual opaque candidate routes, owner URI/config and terminal duplicate/source mismatch before its complete Server request. No reporting label or missing owner can donate Native method identity.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_candidate_at`: Delegate all readonly member lookup to the authentic Core query and URI-owned Workspace candidate.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_locations_at`: Recheck complete target or consumer source/configuration independently before projecting definition/reference spans.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_method_hover_at`: Recheck canonical owner source before rendering a resolved call or possible declaration summary.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_method_queries_keep_opaque_routes_owners_and_terminal_ambiguity` (linked): Checks typed definition/reference/hover helpers and production definition path, exact opaque route, independent owner URI, duplicate-owner terminal behavior and stale whole consumer source. The readonly method-query fixture isolates actual opaque candidate routes, owner URI/config and terminal duplicate/source mismatch before its complete Server request. No reporting label or missing owner can donate Native method identity.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_method_queries_preserve_lexical_variable_provider_priority` (linked): A genuine lexical source-variable read in an unresolved command or known external class selector keeps method-provider Continue and public Server definition navigation selects its local variable declaration URI/line.

## Evidence

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs)
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs)
