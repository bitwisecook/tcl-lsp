# Incoming external-global seed audit

The sealed baseline taint solver retains a genuine root-global read while losing
the independent provenance of its declared external-input seed. This audit
does not reproduce #2243 or establish a missing T100: unknown represented
contents independently produce T100 in these cases.

The [public probe](probe.rs) captures a genuine `NativeCompilationEntry` through
the public VM/`CompileService` boundary and keeps its owner alive. It uses
`CompilationUnit::build_with_analysis_input`, the ordinary interprocedural
solver and `original_source_read_at_unit`. It constructs no SSA symbols or
Native entry rows and calls no private compiler helper. The VM capture attests
command and namespace registration; it neither executes the analysed source
nor asserts a bound CLI variable or its incoming value.

| Fixed source case | Exact read and incoming seed |
| --- | --- |
| Top-level `expr {$argv + 1}` | Original read retained, same-entry native root, represented version unavailable; incoming 0 is `OtherTaint`. |
| `proc writer {} {set ::state 1}; expr {$argv + 1}; writer` | Original read retained at 38–43, same-entry native root, represented version unavailable; incoming 0 is incorrectly `ConservativeGlobalSeedOnly`. The writer's conservative seed prevented the independent external seed from joining. |
| Top-level `$env(PATH)` plus reachable writer | Original element read retained with its root identity; incoming 0 is likewise conservative-only. |
| Qualified-only top-level `$::argv` | Original read and same-entry native root retained; incoming 0 has no seed. Baseline startup seeding searches bare display names. |
| Root alias `upvar #0 argv argc` | Original `$argc` read retains the native-root `argv` cell, but incoming 0 has no seed. The same display alias to `box::argv` retains a non-root cell and must refuse incoming root provenance. |
| `gets` into `argc` | Original native-root read retained with represented version 1 and `OtherTaint`; T100 emits. |
| Unset/rebind `argv` through `gets` | Original native-root read retained with represented version 2 and `OtherTaint`; T100 emits. |
| Procedure/global, local-formal and named-namespace candidates | Exact original-read selector declines. Their emitted diagnostics do not satisfy that coverage prerequisite; they cannot establish the intended physical-cell regression. |

[Initial rows](rows.json), [top-level rows](top-level-rows.json),
[alias/rebind guards](guard-rows.json) and the
[exact IMAP4 row](exact-imap4-row.json) retain source, argv, actual process
status and streams. Each row is a fresh process. All 21 processes exited 0.
The [four baseline properties](baseline-properties.json) fail despite each
having an exact original read and the matching native-root cell.
Both exact IMAP4 `argc` reads remain unavailable, with zero procedures and one
function, even with the independently captured native command entry.

The [link receipt](link-receipt.json) records the exact source closure and all
138 dependency rlib hashes before and after the short `rustc` link. They agree.
This used the complete matching dependency set from source
`2a413918de001e01418e67d31b4e8b28470c4550` plus tracked patch
`7a5ddc68c6bef35a00bbad8b2d665e773a5511dd6ca43450b2a973988babf973`.
The compiler rlib SHA-256 is
`9c61225962f50fc60cafb24c8fd6ef10b92c7033005d290fcff5570ac8a2a91a`.
No Cargo build or compiler change was needed for these observations.

Baseline `seed_entry_taints` inserts conservative global-write taint first,
then uses `or_insert` for registry-declared external-input taint. Its incoming
lookup uses source names rather than retained physical cells. Any repair must
use the shared variable owner, preserve element identity and independently
justified input provenance, and refuse local, foreign, unknown or recreated
lifetimes. The diagnostic fallback for unknown contents stays independent.
The capture describes the baseline. The separate [repair receipt](../seed-repair/README.md)
records the shared cell projection, provenance join and passing fixed controls.
Neither result establishes the cause of #2243.
