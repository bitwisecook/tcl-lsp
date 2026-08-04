# Frame-effect summaries: caller-frame and global-frame callee effects

A Tcl procedure routinely writes variables that belong to *someone else's*
frame: `upvar` aliases a caller-frame (or global) name into a local, and
`uplevel` runs a whole script in the frame its level word selects. None of
that shows up in the caller's own text, so a single-function analysis would
fold, forward, or delete values a callee really rewrites.

Two per-procedure summaries model this, both computed once per module in
`rust/tcl-compiler/src/cfg_builder/` and consulted at every call site by
`CfgBuilder::apply_upvar_invalidation`:

| Summary | Module | Owns |
|---|---|---|
| `UpvarInfo` | `cfg_builder/upvar_info.rs` | effects on the **immediate caller's** frame (`upvar 1`, `uplevel 1`) |
| `GlobalWriteInfo` | `cfg_builder/global_write_info.rs` | effects on **global/namespace** cells (`global`, `variable`, `upvar #0`, `uplevel #0`) |

The frame table (pinned on tclsh 9.0.4 / 8.6.14, see `upvar_info.rs`'s module
doc for transcripts):

| written in the callee | frame written | which summary |
|---|---|---|
| `upvar 1 x y` / `upvar x y` | the caller | `UpvarInfo` named binding |
| `upvar #0 g l` / `uplevel #0 {…}` | the global frame | `GlobalWriteInfo` |
| `upvar 0 x y` / `uplevel 0 {…}` | the callee's own frame | neither (no cross-frame effect) |
| `upvar 2 …` / `upvar $lvl …` / `uplevel 2 …` | further out / unknown | `UpvarInfo` opaque widening |

## The lattice, per name

Each summary answers "which names does a call to this procedure touch, in
which frame?" at one of three precision levels, and every consumer must
respect the ordering — a level may only ever be *widened*:

1. **Named** — the effect is a known literal name (`literal_targets`,
   `uplevel_literal_writes`, `GlobalWriteInfo::names`) or a name resolvable
   from the call site's own arguments (`param_targets`,
   `uplevel_param_writes`, `args_tail_upvar`). The call site widens exactly
   those names (merged into the call's `defs`, so SCCP/O102 see a fresh
   definition).
2. **Opaque frame** — the effect is real but no name is enumerable
   (`has_unresolvable_caller_target`, `caller_frame_opaque_writes/reads`,
   `GlobalWriteInfo::opaque_global_frame`). The call site widens with a
   `Statement::Barrier`, which SCCP, O102 load forwarding, O109/DSE, branch
   folding, and the I230 existence fold all already treat as "anything may
   have changed".
3. **Empty** — no cross-frame effect; the call site is left alone.

The abstention direction is fixed: a shape the analysis cannot read must land
at level 2, never silently at level 3. Three shapes used to fall through and
are now covered:

- `upvar 1 x $dst` — a dynamic **local** side (issue #1165). The caller-side
  name is still exactly `x`, so it lands at level 1 in the keyless
  `uplevel_literal_writes` bucket (a `$param` source resolves through
  `uplevel_param_writes`; anything else widens to level 2).
- `uplevel #0 …` in a called procedure (issue #1198). A literal or
  `[list set g …]`-constructed script contributes level-1 global names; a
  dynamic script (`uplevel #0 $body`) sets `opaque_global_frame`, which is
  transitive over the direct-call closure and becomes a barrier at every
  call site — including calls embedded in command substitutions and in
  `if`/`while`/frozen-loop conditions. This is what fixed the documented
  O102 miscompile (`proc setter {} {uplevel #0 {set x 99}}; set x 5; setter;
  puts $x` must print 99, not 5).
- a dynamic write target inside a readable script body (`uplevel 1 {set $n
  1}`, `uplevel #0 {set $n 1}`): `ssa::defs_of` drops such a target
  entirely, so both body scans widen explicitly
  (`script_has_dynamic_write_target`).

## Method dispatch (`my` / `next`)

A callee reached through `my`/`next` is a method, never in the upvar-procs
table — the dispatch does not name its target. When the module's
method-dispatch evidence is incomplete
(`upvar_info::module_method_dispatch_evidence_is_incomplete`: any method body
that can reach its caller's frame, or any redefined method), the method-body
CFGs are built with synthetic level-2 entries for every registry command
carrying `TCLOO_SELF_DISPATCH` / `TCLOO_NEXT_CHAIN` (issue #1177). The same
evidence rule gates the optimiser's method-body constant propagation (issue
#1097), so the two consumers cannot drift. `self` (`TCLOO_INTROSPECTION`)
dispatches nothing and is excluded.

## Conservative limits (all widen, none miscompile)

- **Renames and aliases.** The summaries key on the command name as written;
  a proc reached through `rename`/`interp alias`/a computed name is not
  found, and the callee-side effect is missed. The dynamic-name barrier and
  `command_binding` mutation tracking cover parts of this independently.
- **`next` chain targeting.** A chained implementation's `upvar 1` skips the
  calling implementation's frame entirely (tclsh 9.0.4: it lands in the
  frame of whoever invoked the whole method). Widening the dispatch site is
  sound for the calling body but does not model the one-frame-out effect;
  per-chain precision needs the MRO work of issue #1164.
- **Cross-file callees.** `detect_upvar_procs` and
  `detect_global_write_procs` are single-`Module`; a callee defined in
  another file contributes nothing (issue #1139's idx-59 residual).
- **Absolute non-global levels** (`uplevel #2`) widen the calling function
  through the caller-frame opaque path rather than being mapped to a
  specific frame.
- **Relative levels beyond the caller** (`upvar 2` in a callee of a callee)
  widen only the direct caller's function; the effect is not composed
  transitively through the call graph, so the grandparent frame relies on
  the direct caller's own barrier.

## Consumers

- `CfgBuilder::apply_upvar_invalidation` — per-call-site defs widening +
  barriers (direct calls, embedded `[…]` substitutions, condition
  substitutions, `return [callee …]`).
- `Function::caller_frame_barrier` / `alias_observed_vars` — whole-function
  blindness and "a callee may observe this store" facts for O109/O126.
- The LSP navigation half lives in `rust/tcl-lsp-core/src/caller_frame.rs`
  (per-parameter and literal caller-frame bindings on
  `ProcDef::caller_frame_params` / `caller_frame_literals`, issue #1139) —
  same frame table, independent computation from source text in the
  analyser (`analyser/param_traits.rs`).
