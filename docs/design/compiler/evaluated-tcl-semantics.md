# Evaluated Tcl semantics

The compiler, analyser, language server and runtimes share owners for command
bindings, variable storage, execution context and dialect policy. Each consumer
uses a query for its own purpose. A known spelling, available command
description, selected implementation, compiler operation and normal result are
independent facts.

The public query contracts are in
[resolved semantic queries](../contracts/resolved-semantic-queries.md).
The [implementer guide](name-resolution-implementer-guide.md) describes API use,
operand mappings, invalidation and consumer requirements. The
[owner inventory](../contracts/shared-utility-contracts-rust.md) identifies the
shared owner of each semantic axis.

## Commands and namespaces

`tcl_syntax::naming` owns qualified-name construction and lookup candidates.
Lookup uses the current namespace, its selected namespace path and actual
command existence. A matching namespace tail elsewhere in a document supplies
no lookup candidate. Relative qualified names and unqualified names follow
their own Tcl lookup rules.

`SourceCommandBindings` retains the command world at each original invocation.
`invocation_at_source` selects that invocation by its source instance and site.
`lookup_command_word` queries another command value in the retained world; it
does not create another invocation or prove that the command executes.

Aliases retain their target interpreter and inserted argument prefix. Imports
retain the called slot and linked implementation separately. Renaming changes
the command table without treating the old name as another live command.
Namespace allocation, command allocation and implementation incarnation remain
part of dependency identity. A deleted namespace or replaced implementation
cannot be reconstructed from equal text.

Navigation and writable references have separate contracts. A declaration can
be navigable without supplying one unambiguous editable operand. Unknown lookup
alternatives remain explicit even when some alternatives identify a useful
declaration. Registry availability and document stubs supply assistance;
neither proves that an implementation is installed or selected.

## Arguments and compiler selection

Original written words, effective alias arguments and selected private-handler
arguments retain aligned values, origins, role positions and physical reads.
Expansion can change argument count. A captured alias-prefix argument has no
written edit position. Consumers use the shared operand mapping rather than
adjusting offsets independently.

Compiler admission is independent of normal command lookup. A selected native
recipe can retain an operation or private name before evaluating arguments;
ordinary dispatch resolves its handler under the reached runtime rules.
Changes inside command substitutions therefore cannot be modelled by simply
consulting the command table after the whole statement.

The source and executable owners retain the actual admitted operation,
original argv, compiler prerequisites and relevant cache dependencies.
Operation selection sites retain the exact ordinary command compiler receipt
separately from auxiliary implementation requirements. The VM validates that
receipt with the same command-prerequisite validator at its selected chunk or
argument boundary. Imported compiler receipts identify the raw imported token;
the callable origin does not supply its compiler registration. Failed selection
replays the retained original source and namespace, and selected nested ranges
retain their independent continuations.
Compiler failures and expression preparation retain their raw compiler receipts
in the function's admission inventory. Ordinary and ensemble receipts use their
respective registration validators; auxiliary worker and handler lookups keep
independent implementation guards. A copied compiler registration is not
converted into a claim about its callable origin's handler.
A proved generic native selection retains ordinary dispatch. Missing admission
evidence remains an explicit compiler obligation; it supplies no permission to
execute a generic replacement or an apparently equivalent intrinsic.

`RegistryInvocationAssistance::unanimous_command_words` supplies only diagnostic
words on which every closed candidate agrees. It declines possible absence,
unknown residuals and disagreement in identity, values or origins. This view
does not supply executable selection, body entry, effects or writable spans.

## Variable cells and frames

The variable owner distinguishes access syntax, a physical cell address,
contents, alias binding and cell lifetime. The same printed variable name can
refer to different procedure activations, namespaces, interpreter roots or
host execution domains. Array elements retain their complete keys, including
literal sigils; base-name equality is insufficient.

`global`, `variable`, `upvar` and `namespace upvar` establish bindings under
their selected dialect and reached frame. `uplevel` selects the script's
execution frame independently of the frame that performs the selection.
An uninvoked procedure declaration can support conditional local-body analysis,
but cannot supply an actual caller stack or caller-frame target.

Reads and stores retain their concrete order. A substitution captures its
address before later arguments execute. A write uses the reached setter
context. Read traces, write traces, unset traces and object callbacks can change
bindings, contents or observer registrations between those operations.
Successful unobserved reads and normal stores have their own receipts.

