# Contract: retained Explorer source views

Explorer source views consume the `CompilationUnit` attached to the result.
The result's dialect string is a presentation label. It does not select a
replacement profile, lexer configuration or command registry.

## Fresh document input

The fresh Explorer pipeline consumes a complete document and selects
`LexerConfig::for_file_grammar` from the resolved environment's grammar.
Only the actual grammar's file policy can skip a leading U+FEFF. Later
U+FEFF characters and extracted script bodies retain their content role.
Configured or rewritten source retains its supplied full policy rather than
selecting another default.

Database document compilation uses the same whole-document constructor in
`LexerCfgKey::to_config`. A supplied document's complete analysis input must
agree with that requested policy before `compilation_unit_for_config`
publishes a unit. Changed config, environment or command store remains a
terminal refusal. The scalar `compiler_check_diagnostics_uncached` entry
has its own explicitly fresh whole-document builder; supplied analysis uses
`compiler_check_diagnostics_uncached_from_analysis` and cannot enter that
scalar route after a refused owner.

## Ownership and selection

`retained_explorer_source` is the common checked projection for event order,
optimiser rewrites and passes, statistics, source annotations, ASM, WASM,
shimmer, GVN, taint warnings, post-SSA CFG, liveness, iRules flow, bounds and
rewritten Explorer results. It requires agreement between the result source, unit source,
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

## Actual-unit consumer boundaries

ASM uses the retained command registry while rebuilding its codegen CFG from
the actual IR with faithful-exception transforms disabled. This is a separate
codegen purpose over the same original Module, not another source ingress.
WASM uses the actual unit and registry; its Native admission, selector,
semantic-plan, layout and lowering guards remain independent.

Shimmer retains its per-function `ShimmerContext` checks. GVN retains the
function's target-neutral semantic sidecar and original site-legality checks.
Taint retains the actual Module realm, function metadata and callback/operand
checks. Post-SSA CFG and liveness retain the actual-unit SSA/def-use artefacts
and the lower dead-store collector's place, observer and hidden-read checks.
These row views do not grant execution or effect closure.

iRules warning aggregation is shared by the warning view and its statistics
count. Individual passes retain their own surface, reachability, event and
operation checks. The unnormalised-getter collector retains the supplied
Module's source interpretation and actual Module/FU metadata before selecting
`NestedScript` over the function's full source config. Explicit standalone
Module/CFG producers are a separate ingress. The existing advisory getter-form
classifier does not establish a runtime getter or Normal completion.

Bounds validates every required FU metadata join to the actual Module. It uses
the actual function input's profile for numerals and character rules and
`source_lexer_config().grammar_over(profile.grammar)` for lexical axes.
`BoundsSemantics` retains that registry and metadata through the shared resolved
bounds and entered-operand division queries. Missing FU ownership withdraws
the view. Source grammar, conditional findings and runtime operation admission
remain separate purposes.

## Serialization and rendering

The retained fields `eventOrder`, `optimisations`, `optimiserPasses`, `stats`,
`annotations`, `annotationsByLine`, `asm`, `wasm`, `shimmer`, `gvn`,
`taintWarnings`, `cfgPostSsa`, `liveness`, `irulesFlow` and `bounds` serialize
as JSON `null` when their
checked source owner is unavailable. An array, including `[]`, is a completed
row inventory. Statistics use an object for an available result. Consumers must
check this structural state before reading fields, counting rows or presenting
an empty-result message. They must not replace `null` with `[]` or zero.

The shared browser renderers and Rust view trees show
“This view is unavailable for this analysis” for unavailable retained views.
Standalone and editor shell badges use the shared `explorerArrayCount` helper;
unknown totals have no count. The combined iRules view retains independent
event or warning rows while showing that its other section is unavailable.
Taint tracking is a separately retained analysis artefact and remains visible
when warning analysis is unavailable. Completed empty arrays keep their
existing empty-view presentation.

## Controls and evidence

The source-owner controls cover actual known C Tcl 8.4, 8.5, 8.6, 9.0 and 9.1,
and Jim profile ingresses, custom lexer configuration, actual command-store
selection, missing and foreign owners, changed source/channel/configuration,
independent rewritten-image ownership, actual-CU label independence,
foreign range-source arguments, required FU withdrawal and warning/count
agreement. Parser controls distinguish a
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
