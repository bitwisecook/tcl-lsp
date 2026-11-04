# #940 source-owner regression plan

These are fixed source inputs and expected facts for review of the pending
source-owner change. They are not a validation result or installed CI tests.
The baseline is `f12b8afb23d8659ed286fb091058d612ba7021ef`; no candidate was
available when this plan was recorded on 4 November 2026.

`cases.json` records the exact authored sources, references to the already
published external-report fixtures, and their expected warning and IR facts.
`guards.json` records the ownership refusals that must accompany any restored
analysis. All original sources use the existing Tcl lexer, child parser,
registry and shared naming owners.

| Cases | Required result | Evidence needed before claiming success |
| --- | --- | --- |
| Conditional and unconditional accumulators, top level and named procedure | No S102 for `ys`/`pts` | Real loop/conditional bodies and their list definitions remain analysable |
| Braced and computed `fontSizes`, top level and named procedure | No S100 for `fontSizes` | Same retained original producer and iteration owner |
| Self-referential int/string loops, including conditional foreach | S102 for `x` | Actual body definitions retain both representations; a header phi alone is insufficient |
| Computed int/list and dict/list controls | S100 for the converted variable | Native-confirmed producer and list consumer remain in the original source body |
| Original #2501/#2502 reports | No reported W210 | Original binders still provide their source-owned definitions |
| #2501/#2502 local removed-binder controls | Observe whether genuine W210 returns | Previous baseline is zero; do not turn this observation into a pre-fix CI assertion |
| #2286 Tcltest original-program control | Keep the separate admission gap explicit | Previous baseline is zero instead of two; ordinary loop recovery does not establish a framework body owner |

Source recovery must preserve the full `NativeCompilationAdmission`, including
its source and provider requirement, and both `script_admission_plan` scopes.
The existing `HostScript`, `HostProcedure` and `RefuseMissingSource` obligations
cannot become `NoRetainedObligation` because analytical structure was restored.
For newly analysable child invocations whose original baseline lacks native
compiler admission or a completed Normal result, both authorities must remain
absent. Compare exact original invocation carriers, not command spellings.

The native-profile/AuthoredSimulation control first establishes the actual
`AuthoredSimulation` source purpose. It must be refused by NativeSourceSchema.
Any purpose-correct authored source projection remains subject to its own
immutable body, registry, namespace, word-vector and local-frame owners.
Bytecode and BytecodeTraced must retain their original generic dispatch where
the baseline has no admitted specialisation.

The planned full-pipeline runner uses `Analyser`, its actual resolved input and
source compilation settings, `CompilationUnit`, the shared IR walker and SSA
type facts. Select named procedure bodies by Module-issued `ProcedureId`, not a
rendered qualified name. The candidate is frozen and hashed before and after
the build/test run; all heavy builds use `/tmp/lsp-heavy-build.py` with this
checkout's own target directory. No pre-candidate compiler build was started.

Gate status for this data-only checkpoint: JSON syntax and referenced fixtures
were checked. Rust compilation, rust-check and prep-pr remain pending the main
candidate snapshot; this checkpoint establishes no new passing product result.
