# #940 source-owner regression plan

These are fixed source inputs and expected facts for review of the pending
source-owner change. They are not installed CI tests. The initial plan used
`f12b8afb23d8659ed286fb091058d612ba7021ef`; the observational baseline below
uses `182ea778465ebe9752e63be808201ccc52e21632`. No candidate was available.

`cases.json` records the exact authored sources, references to the already
published external-report fixtures, and their expected warning and IR facts.
`guards.json` records the ownership refusals that must accompany any restored
analysis. All original sources use the existing Tcl lexer, child parser,
registry and shared naming owners.

`phi-provenance-review.md` records the read-only S102 provenance review and
native oracle evidence. The matrix now includes six additional boundaries:
dependent branchy and within-iteration positives, independent sequential and
branchy reset negatives, sibling loops and aliased storage. The existing
fresh-sequential S102 test expectation is contradicted by the native oracles.

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
checkout's own target directory.

The authorised baseline probe compiled with `--no-run` through the shared build
limiter, then ran the emitted binary directly after releasing that lock. All
25,751 frozen files and HEAD matched before and after the run. Exact driver,
inputs, commands, raw logs, binary hash, complete compressed source manifest
and results are in `baseline-2026-11-04`.

The 27-case probe completed successfully as an observation. It made no assertion
that the known gaps are wanted behaviour. All six genuine S102 controls are
quiet. Both computed-int/list controls have Known Int producers but remain
quiet; the dict/list control warns at top level but stays quiet inside the
procedure loop despite a Known Dict producer. #2501/#2502 originals remain
quiet, as do their removed-binder controls. The #2286 original-program control
still emits zero instead of two W210 warnings.

A separate one-case input manifest uses the same binary and unchanged Rust
source: `proc uncoveredOwner {} {puts $__owner_missing}; uncoveredOwner` emits
W210 for `__owner_missing`. That control is also in the current 28-case matrix.
The false-positive absences therefore coexist with a specific context/coverage
gap; W210 is not globally disabled.

Existing authority is nonzero in this baseline. The 172 retained original
carriers include 70 compiler admissions and 37 completed Normal results; their
script plans include 23 HostScript and 149 NoRetainedObligation observations.
These are per-carrier counts, not distinct obligations. Ordinary Bytecode also
retains existing admitted typed loops, while BytecodeTraced produces 78 generic
Calls and no native compiler admissions. Future no-new-authority checks must
compare the exact originally absent child sites and preserve existing provider
plans, rather than assert global absence.

Gate status for this data-only checkpoint: JSON syntax, fixture references,
probe compilation, observational execution, source freeze and oracle checks
passed. rust-check and prep-pr remain unexecuted here; the earlier gate failures
remain recorded in `docs/LSP_RELEASE_BINDINGS.md`. This establishes no passing
candidate result.
