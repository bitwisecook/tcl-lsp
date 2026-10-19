# Sol-35: frame-relative walk counters (findings so far, stopped on request)

I stopped on the coordinator's instruction (focus moved to the naming-migration merge; this issue is parked as a
tracked follow-up). I ran no cargo and changed nothing in the repository. I did not write any patch: the work copies
were never edited, so there is no partial patch.

Base: HEAD efdb1af7b + sol-31 depth-caps-v2.patch + deferred-depth-seed.patch. Line numbers below are in that tree.

Oracle scripts and output:

- `probes/frame.tcl`, `probes/apply.tcl`, `probes/method.tcl`
- `probes/results.txt` (tclsh 8.4–9.1), `probes/results-method.txt` (8.6–9.1)

## 1. C Tcl evidence

All results agree on every version that has the command.

| # | Probe | Result | What it shows |
|---|---|---|---|
| 1 | `if {1} { proc pp {} { package prefer latest; package prefer } }` then `list [package prefer] [pp] [package prefer]` | `stable latest latest` (8.5+) | The body is straight-line in its own frame. |
| 2 | `if {1} { proc pr {line} { set re {^foo}; regexp $re $line } }` then `pr foobar`, `pr barfoo` | `1 0` | The `set` dominates the read in the proc frame. |
| 3 | proc defined under `if` doing `rename victim {}`, then called | the command is gone after one call | |
| 4 | proc defined under `if` | its body does not run at definition | |
| 9 | TclOO method defined under `if` | not run at definition; straight-line per call | same as proc |
| 10 | `tcl::OptProc` defined under `if` | not run at definition; straight-line per call | same as proc |
| 5 | `if {0} { apply {{} {package prefer latest}} }` | `stable` | |
| 6 | `foreach i {} { apply {{} {rename victim2 {}}} }` | the command survives | |
| 7 | `foreach i {a b c} { apply {{} {incr ::n}} }` | `3` | |
| 8 | inside `apply` under `if`: `set re …; regexp $re …` | `1` | |

**Conclusion.** The coordinator's premise holds for `proc`, `tcl::OptProc` and methods. It does **not** hold for
`apply`.

- **`proc`, `tcl::OptProc`, methods:** the body runs in its own frame, later, so being under a branch or loop is relative
  to the body. Save, zero and restore is correct.
- **`apply`:** `apply` runs its body immediately, at the call site (probes 5–7). For the reachability readers (package
  records, rename, Tk, objdefine), the body inherits the call site's branch and loop context. Zeroing would contradict
  C Tcl, so it should be seeded instead.
- The only frame-scoped reader inside `apply` is constant domination (probe 8). There, zeroing is exact and inheriting
  is merely conservative (a missed fold).

**Recommended design:**

1. At the `proc` (`handlers.rs:2948`), `OptProc` (:3104) and method (`oo.rs` `walk_method_body`, :2031) sites: save,
   zero and restore the three counters. Also take `current_event` at the `OptProc` and method sites, which `proc`
   already does.
2. At `apply` (`handlers.rs` :3537): keep inheriting.
3. Add `DeferredBody::branch_depths`, recorded as `self.branch_depths()` at all four deferral sites (zero after the
   reset at the three frame sites, inherited at `apply`).
4. Have `analyse_proc_body_isolated` set it next to `body_depth` (`per_item.rs:1624`).
5. Pack it into `ItemBodyKey.body_env` as `.5`. This is a key change, needed only because of `apply`.
6. Update `item_body_analysis` (`tcl-lsp-db` :2549) and its 3 test constructors (:6618–6690). Add a key-distinctness
   assertion like the one for `deeper_key`.
7. Add a paragraph to `docs/design/rust/incremental-analysis.md` after the nesting-depth one (:126).

## 2. Per counter: semantics and divergent readers

Inside a frame body, whole-file currently inherits the counter, while per-item starts the isolated walk at 0.

### `conditional_depth` (frame-relative for proc, OptProc and method)

- **`handlers.rs:7106` `package prefer latest` → `package_prefer_latest[].conditional`.** It is not on the
  owner-fallback list (`per_item.rs:1691`), so the result diverges.
  - The consumers are `package_resolver.rs:120` (`!p.conditional && in_effect`) and `workspace_index.rs:2131`.
  - After the fix, a proc defined under `if` behaves like a top-level proc: its body raise counts, under `in_effect`'s
    existing leniency for other bodies.
  - The C Tcl-correct value is `conditional == false` (probe 1).
- **`scope.rs:1984` `nondominating_consts` → `lookup_logical_const_string_at` (:2007).** This diverges, and is
  observable through `regex_patterns`:
  - `handlers.rs:12237`: a `regexp $re` whose `$re` is a constant records two patterns.
  - Also `dict with` (:5899) and dynamic `apply $lambda` (:3188).
  - Best repro: `if {1} { proc p {line} { set re {^foo}; regexp $re $line } }`. Whole-file records no `$re` pattern;
    per-item and C Tcl (probe 2) record it.
  - Luna's `set c llength; $c a b` probably does **not** discriminate: `$c` dispatch goes through
    `pending_const_dispatches`, which CFG/SSA settles (`commands.rs:5430`), not through the lexical constant map.
- **`scope.rs:2288` interpreter-path provenance** (`conditional_depth != 0` refuses it): likely divergent for
  `set i [interp create]` inside such a body. Not traced further.
