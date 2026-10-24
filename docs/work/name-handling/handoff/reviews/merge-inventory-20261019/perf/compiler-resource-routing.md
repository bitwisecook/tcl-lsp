# Compiler filter and resource comparison

Last updated: 2026-10-24 01:47 UTC

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

The clean six-file/no-switch compiler comparison runs next with the same inputs
and unchanged depth1000 alone BOTH. Old ENV is ignored there. Its results are
pending; do not infer them from the switched full8 build.

The empty-checked-regions draft avoids a clone only after existing complete
empty coverage. It remains uncompiled/unproven/Blocked to the owner and is
not presented as a repair for this depth1000 OOM. Investigating broader retained
body/graph preparation and traversal belongs with its semantic owner; preserve
whole input, registry, owner, source-transition effects and traversal outcomes.
