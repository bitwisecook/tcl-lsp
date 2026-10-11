# Contract: retained Explorer source views

Explorer source views consume the `CompilationUnit` attached to the result.
The result's dialect string is a presentation label. It does not select a
replacement profile, lexer configuration or command registry.

## Ownership and selection

`retained_explorer_source` is the common checked projection for event order,
optimiser rewrites and passes, statistics, source annotations and rewritten
Explorer results. It requires agreement between the result source, unit source,
Module document image, original source interpretation, actual registry snapshot,
complete resolved input and full lexer configuration. The existing positioned
Module projection also checks entry, namespace and binding interpretation.
Missing, foreign or changed owners withdraw these views. A label, default
registry or newly scanned equivalent source cannot repair that withdrawal.

`document_realm_bindings_for_module` is a readonly projection of the actual
Module's retained positioned command bindings. It does not issue Native entry,
compiler selection, frame, completion, allocation or editable-span receipts.
Consumers must still retain any independently required operation owners.

Event order uses
`top_level_when_handlers_with_source_config_and_head_resolver` with the actual
registry, complete parent lexer configuration, explicit `WholeInput` purpose
and retained realm resolver. `NestedScript` is an explicit separate parser
purpose and selects `parent_config.nested()`. Both event boundaries and inherited
priority declarations use the same selected parser configuration. The
standalone source event API remains a separate fresh-source ingress.

Optimiser-derived views use `optimise_unit` with the actual unit and retained
registry/profile. Per-pass output uses the existing actual-unit pass API.
Source-only optimisation remains an explicit fresh-source API and does not
replace a refused retained query.

Rewritten source is a new document image. Its compilation retains the supplied
resolved input, configuration, profile and command store, and establishes its
own source interpretation. It receives no execution receipt or old-image
ownership from the source being rewritten.

## Serialization and rendering

The retained fields `eventOrder`, `optimisations`, `optimiserPasses`, `stats`,
`annotations` and `annotationsByLine` serialize as JSON `null` when their
checked source owner is unavailable. An array, including `[]`, is a completed
row inventory. Statistics use an object for an available result. Consumers must
check this structural state before reading fields, counting rows or presenting
an empty-result message. They must not replace `null` with `[]` or zero.

The shared browser renderers and Rust view trees show
“This view is unavailable for this analysis” for unavailable retained views.
Standalone and editor shell badges use the shared `explorerArrayCount` helper;
unknown totals have no count. The combined iRules view retains independent
warning rows while showing that its event section is unavailable. Completed
empty arrays keep their existing empty-view presentation.

## Controls and evidence

The source-owner controls cover actual known C Tcl 8.4, 8.5, 8.6, 9.0 and 9.1,
and Jim profile ingresses, custom lexer configuration, actual command-store
selection, missing and foreign owners, changed source/channel/configuration,
and independent rewritten-image ownership. Parser controls distinguish a
leading U+FEFF under `WholeInput` and `NestedScript`; they establish parser
configuration behavior, not BIG-IP configuration acceptance.

Wire and render controls consume a payload whose real retained interpretation
is removed and distinguish it from completed empty and current event payloads.
The Node driver executes shipped JavaScript with a DOM stand-in. It is not a
browser execution or Tcl provider experiment. The Rust view-tree controls cover
null, absent and empty row fields and independent iRules warning preservation.
These source and presentation controls supply no runtime C/Jim/BIG-IP result,
Native operation admission or measured performance improvement. Provider
observations and execution receipts remain separate evidence in the
[name-resolution proof directory](../analysis/name-resolution-proofs/README.md).

## Implementer usage

For a view over an existing result, obtain the checked actual Module projection
and retain its full input/configuration/store through every consumer. Select
parser purpose separately from the parent metadata owner. Use the actual
`CompilationUnit` for optimiser-derived information. If the check refuses,
serialize unavailable and let presentation consume that state.

For independently supplied new source, use an explicitly named fresh-source
producer with the supplied complete metadata. Do not carry old-image execution
or source receipts into that producer. Add a positive current-owner control
and a control changing exactly the relevant owner axis; record whether the
assertion establishes source structure, metadata, rendering or actual provider
execution. Keep later Native admissions and refusal controls independent.
