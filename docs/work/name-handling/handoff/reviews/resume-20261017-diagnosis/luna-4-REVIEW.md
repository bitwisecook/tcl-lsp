# Luna-4 source review: Sol-2 fix-1 / fix-3 (fix-2 brief)

Repo `/home/user/tcl-lsp`, branch `work`. HEAD is now `4c3eb7953`, not `2a3928bc6`. The only change between them is docs and review files (`docs/work/...`, `docs/handoff/...`), and no Rust source changed. All three patches dry-run and apply cleanly to HEAD copies in
`/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/luna-4/mini/` (patched copies are kept there).
Method: static only. No cargo, no test binaries, no gdb, no tclsh.

| Patch | Verdict | One line |
|---|---|---|
| fix-1 (nextto canonicalisation) | **HOLD** | It turns test 256 green, but it resolves `nextto` names in the class's defining namespace. That contradicts tclsh and the repo's own TclOO method-body contract. |
| fix-3 (coverage over unknown-role precedence) | **HOLD** | By construction it flips at least five passing-by-design assertions that expect "roles are unavailable" (namespace_rename.rs:686, :773; rename_safety.rs:2065, :2081, :2122). |
| fix-2 (DB921 duplicate installed body) | not reviewed for admission | Awaits the user's identity-policy decision. The mechanics look sound, with notes below. |

---

## fix-1: nextto / next class canonicalisation

### Diagnosis re-derived (agrees with Sol-2)
- Failing selector 256 is `definition::tests::nextto_picks_occurrence_at_cursor_not_first_on_line` (result.json run 256, exit 101). The panic is at `definition.rs:7097`, where `locs.len()` is 0 instead of 1.
- `next_dispatch_target` (definition.rs:1287-1319) calls `canonicalise_class(analysis, cursor, &target)?` (definition.rs:1307). That calls `class_hierarchy::source_namespace_at(&analysis.global_scope, at)?` (definition.rs:1331-1332; class_hierarchy.rs:877-893).
- The cursor sits inside a TclOO method body. That scope's `naming_scope` is deliberately `None`, because the object frame/namespace is unknown (types.rs:2196-2205; interp_visibility.rs:1184/1221 key off `Method && naming_scope.is_none()`). So `?` returns None, and go-to-definition returns `[]`.
- The same defect is in `references.rs:2356-2363` (`canonicalise_class_name`), used at 2409 and 2450 for constructor/destructor `nextto` references.

### HEAD API signatures (exact)
- `class_hierarchy.rs:877` `pub fn source_namespace_at(scope: &super::types::Scope, offset: u32) -> Option<&SignatureNamespaceScope>`
- `class_hierarchy.rs:849` `pub fn resolve_written_class_name_in_scope<S: BuildHasher>(name: &str, namespace: &SignatureNamespaceScope, classes: &HashMap<String, ClassDef, S>) -> Option<String>`
- `class_hierarchy.rs:898` `pub fn resolve_written_class_name<S: BuildHasher>(name: &str, classes: &HashMap<String, ClassDef, S>) -> Option<String>`. This is the unpositioned root lookup.
- `class_hierarchy.rs:805` `pub fn resolve_class_lookup(...)`. It withdraws on `source_name_ambiguous`.
- `signature_scan/scope.rs:460` `pub(crate) fn body_scope(&self) -> Option<SignatureNamespaceScope>` on `SignatureSourceCommand`. For the C recipe it returns `C(self.slot.namespace)`, the namespace that holds the class command slot.
- `analyser/types.rs:1455/1457` `ClassDef.source_name: Option<SignatureSourceCommand>`, `ClassDef.source_name_ambiguous: SourceNameAmbiguity`. `is_observed(self)` is a const fn (types.rs:1422).
- `analyser/scope.rs:750` `pub fn innermost_scope_reaches_oo_helpers(root: &Scope, byte_offset: u32) -> bool`
- `definition.rs:1330` `fn canonicalise_class(analysis: &AnalysisResult, at: u32, name: &str) -> Option<String>`. The patch changes this to `owner: &str`.
- `references.rs:2356` `fn canonicalise_class_name(analysis: &AnalysisResult, at: u32, name: &str) -> Option<String>`. The patch changes this to `owner: &str`.
- New in the patch: `class_hierarchy.rs:899` (patched) `pub fn class_declaration_namespace(class: &ClassDef) -> Option<SignatureNamespaceScope>`.

