# Command aliases, renames and source advice

A command name selects a mutable interpreter slot. A rename transfers its
command object to another slot; an alias retains a target-name lookup and frozen
prefix arguments. Current lookup, readonly source metadata and reporting
candidates have separate shared owners. Every consumer selects the owner for
its purpose and retains that owner's refusal and applicability obligations.

The interpreter contract is documented in
[command-binding-and-aliasing.md](command-binding-and-aliasing.md), and the
engine-specific lookup rules in [command-resolution.md](command-resolution.md).
The [consumer matrix](naming-consumer-matrix.md) and
[implementer guide](../compiler/name-resolution-implementer-guide.md) identify
the complete original inputs required by each public surface.

## Mutable command identity

For a successfully reached rename, the destination retains the original command
implementation. A later declaration at the old source name supplies a separate
implementation:

```tcl
proc p {} {return first}
rename p oldp
proc p {} {return second}
```

An alias looks up its retained target name at invocation time. Moving the alias
preserves its captured prefix; moving its target can make that later lookup
fail. Nested aliases preserve inner-before-outer capture order. The
[finite native alias controls](../analysis/name-resolution-proofs/callback-alias-capture-source-control.md)
record their exact public source vectors and provider outcomes. Those
observations do not establish an arbitrary source site's original object,
compiler admission, handler, current namespace or entered frame.

A source statement expressing a mutation cannot establish that the mutation ran
or succeeded. Unknown preceding effects, conditional branches, callbacks and
uncalled bodies retain independent applicability. A later textual definition
cannot replace a positioned lookup at an earlier call.

## Shared purpose owners

| Consumer purpose | Shared owner | Required boundary |
| --- | --- | --- |
| Native invocation and handler selection | `SourceInvocationBinding`, `proved_execution_target`, reached handler projections | Genuine original invocation, selected interpreter/realm, independently retained lookup and implementation. Catalogue names cannot supply missing proof. |
| Readonly command arguments, expressions and scripts | `registry_invocation::source_structure::OriginalRegistryWords` | Complete unchanged image, full lexical configuration, actual ContextRegistry and authentic effective argv. Selected or conditional schema retains its source obligations. |
| Source callback inventory | `OriginalCallbackPrefix` | Exact selected installer and prefix producer, captured arguments and appended-count grammar. A known installer replacement/deletion remains terminal. |
| Source callback signature advice | `source_callback_procedure_target_at`, `SourceCallbackSignatureLookup` | Genuine registration horizon, structured target/refusal and canonical local header or independently held external target. A missing source horizon withholds external header assistance. |
| Source class and receiver metadata | `source_class_reference_at`, `source_class_instance_words_at` | Canonical original declaration and genuine operand/constructor/receiver ancestry. An ordinary alias does not acquire direct class-object reference identity. |
| Unresolved command diagnostics | Positioned command-slot presence owner | Authentic original head and lookup purpose, including unknown fallback alternatives. Message text and final callable reports cannot settle presence. |
| String reporting compatibility | `analyser::indirection::walk`, `Indirection::resolve_at` | Representable retained reporting records and their bounded order model. Their target labels supply no Native callable or allocation receipt. |

Source command-prefix and script projections preserve the whole selected
argument vector. An aliased expression command can therefore retain its
original expression operand for variable-reference and syntax consumers:

```tcl
interp alias {} arithmetic {} expr
proc calculate {x y} {
    return [arithmetic {$x + $y}]
}
```

That readonly role does not admit expression compilation or establish that the
procedure body is entered. Captured operands retain their producer anchors and
cannot borrow a later written span. Script queries distinguish Syntax inventory
from PotentialEvaluation timing; reference-only bodies retain Syntax only.

## Reporting compatibility

`indirection::walk` reads `command_aliases` / `alias_offsets` and
`renamed_commands` / `rename_offsets` under one representable reporting model.
The latest eligible reporting event determines the next hop.
`Indirection::resolve_at` retains the reporting time associated with a moved
implementation, while aliases retain late target-name lookup. The walk is
bounded by `MAX_COMMAND_NAME_HOPS` and refuses self-aliases and captured-prefix
chains that its String-only result cannot represent.

Its body-order convenience is a lexical reporting assumption, not proof that
the complete file loaded before every body runs. Native consumers retain the
original positioned command-binding owner instead. Class-factory report
inventory can use the bounded walk to locate a reported metaclass layout;
that inventory supplies neither a class allocation nor successful construction.

## Refusal boundaries

Complete original source/configuration/context ownership is mandatory for source
queries. Stale or foreign words, unavailable realm/namespace, dynamic selectors,
unknown expansion and missing canonical declarations retain the appropriate
refusal. A represented refusal does not permit a fresh nominal scan.

Known source command deletion, replacement, nonprocedure targets and cycles are
terminal where the shared source horizon represents them. Unknown earlier
mutations preserve conditional alternatives and explicit obligations; they do
not become known deletion or absence.

Child-interpreter aliases retain independent source and target interpreter
contexts. The child's command table cannot be projected into the parent by a
shared display spelling. Cross-document source and package relationships supply
only their retained ordering and context; byte offsets order positions within
one document. Native and hosted names select their own publication/lookup
protocols instead of reconstructing qualified String keys.

## Key files

| File | Role |
| --- | --- |
| `rust/tcl-compiler/src/command_binding.rs` and `command_binding/` | Original invocation, implementation, lookup and conditional source owners. |
| `rust/tcl-compiler/src/registry_invocation.rs` and `registry_invocation/source_structure.rs` | Purpose-specific effective words, roles and full-context source facades. |
| `rust/tcl-compiler/src/analyser/indirection.rs` | Bounded String reporting walk and as-of comparisons. |
| `rust/tcl-compiler/src/analyser/handlers.rs` | Mutation reporting and source/factory consumers. |
| `rust/tcl-lsp-core/src/receiver_identity.rs` | Original receiver and source assistance joins. |
| `runtime/rust/src/cmd_alias.rs` | Runtime alias operations and retained contexts. |
