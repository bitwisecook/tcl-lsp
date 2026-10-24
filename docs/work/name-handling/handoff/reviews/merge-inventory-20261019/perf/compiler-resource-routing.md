# Compiler filter and resource comparison

Last updated: 2026-10-24 02:22 UTC

Status: **Blocked/routed to compiler/compiler-names for the resource gate**.
Perf source is frozen83774d43464c7d177bcd505ef22fe898ffee9a25 plus full8 batch
and temporary switch. No assertion, depth, stack size or thread setting changed.
The full authenticated102Tcl+15iRules+7synthetics and ordered PackSet proof
passes; that does not make these compiler failures green.

| Suite | new | old |
|---|---|---|
| Original8 filters +native0xff +unchanged extraReadFlow, excluding separate deep case |143pass/89fail, all232 completed|143pass/89fail, all232 completed|
| Same nondeep wall/peak RSS |684.068s /408412KiB|626.219s /408764KiB|
| Original depth1000 alone,64MiB stack, original assertion |OOM exit-9;0/1 outcomes|OOM exit-9;0/1 outcomes|
| Deep wall/measured peak RSS |437.385s /16134540KiB|449.877s /16037596KiB|

Every observed nondeep outcome is identical; zero observed pass-to-fail.
The missing deep Rust outcomes remain missing, and the strict complete-inventory
assertion fails. Neither process kill is relabelled a Rust FAILED or PASS.
The two OOM kills are reflected in memory.events, oom9->11/oom_kill8->10.
The exact private compiler ELF is removed in finally.0Rust build warnings.
Raw source/build/unit hash, lists, all assertions, outputs and final receipt:
evidence/current-83774d43-repaired-workflow-api22/. The 232-test comparison is
compiler/nondeep-comparison.json. No complete compiler gate acceptance claimed.

The ordinary extraReadFlow foreign-card test still fails BOTH at frozen837;
compiler's canonical0f4b8e19 repair is later and must be retained at final rebase.
Its test assertion is unchanged. See readflow-owner-failure.md.

The clean six-file/no-switch comparison completes all231 nondeep outcomes
BOTH: identical142pass/89fail,662.793/593.669s, peaks409624/407420KiB. Every
actual named outcome in BOTH clean6 runs matches the real switched OLD baseline;
full-only native0xff control is the extra232nd test. Zero observed P2F. Old ENV
is ignored in clean6, so this explicit cross-build comparison is required:
evidence/current-83774d43-clean6-workflow-api22/nondeep-cross-baseline-comparison.json.
Unchanged depth1000 completes its attempts ALONE BOTH:440.235/371.861s,
peaks16320104/16298828KiB, exit-9/0of1 Rust outcomes each. The strict
clean6 complete-inventory assertion FAILS; original/private ELF removed and
source seal unchanged. Across all4 full/clean6 isolated attempts, cgroup
OOM counters move9->13/8->12. Complete receipts: current-83774d43-resource-summary.json.
This is a genuine resource block on complete compiler proof, retained alongside
zero observed P2F on every actual baseline-positive result.

The empty-checked-regions draft avoids a clone only after existing complete
empty coverage. It remains uncompiled/unproven/Blocked to the owner and is
not presented as a repair for this depth1000 OOM. Investigating broader retained
body/graph preparation and traversal belongs with its semantic owner; preserve
whole input, registry, owner, source-transition effects and traversal outcomes.
