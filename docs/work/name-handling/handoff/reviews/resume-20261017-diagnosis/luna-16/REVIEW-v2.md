# Luna-16 re-review: Sol-17 v2

- **Reviewed:** `sol-17-admissible.patch` (sha `6ef76858…`), `sol-17-CD-v2.patch` (sha `2126a3b7…`), the v2 `NOTES.md` and `probe2.tcl`.
- **Base:** HEAD `75c75665f`.
- **Method:** static reading plus `nice tclsh8.6/9.0` probes (`probe2.tcl` and my `probe3.tcl`). Nothing was compiled or run in the repository.

## Verdicts

| Patch | Verdict |
|---|---|
| `sol-17-admissible.patch` (A + B + dead `Condition::TclSatisfies` + R) | **ADMIT** |
| `sol-17-CD-v2.patch` (C-v2 + D-v2 + tests) | **HOLD** |

CD-v2 fixes C-2, C-3 and D-1. It still leaves C-1 open on two lookup families (V2-1 and V2-2 below).

---

## 1. Admissible patch: exactly A + B + dead code + R, with the NOTES corrections

- **Hunks match v1.** The `+`/`-` lines of `sol-17-admissible.patch` are byte-identical to `superseded/parts/{A,B,R}-*.patch` concatenated (`diff` empty). There are no C or D hunks. It touches only:
  - `realm.rs`
  - `reachability.rs`
  - `package_.rs`
- **NOTES corrections are present:**
  - A-1: the analyser production callers are `handlers.rs:2257` and `commands.rs:446-449`.
  - A-2: the lowering divergence is recorded as a follow-up.
  - B-1: the 8.4 `vsatisfies` arity gap is recorded as a follow-up.
  - The R oracle across 8.4 to 9.1 is recorded.
  - The expected-flip table now says test-047, test-178 and compiler-002 still fail without C-v2, and T045 still fails without D-v2.
  - The run conditions I asked for are listed: analyser `param_traits`, list-quoted body, `registry_invocation.rs:12042/12083`, the optimiser/GVN/effect-SSA tests, `trusted_package_transfer.rs:259`, and `workspace_index.rs:11643`.

My v1 findings for A, B and R stand unchanged. Admission is conditional only on those executed selections.

---

## 2. CD-v2

### C-2: `ClauseGrammarSpec::evaluates_conditions_once_before_bodies`. Fixed.

- **Registry data only?** Yes:
  - It is a derived predicate (`clause_grammar.rs` +612-629) over the existing `selection` field and the row `timing` fields.
  - It adds no new field and changes no spec data.
  - So there is no SpecTcl loader, renderer or studio parity obligation, and no codegen or callback-inventory drift. SpecTcl already spells `selection` and per-row timing; a pack that keeps the loader's default `FirstMatch` (`tcl-spectcl/src/loader.rs:4783`) but has a loop-timed row is now excluded.
  - The consumer reads the registry and never a command name.
- **Only `if` qualifies:**
  - `if_.rs:86-104`: every row is `Selected` and selection is `FirstMatch`.
  - `try` has a `Protected` head.
  - The loops are `All` with `PerIteration` or `LoopFixture` rows.
  - The new registry test covers `if`, `while`, `for`, `foreach`, `try`, `catch` and a synthetic `FirstMatch` grammar with a `PerIteration` row.
  - `ClauseGrammarSpec` and `ClauseRow` are `Copy`, so `..*shipped("if")` and `tail.is_none_or` type-check. The test module already imports `ArgRole::{Body, Expr}`.

### C-3: math functions and parse failure. Fixed.

- `ordered` additionally requires `OriginalSourceExpression::capture(source, &context, config)` to succeed **and** `function_calls()` to be empty.
  - A parse failure, or any function call, takes the old path: the marker plus the single pre-loop clone.
  - Requiring "any function call" rather than only "non-registry" ones is correct. The `probe2.tcl` `abs` override shows that a builtin name can be a redefined `tcl::mathfunc` command.
- Ordering applies only when both predicates hold, and is computed per `Expr` role:
  - Variable reads still record the marker (traces).
  - A mixed chain is handled correctly: a later condition that is unordered clones the graph *after* the earlier conditions' threaded effects.
- Using the source parser on the Native non-source path can only reduce ordering, so it fails closed.

### D-v2: possible-occupant set. D-1 fixed; C-1 only partly closed.

**What it fixes:**

- `possible_occupants` is never a cell. `lookup_head_key`, `cells`, `logical_coordinates` and `external_header_candidates_at_selected` (`logical_navigation.rs:388-401`) are unchanged.
- So procs defined inside `namespace eval`, `if` or `catch` bodies keep their header candidates. That covers `workspace_index.rs:4613/5012/5184/6378`, `handlers.rs:10469`, `project_procedure_roles.rs:365` and `source_relation_provider.rs:745`. The new navigation test captures this. **D-1 is closed.**
- The set is merged transitively (`extend(branch.possible_occupants)`).
- The new helper short-circuits when the set is empty, so the budget is unaffected.

**Lookups that now refuse correctly.** Both go through `resolve()` or `blocks_registry_source()` (and therefore `readonly_logical_source_candidate`, which calls `blocks_registry_source`):

