# Namespace and command resolution

The shared naming owner parses written names and constructs lookup candidates.
The source command-world owner resolves those candidates at the actual invocation.
Consumers retain the resulting slot, implementation and source receipts through
lowering, analysis, optimisation and LSP features.

Use the [semantic query contract](../contracts/resolved-semantic-queries.md) and
[implementer guide](name-resolution-implementer-guide.md) when changing these
APIs.

## Written names and constructed keys

A written name is input to native lookup. A constructed key already identifies
retained namespace segments. Passing a constructed key through written-name
normalisation can change its meaning, particularly for literal colon segments
and empty command names.

`tcl_syntax::naming` owns these distinct operations:

| Input and purpose | Shared operation |
| --- | --- |
| Written command spelling | `canonical_written_command`, `written_command_tail` |
| Constructed namespace/command key | `key_holder_and_tail`, `key_tail` |
| Add or remove a constructed key's root marker | `root_unrooted_key`, `unroot_rooted_key` |
| Ordered candidates for actual namespace and path | `command_resolution_candidates` |
| Path-free candidate enumeration | `bareword_resolution_candidates` |
| C library autoload index keys | `native_autoload_command_candidates` with actual C string protocol; analytical `autoload_command_candidates` |
| Jim global variable key from an actual rooted namespace | `jim_global_variable_key_bytes` |

Candidate enumeration does not establish command existence, a native handler,
procedure identity, or execution. `normalise_qualified_name` remains a
compatibility/display helper; it must not replace a retained physical key or
implementation allocation.

Procedure publication has its own native contract:
`native_procedure::{procedure_name_publication,published_procedure_key}`.
The selected namespace owner can differ from the owner obtained by parsing its
rendered name. Retain the published slot directly instead of looking up that
rendered spelling a second time. The native colon-namespace controls cover the
release differences between C Tcl 8.4/8.5, later C Tcl, and Jim.

## Lookup order and temporal state

Absolute written names select their rooted route. Relative names, including
`inner::p`, search the current namespace, each supported `namespace path` entry,
then the global namespace. Lookup selects the first existing command; the
existence of an intermediate namespace does not prevent later command fallback.
There is no implicit ancestor walk. Namespace path entries are resolved against
the namespace that sets the path, without command-style global fallback.

```rust
use tcl_syntax::naming::command_resolution_candidates;
assert_eq!(
    command_resolution_candidates("::caller", &["::library"], "inner::p"),
    vec!["::caller::inner::p", "::library::inner::p", "::inner::p"],
);
```

The source owner applies the actual dialect's availability and retains every
namespace-path alternative. Its private `ModuleCommandBindings::source_lookup_paths`
is shared by execution, slot presence and navigation. A consumer must not
reconstruct a path from the document's final namespace or search unrelated
namespaces for a matching tail.

At a source invocation, retrieve the original binding through
`SourceCommandBindings::invocation_at_source`. Argument substitutions may change
lookup before dispatch. Compiler admission retains a separate earlier snapshot;
it does not donate the handler selected after arguments evaluate.

| Question | Positioned query |
| --- | --- |
| Does the called slot exist? | `SourceInvocationBinding::selected_slot_presence` |
| Is missing-command advice supported? | `selected_slot_diagnostic_presence` |
| Which slot supports navigation? | `command_reference` |
| Which original terminal declaration supports navigation? | `linked_definition` |
| Which implementation executes after argv? | `proved_execution_target` |
| Which converged handler supplies a normal-transfer contract? | `proved_handler_target` with its purpose-specific adapter |
| Which recipe was admitted before argv? | `admitted_inline_invocation`, `admitted_named_invocation` |

Missing evidence remains unknown. A present alias can have a missing terminal
command. A missing slot can invoke an autoloader or custom `unknown` handler.
A procedure token can survive redefinition while its implementation allocation
changes. An import retains its native origin relationship through rename and
retirement. These distinctions cannot be represented by one qualified-name map.

## Consumer and test requirements

Lowering, compiler emitters, variable transfer, call graphs, taint, optimisation,
and LSP navigation consume their specific retained proof. Final procedure and
class maps supply declaration assistance, rather than temporal execution
identity. Loaded or materialised sources also need their own source origin;
matching name bytes cannot provide an editable authored span.

When changing lookup, test current/path/global order, relative qualified names,
absolute names, empty names, literal colons, path alternatives, imports and
aliases, deletion/recreation, redefinition, and mutations during argv. Pair
known namespace absence advice with an external unknown namespace. Run the
shared command-resolution and execution vectors against all five pinned C Tcl
releases and current Jim, then check every affected consumer and cache. Keep
native observations, source-analysis tests and emitted execution tests separate
when evaluating each contract.