### Real Tcl semantics of the `nextto` class operand
- 8.6.18 `tclOOBasic.c:827ff` `TclOONextToObjCmd` calls `Tcl_GetObjectFromObj(interp, objv[1])`. That calls `Tcl_GetCommandFromObj` (tclOO.c `Tcl_GetObjectFromObj`), which resolves in the **current frame namespace**.
- 9.0.4 behaves the same (tclOOBasic.c:1390). 9.1.0 uses `TclOOGetClassFromObj`, and the bytecode path (`tclExecute.c:4960`) also uses `Tcl_GetObjectFromObj`.
- The frame namespace of a procedure-like method is the **object's instance namespace** (`contextPtr->oPtr->namespacePtr`, tclOOMethod.c). The only exception is `USE_DECLARER_NS`, which is C-API only and even then is the class *object's* namespace (`::oo::ObjN`), never the namespace that holds the class name.
- The object namespace path is `::oo::Helpers` only (tclOO.c:699 for 8.6, :941 for 9.0). Lookup order is therefore object ns → `::oo::Helpers` → `::`.
- So with only `::Ns::A` defined, `nextto A` inside a method of `::Ns::C` raises "A does not refer to an object". The class's defining namespace is never searched.

### Repo contract comparison
- `analyser/types.rs:2196-2205` (`Scope::oo_global_resolution`) says the class's defining namespace is NEVER searched from a TclOO method body, pinned against tclsh 8.6.16 and 9.0.4. The documented static approximation is global-only.
- `analyser/scope.rs:131-133, 160-166` (`advance_command_resolution_namespace`): a TclOO method resets to `::`. Only snit/itcl methods use the defining-namespace rule.
- `docs/design/analysis/name-resolution.md:393-396`: "TclOO reporting bodies use a global approximation (`Scope::oo_global_resolution`) when the actual object namespace is unavailable".
- `docs/design/contracts/naming-consumer-matrix.md:312`: class source queries "withdraw rendered collisions or missing contexts".
- **Finding F1-1 (blocking):** `class_declaration_namespace` hands a TclOO member body the class publication slot's namespace as its lookup context. That is the snit/itcl defining-namespace rule applied to TclOO. It contradicts all four places above and real C.
  - It also substitutes a context the matrix says must be withdrawn.
  - The new doc comment (patched class_hierarchy.rs:895-897) calls this namespace "the owner context for a member body's written next/nextto class operand". That claim is false for C.
  - The patch matches only the legacy in-code prose: definition.rs:1321-1328 "owner-aware", and the comment in test `jump_to_nextto_namespaced_class_method` at definition.rs:7075-7088. That test asserts `nextto A` → `::Ns::A`, which tclsh rejects.
- **F1-2:** the namespace also depends on where the class command was published, not where it was written. `oo::class create ::Other::C` written inside `namespace eval Ns` yields `::Other`. Neither is C behaviour.

### Ambiguity, identity, rules
- Ambiguity: the patch refuses when the owner `source_name_ambiguous` is observed, or when `source_name` or `body_scope` is missing. The target lookup still goes through `resolve_class_lookup`, which also withdraws collisions. Ambiguity handling is fine.
- Native/object-frame identity: none is granted. The fn returns source geometry only, and the provider walk is unchanged (`member_next_provider`). The violation is a lookup *context* grant, not a Native identity grant.
- Contract row:
  - The `xtask owner-resolution` gate (rust/xtask/src/owner_resolution.rs:1394-1406) checks only `resolve_written_class_name_in_context`, `resolve_class_lookup(&lookup, classes)` and `source_name_ambiguous` in class_hierarchy.rs. The patch keeps all three.
  - The existing pub lookups (`source_namespace_at`, `resolve_written_class_name_in_scope`) have no row in `shared-utility-contracts-rust.md`, so a row is not required by gate or precedent.
  - If the fn were admitted, though, `naming-consumer-matrix.md:312` and the TclOO method-body statements above would need amending, because the fn introduces a new class-query context purpose.
- Compile risk: low. All types are in scope (ClassDef is imported at class_hierarchy.rs:36), `body_scope` is pub(crate) in the same crate, `&String` coerces to `&str` for `class_q`/`other_q`, and `span` is still used in references.rs. Sol-2 reports it as measured/compiled.
- Rule compliance:
  - No assertion, selector, fixture or timeout change. No `#[allow]`. No hardcoded command names. Comment density matches the surrounding code.
  - The references.rs:2348-2355 doc comment is left stale ("registry's own `resolve_class_name`", "falls back to the written name"). That is minor and pre-existing, but the patch rewrites the signature under it.

