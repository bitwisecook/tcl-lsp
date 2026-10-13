# Shared APL variable convention

The APL presentation-global convention is a domain projection of a Tcl variable root. It supplies diagnostic metadata and proposed exports; it supplies no physical cell, namespace token, appliance execution or original read receipt. One owner in `tcl-bigip::apl` serves `AplModel::tcl_variable_names`, `apl_name_to_tcl_var`, `tcl_var_to_apl_name`, and iApp reference extraction.

## Existing owners to consume

- `tcl_syntax::naming::written_command_parts(&[u8])` owns counted `::` delimiter geometry. Its documentation explicitly grants no native purpose/token. The same delimiter geometry is used for command/variable roots; retain the returned `absolute`, `qualifiers`, and exact `simple` bytes separately.
- `tcl_syntax::naming::variable_reference_root_bytes(original_reference, actual LexerConfig)` selects a whole original variable reference's analytical root under the actual grammar, including separate/braced array forms. It supplies no cell or native read proof.
- `tcl_lexer::word_parts`/`ExecutablePartArena` own original substitution/component boundaries and selected grammar. Do not find variable references by regex over the raw entire source.
- The registered `f5-iapps` source environment is selected with `tcl_registry::model::ingress::resolve_environment("f5-iapps")`. Its analyser profile may select source grammar; this does not prove how a BIG-IP publishes presentation fields.

## Proposed domain API

Names below are proposed additions, not claims that existing methods are callable:

```rust
struct AplVariableConvention {
    original_root: String,
    absolute: bool,
    simple: String,
    apl_name: String,
}
fn apl_convention_from_variable_root(root: &str) -> Option<AplVariableConvention>;
fn apl_convention_variable_root(apl_name: &str) -> String;
fn extract_iapp_var_refs_with_config(source: &str, config: LexerConfig) -> Vec<IappVarRef>;
```

`apl_convention_from_variable_root` decomposes the exact root once using shared counted geometry. It rejects a nonempty qualifier list, an empty simple component, and a simple component without `__`. Only the exact simple component is transformed from `__` to `.`. A lone colon remains data. It retains original root spelling and absolute status independently of the domain label. The compatibility public decoder can return the owned APL label from this result.

Expected convention projections, independently of native cell lookup:

| Original root | Domain result |
| --- | --- |
| `basic__addr` | `basic.addr`, relative source geometry |
| `::basic__addr` | `basic.addr`, absolute source geometry |
| `:::basic__addr` | `basic.addr`, absolute shared delimiter geometry |
| `:basic__addr` | `:basic.addr`, leading lone-colon data retained |
| `::ns::basic__addr` | unavailable: a namespace-qualified root is not this presentation-global convention |
| `basic__addr::` | unavailable: empty simple component |
| `::plain` | unavailable: no domain separator |

Forward projection owns `.` to `__` transformation in exactly one place. `AplModel::tcl_variable_names` delegates to the public forward conversion rather than copying it. An optional checked source-export API must reproduce the intended counted root geometry independently: adding `::` before a literal leading-colon component can change its simple name. Keep convention reports distinct from usable source operands. Preserve existing public reporting behavior until the checked consumer migration is explicit; never silently drop an unrepresentable field or certify a report as a native export. This string convention is not assumed injective; retain model field rows and report collisions separately if queried. Underscore/punctuation/collision behavior belongs to the BIG-IP questions below.

## Source extraction handoff

Add a config-taking source extraction entry and update LSP/report callers to pass the selected `f5-iapps` source grammar. Preserve the public convenience wrapper by explicitly selecting that registered source environment. Feed actual whole original references to `variable_reference_root_bytes`, then the one domain mapper; retain original document spans for `IappVarRef.range` and original source spelling separately from an optional conventional report.

Use shared command/word component scanning so comments, escaped dollars and inert braced data do not become substitution reads. A Registry-selected script-body source route can admit references inside literal Tcl bodies; scanning every braced word as a script would make the same mistake as the regex. Unknown/dynamic body entry is unavailable, not proof of absence. If diagnostics still need broad potential textual references, give those a distinct MAY/report carrier and do not use them to suppress missing/unused findings as authentic reads. Keep current source and Registry context attached when crossing from a retained analysis.

Actual consumers are `tcl-lsp-server/src/lib.rs:31096,31108,31128` and `bigip-report-gen/rust/src/model.rs:153,163`. Current `IappVarRef.apl_name` determines unused-field suppression and unknown-field diagnostics in `apl/iapp_diagnostics.rs:107,146`; this is a semantic diagnostic handoff, not a cosmetic label cleanup.

## Oracle work

Existing shared written delimiter/variable scanner oracle evidence can substantiate source geometry. Add original C Tcl 8.4–9.1/latest Jim source controls only for questions their real interpreters can answer: global scalar roots with one versus repeated colon runs; a lone-colon scalar preserved in braced references; a namespace-qualified global root distinct from the simple presentation root; `${::basic__addr(index)}` versus `$::basic__addr(index)`; escaped dollars/comments/inert braced strings versus substituted words; native grammar-specific braced closing behavior. Record actual outputs before writing assertions. No tcltest is necessary for these small scripts; if used, retain the interpreter version's original tcltest.tcl/init.tcl.

BIG-IP answer documents must establish presentation field export behavior: nested APL qualification, existing underscores/double underscores, literal dots/colons or rejected characters, punctuation/unicode, table fields and array/scalar publication, export collisions, redeclaration and relative/rooted access. Ask the appliance chat to retain model/source/version, exact commands, status/stdout/stderr and observations in the same branch. C/Jim results cannot establish these domain publication rules. The implementation can centralise the current explicit convention and authentic source geometry while those appliance answers remain pending.
