# Luna-24 review: Sol-17 i43 fix (`body_effects.rs`)

**Verdict: ADMIT-WITH-CONDITIONS**

## What was reviewed

- The patch is `sol-17/i43-fix.patch`, sha256 `10cbc258…bbd8`. It is byte-identical to `git diff` at HEAD `6f01dc10c`.
- The review was static only. No cargo, no repo changes, no worktrees.
- Oracle probes were run on tclsh 8.4, 8.5, 8.6, 9.0 and 9.1:
  - `luna-24/probe-q5-84.tcl`: Sol-17's three sources verbatim. Membership is tested with `lsearch`, because `in` does not exist in 8.4. Sol-17's original probe errors under 8.4.
  - `luna-24/probe-q5-portable.tcl`: elseif, else, erroring-condition, `while` and `for` controls.

## Fix (1): conditions walked before body catalogues — sound

### It matches the existing pattern

- Inside a body walk, `BodyWalk::command_inner` (`body_effects.rs` ~1349) already does `expressions(graph)`, then `graph.widen` if that is incomplete, then `regions(graph)`.
- The top-level main path `apply_immediate_body_effects` uses the same order.
- The fix brings the top-level body catalogue (`retain_original_body_catalogue`) into line with both.

### Only possible effects escape the condition walk

- Each condition command walks in a branch, which is merged by `retain_possible_effects`. That merge only adds:
  - Shadowed cells;
  - `possible_occupants`;
  - unknown paths and exports;
  - debts and uncertain operations.
- So a condition effect can only make a body lookup more conservative.
- This holds in the `if`-ordered (threaded) mode too.
- The walk's `authored_prefixes` tape is thrown away, so the conditions add no extra inventory.
- `observe_publication_proposal` can only set its admitted flag to false (monotone).
- `unavailable_lifecycle_effects` cannot be set here, because `inventory` is `None`.

### No leak between sibling arms or into later commands

- `after_conditions` is a clone.
- Each body is walked from its own clone of it, so one arm's effects never reach another arm.
- The caller's `graph` is untouched.

### Widening on an incomplete walk fails closed

- Widening is the house operator for unknown effects.
- Catalogue advice already carried `UnknownEarlierMutation` through `record_uncertainty`, so no obligation is dropped.

### Order against C Tcl

| Probe | tclsh 8.4–9.1 | Model after the fix |
|---|---|---|
| T1: shadow created in the `if` condition, body registers | not registered | refused (the fix) |
| T2: shadow created in an earlier `if` condition, top-level registration | not registered | refused (existing CD-v4) |
| T3: positive control | registered | advertised |
| E1: shadow in the elseif condition, elseif body registers | not registered | refused |
| E3: shadow in the elseif condition, else body registers | not registered | refused |
| X1: condition errors after creating the shadow | not registered (body never runs) | refused |
| X2: as X1, registration at top level after a `catch` | not registered | refused |
| W1: `while` condition creates the shadow | not registered | refused |
| E2: arm-1 body runs **before** the arm-2 condition that creates the shadow | registered | refused |

E2 is over-refusal. Every condition's effects are applied to every body. It loses precision but stays sound: it is only a missing positive discovery. It cannot become absence-as-completeness, because the survey walker already sets `complete = false` for any non-constant Expr guard (`reachability/original.rs` ~297-306). That is also why a refusal here can never produce a `Complete` disposition.

### Budget: not on the critical path

`parse_pkg_index_survives_deeply_nested_source_wrapper` is the 180 s path.
- `ifneeded` has no `Expr` role, so the extra cost is one `AdviceGraph` clone plus one `authored_roles` call.
- Elsewhere the fix adds one condition walk per Body-bearing invocation. That is a constant factor: the conditions are already walked 3× (immediate effects, expression catalogue, native expression syntax), and the walk is bounded by `MAX_COMMANDS` / `MAX_DEPTH`.

## Fix (2): test rewrite — acceptable, with a coverage caveat

### Provenance

- `git log -S` shows exactly one commit: `21a199ad8` ("Integration43 candidate … not built; not admitted").
- The test is Sol-17's own CD-v4 V2-2 test, not an original-handoff test. Rewriting it does not breach the rule against changing original assertions.
- The selector name is kept.

### Why the rewrite was needed

- The old shadow assertion was vacuous.
- The old positive control failed at HEAD (`sta-base`), which confirms the Native path issues no definite advice.

### The new test

- All three assertions agree with tclsh 8.4–9.1: 0 / 0 / 1.
- Assertion 2 exercises the fix directly.

### The caveat

- The name `c_source_…` no longer describes what the test checks.
- More importantly, V2-2 (the refusal in `c_source_head_key` / `c_source_head_candidates` for Native C-source) now has **no** test at all. Before the rewrite it had only a failing one.

## Conditions

1. **C1.** Record V2-2 (Native C-source fall-through past a possible occupant) as an explicit, uncovered obligation in the tracking documents. This test must not be counted as C-source coverage. Preferably add a graph-level `c_source_head_key` test, like the V3-1 one.
2. **C2.** Record these as tracked follow-ups. Both are pre-existing and neither is a regression from this patch:
   - E2 precision: give each arm only the conditions before it.
   - Order of earlier Body-role scripts:
     - In `for`, the start script runs before the condition and body. F1: tclsh registers nothing, but the body catalogue does not see the start script's shadow.
     - Loop-carried body effects.
3. **C3.** Run broad gates before publishing: `make rust-check` (clippy), then `prep-pr`. Capture the time of the nested-wrapper test.

## Optional, non-blocking

- Skip the clone and the walk when the schema has no `Expr` role, or when no body matches. This needs `advice` cloned before it is moved into `source_transition_words_from_advice`.
- `widen` drops Descriptor cells with non-empty lineage. In the incomplete-condition case only, this can re-expose a registry schema for a renamed builtin.
  - This is inherited from the house operator and has the same semantics as the main path.
  - The advice keeps `UnknownEarlierMutation`.
  - Add a regression test if anyone touches `widen`.
- The new comment says a body is reached "only after the invocation's own conditions ran". That is precise for `if` and `while`. For `for` and other grammars it is an over-approximation, so it could be worded as such.