- **`handlers.rs:7002` / `:7035` package require / provide flags:** the result changes in whole-file, but per-item is
  already equal because those records force `OriginalSourceOwnership`.
  - The consumer `document_floor.rs:79` / `:183` then treats a require in a proc under `if` like one in a top-level
    proc.
- **`handlers.rs:6695` `oo::objdefine` `block_conditional`:** read only for an objdefine *inside* the body. The result
  changes; per-item is masked by the object owner fallbacks.

### `control_flow_body_depth` (frame-relative for proc, OptProc and method)

- **`handlers.rs:6275` rename.** In a proc under `while`/`if`, `rename foo {}` now becomes a straight-line deletion
  inside the body.
  - `result.destroyed_commands` is filtered for in-body offsets (`state.rs:3135`), so it is unchanged.
  - Walk-state `deleted_commands` changes, and feeds `renamed_away_builtin_names` (`scope.rs:1489`, unfiltered),
    `live_for_call` (:1111, which tolerates in-body deletions) and `var_command.rs:1370`.
  - The result now matches a top-level proc and per-item (`per_item.rs:1240` merges `walk_deleted`).
- **`handlers.rs:7003` require `control_flow` flag:** as above.
- **`tk_checks.rs:280` path uncertainty:** whole-file changes (widgets created in a proc defined under control flow are
  now certain, like a top-level proc). Per-item is equal because `TkActive` always falls back.

### `irules_debug_gate_depth` (no observable divergence)

- Its only reader is IRULE5001 (`irules_event_checks.rs:756`). That check also needs `current_event`, and the gate only
  opens with `current_event` set (:807).
- `proc` already takes `current_event` (`handlers.rs:2953`), so inside any proc body IRULE5001 can never fire on either
  path, whatever the gate depth. Luna's iRules repro does not diverge.
- The gate is a runtime branch (`if {$static::debug}`), not a load-time wrapper of definitions. iRules defines procs at
  top level only. So zeroing it is correct and changes nothing.
- `apply`, TclOO and OptProc are not on the iRules surface: `apply` is `TCL85_PLUS` and iRules behaves as 8.4
  (`f5/corpus.rs:670`). This is also why `current_event` not being taken at `apply` is unreachable.

## 3. Existing test expectations

- I statically scanned every Rust string literal in `rust/` (`tools/scan.py`, `tools/scan2.py`) and every `.tcl`
  fixture (`tools/scan3.py`) for a definer under control flow that has one of these readers inside its body. **I found
  no test whose expectation changes.** The hits are not affected:
  - top-level procs with inner `if` (`registry_invocation.rs:10314`, `compilation_unit.rs:8205`)
  - bodies with no reader (`handlers.rs:17459`, the Tk wrapper tests, `value_transfer_witnesses.rs:6818`)
  - `apply` cases, which stay inheriting (`diagnostics/tests.rs:16383`, `refactor/mod.rs:1763`)
- This is unverified by execution.

## 4. Planned tests (not written)

These would go in `per_item_corpus.rs`. Each asserts per-item == whole-file plus the C Tcl value on whole-file, so each
fails at HEAD on the value even when per-item falls back.

| Case | Expected value |
|---|---|
| `if {1} { proc p {} { package prefer latest } }` | `!conditional` |
| `if {1} { proc p {line} { set re {^foo}; regexp $re $line } }` | the `$re` pattern is recorded |
| `if {$x} { apply {{} { package prefer latest }} }` | `conditional`; discriminates per-item only if it stays on the fast path |
| `set c llength; $c a b` | equality only |

## 5. Lowering memo and `MAX_LOWER_NEST_DEPTH`: confirmed (static)

The memo lowers a reusable body starting one or more levels shallower than whole-file lowering does.

**The memoised template restarts the count:**

- `OriginalBodyLoweringTemplate::lower` (`lowering/mod.rs:974`) calls `lower_proc_body_isolated` (:6570).
- That uses a fresh `Lowerer` with `nest_depth` 0 (:1453).
- `in_procedure_frame` (:1841–1857) changes `proc_depth`, the constant map and the iRules context, but **not**
  `nest_depth`.
- `lower_body_in_irules_context` (:2547) then lowers the body at `nest_depth` 1.

**Whole-file lowering counts from the document top:**

- The module is lowered through `lower_script` (:1670 / :1718), so `nest_depth` is 1 at the top level (:2311).
- Each wrapper body adds 1 (:2547).
- So a top-level proc body is lowered at 2, and a proc under N wrappers at N+2.

**Nothing at the admission point checks depth:**

- The reuse site (:4357–4372) checks `proc_depth == 1`, not-iRules, and `can_reuse_isolated_body`
  (`command_binding.rs:7784`), which compares dispatch projections and dependencies.
- `OriginalBodyLoweringTemplate::matches` (:988) compares text, namespace, registry, config and dialect only.
- The legacy `body_cache` path (`lower_to_ir_with_body_cache`) behaves the same way.

**Result:** a body whose whole-file lowering reaches the cap gets a `Barrier` (`over_depth_script`) in whole-file
lowering, but full statements from the memo. This is off by at least one level even for a top-level proc. It is
reachable through `tcl-lsp-db` `compilation_unit_for_config` → `build_memoized_with_original_body_lowering`
(`compilation_unit.rs:3685`, `tcl-lsp-db` :2957).

**Fix options** (not drafted; unverified by execution):

- Seed the entry `nest_depth` into `lower_proc_body_isolated` and the template key.
- Or refuse reuse when the current `nest_depth` plus the body's nesting could reach the cap.
