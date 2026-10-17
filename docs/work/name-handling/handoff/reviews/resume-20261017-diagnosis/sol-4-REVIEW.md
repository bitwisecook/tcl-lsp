# Sol-4 review: nextto-c-faithful.patch

Candidate: `scratchpad/i37/nextto/nextto-c-faithful.patch` (base `i37/nextto/a` is byte-identical to HEAD `4c3eb7953` and the worktree for both files).
Read-only review. No cargo, tests or gdb were run. The Tcl probes and their outputs are in `agents/sol-4/probes/p1..p8.{tcl,out}`. I ran `rustfmt --edition 2024` over stdin copies in `agents/sol-4/fmt/`.

## Verdict: ADMIT (source level), with one required doc-comment edit (D1)

The code is correct for every shape that idiomatic TclOO code uses, and it compiles against the exact HEAD signatures. It is a strict improvement on HEAD, which answers nothing in any TclOO member body. Unlike the held fix-1, it does not resolve through the declaring namespace. D1 is required because the doc comment states something false. R1–R5 are known wrong-positive residuals in adversarial edge cases. They are documented here and none of them blocks admission.

## Semantics against tclsh 8.6.18 and 9.0.4 (both gave identical results)

| Probe | tclsh | Candidate |
|---|---|---|
| `nextto A` in `::Ns::B`, only `::Ns::A` exists (p1 A) | ERR `A does not refer to an object` | None (refuses) ✔ |
| `nextto A` with both `::A` and `::Ns::A` (p1 B) | `::A` | `::A` ✔ |
| `nextto Ns::A` (relative-qualified) from a method (p1 C) | `::Ns::A` | root lookup gives `::Ns::A` ✔ |
| `::oo::Helpers::A` and `::A` both exist (p1 D) | `::oo::Helpers::A` | refuses ✔ (conservative) |
| `nextto ::A` (p1 E) | `::A` | `::A`; the shadow check is skipped ✔ |
| `Q::A` under `::oo::Helpers::Q::A` and `::Q::A` (p1 F) | Helpers one | refuses ✔ |
| alias `jump` → `::oo::Helpers::nextto Base`, called from `::N::C` (p4) | `::Base` (object frame) | resolved at the call site to `::Base` ✔ |
| Tcl 9 class `initialise` body (p2) | path `::oo::Helpers ::oo`; `nextto` raises `may only be called from inside a method` | path non-empty (`oo.rs:1786`). Unreachable: `original_source_member_at` needs `original_member_context`, which only `walk_method_body` sets, and the references code scans only ctor/dtor/method bodies. Harmless today. |

snit/itcl bodies are unchanged. `walk_method_body(..., false)` at `oo.rs:2476-2485` and `per_item.rs:2494` set `oo_global_resolution` only for TclOO (`oo.rs:1267`), so the path is empty and the old scoped branch runs.

## Findings

**D1 (required, doc) — `b/definition.rs:1328-1330`.** "The object namespace holds no retained class" is false.
- At runtime, a relative `oo::class create Inner` in a member body lives in `::oo::ObjN`, and `nextto Inner` resolves to it over `::Inner` (p7: `implementation by "Inner" not reachable from here`, which is the object-ns class, not `::Inner.k`).
- The analyser does retain such a class. It is keyed at `::Inner` through the static `"::"` approximation (`scope.rs:166`, `handlers.rs:10721-10734`) with `source_name: None`, because the method scope's `naming_scope` is None.
- Fix: reword the sentence as an explicit limitation, for example: "Classes the object namespace may hold at runtime are not modelled; the root lookup assumes none." Also replace "same-tail class" with "a class the written name reaches under a path namespace", because `Q::A` is relative-qualified, not a tail.

**R1 (low) — class-object frames.** In `self method` bodies, in methods mixed into a class object, and in metaclass instance methods, the frame's path is `::oo::Helpers ::oo`, not `::oo::Helpers` (p5, p6). `nextto Q2` resolved `::oo::Q2` over `::Q2` there. The refusal only consults `TCLOO_MEMBER_BODY_NAMESPACE_PATH = ["::oo::Helpers"]` (`tcl-registry/src/definer.rs:4375`). Built-in `::oo` classes (`class`, `object`, `Slot`, `singleton`, …) are not in `all_classes` either. This is a gap in the registry data, not something the patch introduced. Note it in the doc, or refuse relative names when the member side is class-side.

**R2 (low) — non-class commands on the path win.** `proc ::oo::Helpers::Z` shadows class `::Z`. So do the builtin helpers `self`/`next`/`link`/… and the per-object `my`. In each case tclsh raises `X does not refer to an object` (p1 G, p2), but the candidate answers the root class. Only classes are probed, at `b/definition.rs:1344-1348`.

