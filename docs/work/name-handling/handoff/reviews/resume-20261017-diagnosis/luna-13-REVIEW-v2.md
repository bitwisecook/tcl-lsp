# Luna-13 re-review: layer-a-v2.patch (sha256 0041fe62…cb57)

- **Baseline:** HEAD 3e9b3d049. The six `la2/a` files are byte-identical to HEAD.
- **Mechanical checks:**
  - `git apply --check` passes;
  - `rustfmt --edition 2024 --check` is clean on all six `la2/b` files;
  - the patch adds no `#[allow]`.
- **Method:** static reading, plus `nice tclsh8.6/9.0` probes in `probe/` (`v2-results.txt`, and the earlier `results.txt`).

## Verdict: HOLD (one narrow blocker, with a minimal fix)

v2 closes P1-1, P1-2, P1-3 and P1-5, and most of P1-4. One problem remains: leaving `ensemble create` out of the flag (the "tracked gap") is **unsound, and it regresses from HEAD** for the named namespace statements that v2 newly admits. Once B1 is fixed as described, I would ADMIT without another full review.

### B1 (blocker): `ensemble create` can replace a stock ensemble, and v2 then admits its members

tclsh 8.6 and 9.0 (`probe/v2-results.txt`) show four hijacks. Each rebinds `incr`, and none of them is renamed first:

| probe | what it does |
|---|---|
| `v2_c6_create_over_namespace_in_body.tcl` | `namespace eval ::fake {namespace ensemble create -command ::namespace -map {eval ::tcl::namespace::eval export ::hijack}}`, then `namespace eval ::a {proc h {} {}; namespace export h}`. Result: after=0, incr hijacked. |
| `v2_c3_create_without_rename.tcl` | The same over `::string`. `create` silently replaces the stock command. |
| `v2_c9_implicit_create_over_string.tcl` | `namespace eval ::string {…; namespace ensemble create}` (implicit name). |
| `v2_c7` | `-subcommands` form. |

The control `v2_c8_create_benign_control.tcl` (a document-named ensemble) stays stock.

**Static path for c6 under v2:**
1. The create runs inside `::fake`. `namespace_ensemble_state_transitions` (namespace_.rs:779-800) records only `Ensemble{Current, Create}`.
2. `record` (patch, command_binding.rs +12997-13007) excludes `Create`, so the flag stays false.
3. `apply_namespace_transition` only adds `::fake` to `unknown_lookup_namespaces` (command_binding.rs:19433-19438). No command definition is modelled, so the root `namespace` slot still reads as stock.
4. In the `::a` body, `namespace export h` passes `original_named_selection_is_closed`: `quiet`, geometry and the closed export all hold, and an export needs no preload.
5. `compiler_dependencies` → `original_ensemble_mapping_holds` returns `!ensemble_reconfigured`, which is true. The worker is current, and `with_late_lookup` is not flagged. The export is applied precisely, and the root `incr x` is proved stock.

**At HEAD** the same body export selected `Unknown` and poisoned globally, which was accidentally sound. So v2 creates this hole.

**Pre-existing, but not made worse in kind:** root Direct dispatch, and members on the bounded-handler or inline route (c3/c9 `string repeat`, e9). Those can stay a tracked gap.

**Minimal fix.** Pick one. Either is enough for ADMIT.
- (a) Drop `Create` from the exclusion, so that create sets `ensemble_reconfigured`. Precision suffers at the root after any ensemble create.
- (b) Preferred: add a separate monotone bit `ensemble_created`, set on `Create`. Consume it **only** in `original_named_selection_is_closed`, so the selection falls back to `select_for_facts`, which gives exactly the HEAD behaviour. Root precision is unchanged. The gate already runs on the incoming state (command_binding.rs:8775 → compiled_invocation.rs:1244/1338), so a create earlier in the same chunk is also caught.
- In both cases, add c6 and c9 to `source_body_namespace_statements_outside_the_closed_premises_stay_open`.

