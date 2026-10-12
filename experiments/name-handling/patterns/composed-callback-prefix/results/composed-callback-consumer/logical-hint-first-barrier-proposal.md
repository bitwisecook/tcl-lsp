# Which gate first empties the Logical type-hint output?

Status: read-only source inspection; no new execution or proposed production fix.
Actual input: `namespace eval Ω` with distinct `:pair`/`pair` procedure names,
local `x` assigned `42`/`3.5`; type hints true, parameter hints false.
The retained control passed `allows_lexical_declaration_advice()` and registry
selection, then measured `[]` at the first positive int-label assertion.

`inlay_hints_in_program` first requires the current complete image/config,
then selects the retained profile/registry and calls `source_compilation_unit`.
That shared facade repeats current-image admission and registry semantic-key
agreement before building with the actual `ResolvedAnalysisInput`/source entry.
An empty output alone does not distinguish this facade's refusal from later
absence of compiled type facts, a scope-key mismatch, or absent variable rows.

The Logical branch of `collect_type_hints` builds per-function SSA-name/type
maps, using `normalise_fn_name`; that helper delegates to `unroot_rooted_key`
(one root removal, retaining the extra colon in `Ω:::pair`). The scope walker
selects each Proc's own normalised key and never borrows an enclosing type map.
It emits only for a scoped variable row with a matching type-map name/range.

Next bounded diagnostic must record, in order: current-image/config admission;
source-unit Option and original registry-key agreement; each function's exact
name, SSA variable key and lattice/display for `x`; and analyser Proc scope
name/variable rows alongside the two constructed normalised function keys.
Stop at the first absent/refused stage. The existing source-only observation
cannot yet attribute `[]` to the constructed-key join or justify a rewrite.
Keep the original source and positive/changed-source/missing-analysis assertions.