**R3 (low) — absence-as-completeness in the shadow probe.** At `b/definition.rs:1346`, `resolve_written_class_name(..).is_some()` cannot tell a class that is absent from one withdrawn for collision or ambiguity. `resolve_class_lookup` returns None when `source_name_ambiguous` is observed or several records share a slot (`class_hierarchy.rs:805-829`), and `resolve_written_class_name` returns None when policies disagree (`:898-923`). A withdrawn class on the path is therefore treated as "no shadow", and the root lookup answers. The root lookup itself withdraws correctly.

**R4 (low) — runtime `namespace path` edits.** The idiom `namespace path [linsert [namespace path] end ::H]` in a constructor makes `nextto A` resolve to `::H::A` over `::A` (p8). The analyser records these as `namespace_path_computed`, but the candidate ignores them.

**R5 (low, pre-existing) — nested frames.** A `nextto` inside `namespace eval` within a method raises in tclsh (p4), but the path stays non-empty (`scope.rs:826`), so the candidate answers. HEAD may also have answered here through the nested scope's `naming_scope`.

**S1 (style, recommended) — `b/definition.rs:1346`.** `format!("{namespace}::{name}")` re-derives the shared join. `tcl_syntax::naming::qualify(namespace, name)` (`tcl-syntax/src/naming.rs:1445`) is the owner, and it collapses the written name's colon runs. Keep the `starts_with("::")` guard: `qualify` returns absolute names unchanged, so the probe would then "shadow" itself.

## Compile, format and lint (source level)
- `implicit_command_namespace_path_at(&Scope, u32) -> &'static [&'static str]` (`scope.rs:773`) and `pub mod scope` (`analyser/mod.rs:67`) are already used from lsp-core at `references.rs:579`. ✔
- `resolve_written_class_name`, `resolve_written_class_name_in_scope` and `source_namespace_at` are `pub` with matching argument types (`class_hierarchy.rs:849,877,898`). There are no name clashes in definition.rs. ✔
- `pub mod definition` (`lib.rs:55`). `pub(crate) fn` raises no `must_use_candidate`. The doc_markdown terms are backticked. ✔
- `rustfmt --edition 2024` (no rustfmt.toml) leaves both patched files unchanged. ✔
- Callers: `canonicalise_class_name` had exactly two callers (`references.rs:2409,2450`) and both are migrated. The only remaining `canonicalise_class_name` is the unrelated compiler W308 one (`var_command.rs:2004`). No markdown docs name the removed fn. Gates: definition.rs is a `registry_axes` CLEAN file, but `"::"` is not registry vocabulary. definition.rs is sanctioned in `resolution_drift`. ✔
- No hardcoded command or namespace names: the path comes from registry data. ✔

## Test expectations
HEAD's helper returns None in every TclOO member body (`source_namespace_at` returns the method scope's `naming_scope` = None). The only behaviour change is from None to Some, and only where the path is non-empty.
- `definition::tests::nextto_picks_occurrence_at_cursor_not_first_on_line` (test-256): red → **green** (root `::A`).
- `jump_to_nextto_named_class_method`: red → **green**.
- `jump_to_nextto_namespaced_class_method` (`definition.rs:7075`): **stays red**. The test **contradicts tclsh** (p1 A raises), so rewrite it to assert no target. Add positives for `nextto Ns::A` and `nextto ::Ns::A`, plus a `::oo::Helpers::A` refusal.
- `original_next_definition_uses_captured_target_and_selected_availability` (`definition.rs:8694`): red → **green**. The `jump` call site is inside the method, so the lookup goes through root to `::Base`, which matches p4. Its negative arms (8.6 surface, shadow proc, `$unknown`, `Else`) are decided before canonicalisation or still miss.
- `constructor_next_chain_reference_via_nextto_explicit_target` and `..._via_braced_nextto_target` (`references.rs:6166,6192`): red → **green**. Constructor bodies set `oo_global_resolution` (`oo.rs:1285`). `base_refs` stays empty because the provider is Grandparent.
- No currently-passing test can flip. The other ctor/dtor/code-lens/e2e lens tests use plain `next` (target None). `tp_nextto_dispatch_top_level` goes through `method_next_dispatch_spans` (no canonicalisation). The `references.rs:8946/8972` tests and e2e `same_file_tcloo_nextto_call_arity_checks_named_target` do not reach this helper.

## Side note (out of scope)
`docs/design/analysis/name-resolution.md:544-547` says `resolve_written_class_name` falls back to a "unique tail". The current code explicitly does not (`class_hierarchy.rs:895-897`), so the doc is stale.
