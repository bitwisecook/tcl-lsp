# Proposed typed source configuration-reference metadata

Question: can a genuine selected source schema retain a literal data-group
candidate when its ordinary ITEM value is computed, without guessing that
value or selecting a native handler?

The [unchanged MCP failure](../validation-01/README.md) reports generated
datagroups `[]` against `["helper_dg"]`. The single-capture assertion passed
before that failure. Read-only inspection locates the first loss in
`tcl-irules/src/source_object.rs:155`: the custom resolver requires every
post-head operand literal. `class match [HTTP::uri] equals helper_dg` supplies
a Dynamic ordinary ITEM, so that complete-literal collection declines. MCP
only gathers the shared returned categories; it has no separate name parser.

The proposed [scratch patch](registry-irules.patch) is not applied, compiled
or tested. [Preimages](baseline.json) and [file identities](files.json) retain
its exact inputs and outputs. Direct scratch Rust formatting passed; that is
syntax/formatting evidence only, not typechecking or semantic validation.

The selected Registry subcommand owns one optional enum field,
`source_configuration_reference`, with the single declarative layout
`DataGroupComparison`. The layout owns its exact three-word item/operator/
reference tail and typed DataGroup family. `ResolvedInvocation` projects
actual post-head ordinals from the selected descriptor and current option
scan. The sole comparison vocabulary owner moves to Registry; the existing
iRules compatibility wrapper delegates to it. No command/report label
reconstructs the layout, and Name roles or general DataGroup side effects
never substitute for configuration-reference purpose.

The projected result carries `selection_unknown`. A Dynamic ITEM exactly at
the tail boundary remains a conditional candidate: its value might be an
option and handler acceptance is unproved. An explicit terminator or a proven
non-option ITEM settles only that layout boundary. Missing/extra tail words,
uncertain earlier options, unavailable options, missing widths, unknown
operator/reference values and expanded cardinality withdraw the projection.
The iRules owner maps the typed family through its existing object-kind table
and retains original literal/source-span/currentness guards. Its entire
object-reference API is readonly conditional advice, including unknown
selection; no object availability, frame, Native handler, dispatch, traffic
effect or deletion-safety proof is issued.

Full declarative parity is planned: SpecTcl subcommand word
`source_configuration_reference DataGroupComparison`, loader enum reader and
executed vocabulary, catalogue with exhaustive enum witness, Studio optional
enum schema/draft/native renderer/SpecTcl renderer/coverage/help/examples, and
render/load/projection equality. No native-hook abstention or GAPS entry is
proposed. The compatibility literal-string resolver and its legacy search
order/golden fixtures are untouched; other source layouts are not widened.

| Planned finite control | Expected source-advice result |
| --- | --- |
| Dynamic ordinary ITEM, known operator and literal reference | Reference ordinal retained, selection unknown |
| Explicit `--`, Dynamic ITEM | Correct shifted ordinal, option/data boundary known |
| Known available option prefix or proven non-option ITEM | Actual prefix offsets retained |
| Six admitted comparison operators | Same Registry-owned syntax vocabulary |
| Dynamic selector/operator/reference or unknown operator | No reference projection |
| Missing/extra tail, uncertain earlier option, unknown option | No reference projection |
| Expanded or opaque operand; NUL reference | No reference projection |
| Typed IRules family mapping | Existing data-group kind candidates, no attachment effect |
| Foreign/stale source, inert expression data and DTO label mutation | Existing unchanged owner controls retain their guards |
| Native/SpecTcl/Studio metadata roundtrip | Equal declaration and typed projection, omitted/invalid declarations withdraw |
| Existing failing MCP generator | Original fixture/assertions retained for later exact validation |

The new Registry and iRules controls are authored in scratch, not executed.
SpecTcl/Studio parity owners prepare separate scratch changes under root
coordination. Root must release the shared VM source freeze before any apply,
and separately lease a build target before validation. This proposal makes no
new interpreter or F5 TMM/MCPD oracle claim and changes no original receipt.