**Later refinement (Sol's own proposal).** Add a Registry accessor on `NamespaceTransition::Ensemble` for the affected ensemble command:
- `configure`: the command word;
- `create`: the `-command` value, or the current namespace's name.

Then scope both bits to targets that may be Registry-backed. That also recovers the precision lost to configure (see N2).

### Resolved items

- **P1-1 / P1-2 / P1-3: resolved.**
  - `closed_pattern_transfer` (onb.rs, patch +407-428) is now the single owner of the seal's envelope:
    - `quiet(state)`;
    - geometry;
    - target not in `unknown_lookup_namespaces`;
    - `expected_pattern_transfer`, which calls `import_is_fresh_and_closed` (preload slot **Missing**, fresh destination, closed origins) and `import_origins_are_live`;
    - `quiet(expected)`.
  - **Both callers use it.** The seal calls it with original operands, and the compiled gate calls it with literal `TransitionSubject` bytes through `namespace_pattern_operand_bytes(subject, None, native_entry)`. A dynamic or non-ASCII operand refuses.
  - **The seal refactor is behaviour-preserving.** It has the same checks in the same order. The extra `quiet(state)` duplicates the check at capture's top. The bytes are the same.
  - **`pattern_callbacks_are_stock` and its `apply_namespace_transition` edits are gone**, so root Direct imports are exactly as at HEAD.
- **The preload slot also covers path and unknown lookups.** `original_registry_command_paths` models namespace-path lookup (original_command_table.rs:1908-2010, `lookup_uses_namespace_path`). A root `namespace path ::p` carrying `::p::auto_import` is therefore not Missing. tclsh: `v2_n16` makes the import error, and `v2_n17` (`proc unknown`) does not fire.
- **P1-4 configure: resolved, with a precision note (N2).**
  - **The flag is set** for `Configure`, `Other` and unresolved operations. It is joined monotonically, and it is part of the `NamespaceResolutionProjection` Eq/Hash used by `ModuleCommandBindings` equality (command_binding.rs:13178, 14121).
  - **It has three consumers:**
    - `original_ensemble_mapping_holds`, which feeds `compiler_dependencies` (body named → Unknown) and the root `native_implementation_dependency_holds` → `opaque_source_invocation` (command_binding.rs:10373-10384). That covers e6b at the root.
    - `with_late_lookup`, which covers a configure earlier in the same chunk.
  - **The pre-existing gaps are correctly tracked:**
    - an `Inline` selection bypasses the check (command_binding.rs:5650-5655; e9);
    - `proves_native_implementation_lookup`'s compiled-candidate shortcut is evaluated against the chunk-entry snapshot, and `with_late_lookup` covers that.
- **P1-5: accepted.** The addendum's point 4 is withdrawn.
- **P1-6, the Registry rule: satisfied.**
  - `select_for_original_facts` (nc.rs, patch +3349-3375) keeps `select_for_facts` and its named refusal unchanged, and only the named grammar uses the original-arguments view.
  - The consumer gate supplies premises only. It names no command and builds no string.
  - **The `arguments_from` reason holds.** `select_native_words` selects through `SelectedWorker` (`from = 0` relative to `operand_from`, nc.rs:3171-3184). `select_named_target` and `ProjectedNamedArguments` take the facts convention (`arguments_from = facts.argument_offset`, which is 1 for a subcommand). Adding the grammar to `requires_original_word_preparation` would need that offset reconciled.
  - **One gap remains, and it is precision only:** the named `preparations` stay empty (compiled_invocation.rs:1300). Keep it as the recorded follow-up.
- **The gate's scope is sound.**
  - `declared()` returns None for `UnknownInvocation` (state_transition.rs:1699-1704), so members without a descriptor stay Unknown, as at HEAD.
  - Declared-empty members are admitted only when the transitions are empty. Examples: query forms of `namespace export/import/forget/path/unknown/delete`, and the read forms of `file atime/mtime`.
  - The `file` writers (`filesystem_changed`) and `path`, `unknown` and `delete` with operands hit `_ => None` and stay Unknown.
- **Refusal tests.** Each case is refused by the premise shown (or by B1's fix):

  | case | refused by |
  |---|---|
  | n8 | preload slot holds the document proc |
  | n11 | preload slot holds the stock proc (Missing required) |
  | n15 | same as n11. It does not discriminate from a plain import without the rename, but it is acceptable as a guard. |
  | n9 | command observers (`quiet`) |
  | n10 | command observers and fresh destination |
  | n12 | fresh destination: the object slot is not Missing (onb.rs:752-758) |
  | e1b | flag → mapping (body) |
  | e6b | flag → `native_implementation_dependency_holds` (root) |
  | n1 | the `::a` head is the imported proc |
  | n5 | the root `eval` is imported |
  | n6 | the `::a` `set` is imported |

  The probes reproduce in both tclsh versions. **Add c6 and c9 (B1).**
- **The five existing tests should stay stable.** v2 changes only:
  1. code inside `registered_compiler_selection`;
  2. a behaviour-preserving seal refactor;
  3. a flag that is unset in those fixtures, which contain no ensemble operations.

  Those tests pass at HEAD with `original_completed_command_world()` = Some. That is impossible had any of their body statements reached `registered_compiler_selection`, because the named `Unknown` poisons and withdraws the world. So none of their statements reach it (they run in Direct mode with an Unknown frame), and v2 cannot change their route. This resolves Sol's "residual uncertainty", given the HEAD pass. They still need to be run.
- **Compile risk: none found.**
  - `super::original_namespace_binding::literal_pattern_transfer_is_closed` is `pub(super)` in a child module, which is reachable from `compiled_invocation`.
  - `super::namespace_pattern_operand_bytes` is a private parent function, which is visible to the child.
  - The `&impl Fn(&TransitionSubject) -> Option<Vec<u8>>` closures are annotated, so they are higher-ranked.
  - `.map(operand)` works on `&F`.
  - `&Vec<u8>` coerces to `&[u8]`.
  - `self.lookup: Option<&'static …>` exists (named_invocation.rs:33).
  - The private `namespace_resolution` field is reachable from the child modules.

## Notes (non-blocking)

- **N1.** `NonBindingObservationStamp` (command_binding.rs:12945-12985, built at 14130-14161) claims to cover "every mutable observation axis", but it omits `ensemble_reconfigured`. It only drives `exact_definition_key` in the IR may-transition path (command_binding.rs:17616, 17699), so the risk is low. Add the field for completeness.
- **N2 (precision).** After any `namespace ensemble configure`, including one on a document's own ensemble, every later stock member that needs a worker becomes opaque at the root, `namespace eval` included. Sol lists the tests that may move (definition, hover, semantic tokens, diagnostics). If they flip, scope the flag to the configured command, using the refinement under B1.
- **N3.** The root Direct import, forget and inline gaps stay tracked; they are pre-existing.
- **N4.** The 404 family and `source_c_import_follows_the_original_command_object` stay withheld under the sound rule. The orchestrator must choose between changing the fixture prelude and designing a stock-`auto_import` premise. v2 correctly does not edit the fixtures.