### pass/fail effect
- Fixes: 256 (global classes); non-selected `jump_to_nextto_named_class_method`; `original_next_definition_uses_captured_target_and_selected_availability`; and very likely `references::tests::constructor_next_chain_reference_via_{nextto_explicit,braced_nextto}_target`. All of these use global classes.
- `jump_to_nextto_namespaced_class_method` passes only because of the non-C rule.
- I found no pass→fail risk inside the selected 664.

### Recommended C-aligned alternative (not a patch here)
- In `canonicalise_class` / `canonicalise_class_name`: when `scope::innermost_scope_reaches_oo_helpers(&analysis.global_scope, at)` is true, resolve with the existing `class_hierarchy::resolve_written_class_name(name, &analysis.all_classes)` root lookup. Otherwise keep `source_namespace_at` + `resolve_written_class_name_in_scope`. The root lookup is the documented reporting approximation.
- This needs no new pub fn and no contract change.
- It turns 256, `jump_to_nextto_named_class_method`, `original_next_definition_...` and the constructor-reference tests green.
- `jump_to_nextto_namespaced_class_method` would stay red. That test encodes behaviour tclsh rejects, so changing it needs a **user decision**: either amend the assertion, or explicitly adopt a non-C reporting rule.

---

## fix-3: coverage-incomplete precedence over unknown-role hazards

### Diagnosis re-derived
- 466 `namespace_rename::potential_source_coverage_tests::namespace_hazard_negative_requires_complete_original_source_coverage` panics at namespace_rename.rs:1490. For `set value 1; if {1} $body`, the refusal is the UnknownRole text instead of "potential source coverage is unavailable".
- 656 `rename_safety::checked_rename_hazard_census_tests::missing_body_and_parse_phases_cannot_be_a_negative_hazard_census` fails at rename_safety.rs:2421 for `if {1} $body`.
- Cause: `if {1} $body` gets `roles() == None`. `conditional_roles` (registry_invocation/source_structure.rs:4364-4380) returns None when `authored_source_argument_roles()` is incomplete. My guess, unverified, is the `?then?` noise slot: a computed word could be `then`.
- The visitors then record UnknownRole (namespace_rename.rs:522-527; rename_safety.rs:1011-1013), and `hazard.is_some()` skips the coverage refusal (namespace_rename.rs:571; rename_safety.rs:1032).

### Key structural fact (F3-1, blocking)
UnknownRole always comes with incomplete coverage. Both visitors set UnknownRole exactly when `source_registry_words` is None, or `roles()` is None, or there is no source schema. In the checked syntax census (`SourceStructure::census_command_bodies`, tcl-lsp-core/src/source_structure.rs:1582-1648), each of those cases pushes to `unavailable` or `syntax_unavailable`:
- words None → `syntax_unavailable` or `unavailable` (1594-1607).
- roles None → `source_script_bodies_for_coverage` returns at `words.roles()?` (tcl-compiler/src/registry_invocation/source_scoped_body.rs:1766) → `unavailable` (1641-1646).
- No schema → `with_source_schema(...)?` (source_scoped_body.rs:1784) → `unavailable`.

The census runs on every command of every region under `CapturePurpose::CheckedSyntax` (source_structure.rs:1107, 1257-1258, 1496-1498). So `applicability` is Incomplete (source_structure.rs:389-405), and `visit_checked_analysis_executable_commands` returns Incomplete (executable_regions.rs:203-212).

Result: with fix-3, every top-level UnknownRole hazard is reported as the coverage refusal. The roles message becomes practically unreachable.

### Every test asserting the unknown-role message (repo-wide grep)