SSA identity, current representation, known contents and observer closure are
queried separately. Absence of known traces does not prove arbitrary index
evaluation inert. Unknown frames, dynamic indices or callback effects retain
the affected residual instead of manufacturing a global or scalar cell.

## Scripts and completion

Script roles come from the selected registry invocation and its option-value
layout. Body scripts, expression operands, literal result data and command
prefixes have different grammars and execution contracts. An option's value
width and selector interpretation belong to the shared option owner.

Evaluated source retains its source instance, namespace, lexical configuration,
actual entry and original operands. Conditional declaration analysis and actual
entered execution remain separate. A source range or familiar command name
alone cannot establish a procedure, callback or caller-frame entry.

Completion retains the result object, code, return level and return options.
`catch`, `try`, procedure return and cleanup consume that completion under
their selected interpreter rules. A byte-valued guest error remains a guest
completion until a caller explicitly requests a text projection.

Execution-provider and unmodelled-native-access failures use the host refusal
channel. They cannot be presented as Tcl errors to make `catch` or `finally`
appear to execute successfully. Effects already performed remain visible.

## Packages and dialect policy

A package advertisement, requested version and installed command surface are
different facts. `TrustedPackageLoader` records an explicitly selected provider,
version when known and installed commands/exports. Unknown loading keeps its
effects unknown. Failed loading can leave commands installed; forgetting a
package does not itself remove those commands.

Physical engine, logical invocation dialect, source grammar and command
availability are independent inputs. A compatibility release does not
authenticate a vendor implementation as native C Tcl. Shared queries select
only the protocol supported by their actual inputs and purpose.

Lexer configuration preserves the source coordinates and caller modes while
applying the selected grammar. Numeric syntax, character units, list encoding,
frame rules and expression quoting use their own dialect contracts. A missing
logical policy or physical engine remains missing instead of borrowing the
other axis.

## Original objects and conversion

Native primitive getters are distinct from expression numeric interpretation.
`NativeScalarGetterProtocol` describes the selected wide, double or boolean
getter over the original storage and current cache. Its conversion reports
string-materialisation requirements, cache changes and completion separately.
Cache changes can precede a guest failure.

Adapters retain the original object, preserve the required original string,
apply the full cache change and then present the completion. The returned wide
integer can differ from the cached full magnitude. Primitive error-code
`Unchanged` preserves existing interpreter state; it does not mean set `NONE`.
Script propagation uses the retained primitive diagnostic receipt and its
selected byte-extent rule.

Object-method closure, cache disposition, normal result shape and effect
erasure remain independent. Tcl 9 stock Length can preserve a numeric cache,
while other selected releases can convert it to List. An unknown stock class
or integer-looking string does not establish a numeric representation.
Custom object methods can mutate cells and command bindings during conversion.

## iRules execution domains

Canonical Tcl namespace identity is separate from the host storage domain.
The iRules model retains initialization publication, the current TMM,
connection ownership and event activation. Helpers inherit the domain of their
actual caller; their declaration location cannot select a worker.

`RULE_INIT` supplies initialization values to worker-local static storage.
Other events access the static storage of the TMM executing that event.
Writes on one TMM do not establish a shared mutable value across workers.
Ordinary globals retain their own interpreter/worker ownership. Cross-event
and cross-worker diagnostics consume the same domain facts.

An authored F5 simulation capability is explicit and distinct from an
authenticated native getter or vendor engine. Its logical numeric and quoting
rules do not donate native compiler, object-cache or effect-erasure authority.
Simulator behavior does not certify unmeasured BIG-IP behavior.

## Conservative results

Unknown knowledge, proved absence, rejected input and a normal successor have
different meanings. Each query documents its unknown result and permitted use.
Useful narrow evidence can survive an unrelated unknown, but no consumer may
promote assistance or normal-result evidence into execution or erasure proof.

Changing a shared contract requires the same semantics in its registry
descriptor, SpecTcl loader and renderer, Studio surface, runtime adapter and
consumer projections. Unsupported surfaces are explicit in their contracts.
Native regression fixtures compare the original ingress, object, cache,
completion and observable effects rather than only the printed result.