- A Logical or procedure-local fall-through to the registry builtin:
  - T045;
  - `viacond` (oracle: registers nothing);
  - the control case, where an unrelated `::ns::other` leaves `kept` resolvable and registering (oracle: `kept 1.0`).
- A namespace-local definition named like a builtin (`namespace eval A {proc set …}` then `set` in A). This was wrongly resolved to the builtin at HEAD and now refuses. That is correct.

**Lookups that now refuse but should not.** Only conservative cases: an occupant that was later definitely deleted (`rename`, `namespace delete`) still blocks. That is acceptable.

**Lookups that still resolve but should refuse.** These are the reasons for the HOLD.

#### V2-1 (high): the guard covers only `resolve()` and `blocks_registry_source()`

Every other selection site still uses the raw `lookup_head_key` / `resolve_source_cell` and falls through past a possible occupant to a global user cell:

| Site | Location |
|---|---|
| `resolve_source_cell` callers | `logical_procedure_invocation.rs:217`, `logical_name_reference.rs:181`, `source_procedure.rs:435` |
| `declared_source_selection` | `source_transition_advice.rs:1707` |
| `cells.get(lookup_head_key(..))` selection sites | `source_procedure.rs:219`, `source_class.rs:545`, `source_class_reference.rs:153`, `source_configured_class.rs:87`, `registered_instance.rs:380`, `source_instance.rs:249`, `source_callback.rs:514`, `source_relation_provider.rs:316/559/737`, `source_publication_proposal.rs:150` |
| Alias targets inside `resolve_source_cell_at_slot` | `alias_target_key` with `CallerNamespace` (`:1695`) |

This is a **regression against HEAD that C-v2 introduces**. At HEAD the condition marker made these selections carry `earlier_unknown_mutations` (`logical_procedure_invocation.rs:235`); under C-v2 they become definite.

`probe3.tcl` (8.6 and 9.0) shows the case:

```tcl
proc ::helper {} {return global}
namespace eval A { if {[proc helper {} {return local}; list 0]} {}; helper }
# -> local
```

Under CD-v2:
- `::A::helper` is only a possible occupant;
- Logical lookup in `A` selects the `::helper` `LogicalSourceProcedure` cell;
- `capture_logical_procedure_call` returns a call to `::helper` with `earlier_unknown_mutations=false`.

That is wrong, and it is definite: the first condition always runs.

**Fix:** add `selected_head_key(bytes)`, which is `lookup_head_key` but returns `None` when `falls_through_possible_occupant` holds. Use it:
- in `resolve_source_cell` (this subsumes the `resolve` check);
- in `declared_source_selection`;
- in `alias_target_key(CallerNamespace)`;
- in every `cells.get(lookup_head_key(..))` selection site.

Keep raw `lookup_head_key` only for coordinate and header enumeration (`external_header_candidates`, `logical_coordinates`) and for publication. Add `probe3`'s first case as a test.

#### V2-2 (medium-high): the C-source Native path is excluded

`falls_through_possible_occupant` returns `false` early for `c_source_namespace.is_some() && !native_baseline && procedure_namespace.is_none()`.

But `c_source_head_key` (`c_namespace.rs:51-86`) does fall through:
1. the local coordinate;
2. then `namespace_paths` entries;
3. then the root coordinate.

C-v2's ordered mode also applies on Native C-source graphs (`expression_commands` uses `checked_source_expression_substitutions` there). So the C-1 counterexample reopens under Native advice. `probe3.tcl` shows the shape:

```tcl
if {[namespace eval ::ns5 {proc package args {…}}; list 1]} {}
namespace eval ::ns5 {package ifneeded q5 1.0 {}}
# -> shadow, nothing registered
```

Inside the `::ns5` region, `package` resolves to the root `::package` descriptor. At HEAD the marker covered this.

**Fix, either of:**
- **(a)** Extract `c_source_head_key`'s candidate sequence into one function, and apply the same "selected reached past an occupant" test in the helper.
- **(b)** Limit `ordered` to policies whose lookups the helper covers. This is weaker.

Option (a) is preferred and also closes the pre-existing body-region gap on this path.

#### V2-3 (low): the "possible" label is too weak for the first condition

A command substitution in the *first* `if` condition, outside any lazy operator, always runs. The set nevertheless records it only as "possible". That is still conservative **if** V2-1 is fixed, because refusing is safe. It is noted because V2-1 is what makes it unsafe today.

### Tests and flips

- The four new tests and the expected flips in NOTES §2 are plausible **for the Logical survey path** (T045, test-047, test-178, compiler-002, and the out-of-capture reachability tests).
- Before admission, add tests for V2-1 (the `probe3` `helper` case: the call must be refused or carry uncertainty) and V2-2 (the C-source Native `::ns5` case).
- Run everything NOTES §2 lists, plus:
  - `logical_procedure_invocation` and `logical_name_reference`;
  - the class/instance reference tests;
  - the declared-stub selection tests.

### 180 s nested-wrapper path

Still untouched:
- C-v2 needs an `Expr` role.
- D-v2's helper returns immediately on an empty set, and the nested `[list …]` substitutions never merge.