| Test (fn line) | Assertion | Case | Outcome with fix-3 |
|---|---|---|---|
| namespace_rename.rs:640 `namespace_hazard_keeps_original_roles_and_unavailable_owner_refusal` | :681-687 `.contains("argument roles are unavailable")` | custom `source_namespace_reference` (dict surface) under tcl8.4 → unavailable → roles None | **pass→fail** |
| namespace_rename.rs:706 `supplied_namespace_hazard_patterns_keep_parentheses_and_unknown_roles` | :762-774 same text | same spec under tcl8.4, `::other::$n` | **pass→fail** |
| rename_safety.rs:2044 `original_variable_hazards_keep_selected_alias_origins_and_unknown_roles` | :2061-2066 `.contains("roles are unavailable")` | `source_var` under tcl8.4, ×2 sources (direct, interp alias) | **pass→fail** |
| same fn | :2077-2082 | `proc source_var args {}` shadow under tcl8.6 (roles/schema None) | **pass→fail** (high confidence; same census path) |
| rename_safety.rs:2086 `supplied_variable_hazard_patterns_keep_qualified_and_open_roots_possible` | :2118-2123 | `source_var` under tcl8.4 | **pass→fail** |

- HEAD status of these five is unmeasured. They are outside the 664 selection, and no capture in the scratchpad lists them.
- The "pass→fail" label means a regression relative to their designed HEAD behaviour. I could not execute it under the constraints, but the census derivation above makes it near-certain.
- The other assertions in those tests are unaffected: tcl8.6 "computed at run time", "metadata are unavailable", and `.is_some()`.

### Selected tests named in the brief
- **461** `executable_regions::checked_potential_source_tests::checked_potential_source_keeps_positives_with_unknown_body_and_parse_debt` (passed): its file is not touched, so unaffected.
- **657** `positive_dispatch_hazard_survives_incomplete_source_inventory` (passed): it asserts only the `method_rename_hazard` reason "class is not tracked", and that function is not patched. Unaffected.
- 655, 658, 659, 660 do not exercise the UnknownRole + Complete path. Unaffected.
- 466 and 656 become green: `if {1} $body` → coverage message. `apply $lambda` → LambdaLiteral under the Syntax purpose → Incomplete → coverage message either way.

### Secondary findings
- **F3-2:** `namespace_rename` stops updating `complete_dispatch` after the first hazard (closure early return at :491-493). Under fix-3 the precedence decision reads a truncated dispatch census. It is still a refusal, so this is not unsafe, but the message choice becomes order-dependent.
- **F3-3:** the coverage refusal carries no span, while the UnknownRole refusal carries `Some(span)`. Editor error placement changes.
- **Direction:** the two test groups conflict only under the blanket flip. A principled fix would stop raising UnknownRole when every *possible* role at that position excludes the name role. For `if`, the slot is either `then` noise or Body, and neither is NamespaceName/VarRead/VarWrite, so that case falls through to the coverage refusal. This needs registry support for possible-role sets; I have not checked whether that exists.
  - The alternative is a user decision on message precedence, which would change one of the two test groups.

### Compile and rules
- Compile risk: low. `Option::is_none_or` is already used in tcl-lsp-core (references.rs:588), and the closure patterns bind by reference correctly.
- No assertion, selector, fixture or timeout change. No `#[allow]`. No command names. Comments are 2 lines each, matching the surrounding code. No absence-as-completeness: the result is still a refusal.

---

## fix-2 (brief; pending the user's policy decision)
- Mechanics:
  - It mirrors `already_named`: it skips an installed First-incarnation body whose allocation site, namespace and joined params equal an `original_authored_declaration_body_units` body unit's `declaration_site` / `namespace_context` / `params_raw`.
  - `declaration_site` is `Option<CommandAllocationSite>` (ir.rs:2494), so the comparison type-checks.
- Notes:
  1. Unlike `already_named`, it does not check `installed.allocation.site.source == installed.source.origin` or the executed source equality. Two different executed sources at the same site/namespace/params would be merged.
  2. It also applies to the chained `declaration_bodies` (`original_declaration == true`) entries, not only installed ones.
  3. The comment's claim that the authored readonly declaration "is the readonly owner of the same first-incarnation allocation" is exactly the policy point. It asserts identity between an authored-readonly body and an installed body, which this migration otherwise keeps distinct ("do not drop parallel body identities", current-state.md:44).
- Opinion: do not admit until the user rules. If identity is accepted, add the source-origin equality check that `already_named` has.

## Evidence paths
- Failing logs: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/cp09/partial-lsp35r2/original-tcl_lsp_core/test-256.log`, `test-466.log`, `test-656.log`; outcomes in `result.json` there.
- Tcl sources: `/home/user/tcl-lsp/tmp/tcl8.6.18/generic/{tclOOBasic.c,tclOO.c,tclOOMethod.c}`, `/home/user/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, `/home/user/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`.
- Patched mini copies: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/luna-4/mini/rust/...`
