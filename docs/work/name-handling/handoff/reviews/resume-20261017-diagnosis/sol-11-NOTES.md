# Sol-11: namespaced `nextto` (definition.rs:7114 / :7127)

Tree: `work` @ 1433f2ffe (integration38, now committed; `git status` clean). Binary: `tcl_lsp_core-c42d7b3d9c3645e1`
(sha256 4cd56d42…b0a6), copied to `tcl_lsp_core`. Every run was a single process at `nice -n 19` with
`--exact --test-threads=1`, one at a time. Nothing under /home/user/tcl-lsp was modified.

## Answer: where it first fails

The first failing step is **not** in `canonicalise_class`, `scan_next_dispatch_sites_with_target`,
`member_next_provider` or the class keys. It is upstream, in the compiler's C-source advice tape:
`::Ns::B` and `::Ns::C` are never recorded as source classes, so `C::greet` never gets a method scope.

1. `next_dispatch_target` (definition.rs:1290): `original_source_member_at` (class_hierarchy.rs:115) returns
   **None**. It visits only 3 scopes (global, `Ns`, `A::greet`). `scan_next_dispatch_sites_with_target`
   and `canonicalise_class` are never reached. Evidence: runs/gdb2-*.txt.
2. `walk_method_body` (oo.rs:1914) runs once, for A only (runs/gdb3-*). For B and C, `handle_oo_class_source`
   cannot resolve the creation, so `resolve_class_creation` falls through to `defer_class_creation`
   (handlers.rs:10722). The cause is that `OriginalClassDefinerSource::capture` (original_definer.rs:20) gets
   None from `source_class_declaration_at` (source_structure.rs:2503): the tape has no `class_declarations`
   entry at B's or C's offset (source_class.rs:578). Evidence: runs/gdb4-*, gdb5-*, gdb6-*.
3. The tape walk of the `namespace eval Ns` body (body_effects.rs `BodyWalk::command_inner`, :1157) handles
   A fully. For B and C it returns right after `substitutions`, because `graph.resolve("oo::class")` returns
   None (:1231). `AdviceGraph::lookup_head_key` uses `c_source_head_key` (c_namespace.rs:51). The relative
   local slot `::Ns::oo::class` is unoccupied, and `unknown_paths` contains `::Ns`, so it refuses
   (c_namespace.rs:70-72). Evidence: runs/gdb8-*.
4. `::Ns` is in `unknown_paths` because, after `oo::class create A {…}`, `command_inner` called
   `graph.widen()` (body_effects.rs:1292-1298). That call runs `withdraw_namespace_advice()`
   (source_namespace.rs:29), which marks **every** namespace's path as unknown. `widen` was called because
   `regions()` returned None (runs/gdb10-*: the only widen caller is the regions site). That happened because
   `prepare_region_branch` (body_effects.rs:797) cannot enter a non-empty Structural class-definition body:
   - the Native arm, `c_source_namespace_scope` (c_namespace.rs:263), accepts only `DECLARES_NAMESPACE` bodies;
   - the Logical arm, `logical_namespace_scope`, has the same restriction.
   Evidence: runs/gdb9-*.

This also explains why the global tests pass. At `::`, the local slot `::oo::class` is occupied, so the unknown
path is never consulted. Empty bodies (`{}`) are not affected either: their span is empty, so they skip the
namespace check and never widen. The effect is broader than `nextto`: after any non-empty TclOO body inside
`namespace eval`, every later relative head in that namespace is refused by the C-source tape (`oo::class`, `proc`, …).

tclsh 8.6.18 and 9.0.4 (probe/p1.tcl):
- `nextto Ns::A` and `nextto ::Ns::A` both answer `hi`.
- `nextto A` gives `A does not refer to an object`.
- `namespace current` inside `oo::class create K {…}` is `::oo::define`.

probe/p2.tcl shows that `namespace path` does apply to relative qualified names. The refusal in step 3 is
therefore correct once paths are unknown; the fault is the over-wide withdrawal.

### The downstream path is already correct
I patched the test literal in memory under gdb (patchsrc.py, runp.gdb) to write `::oo::class create B/C`
(same length, same offsets). With that change:
- fully qualified: passes (runs/patch-fq-abs.txt);
- relative `Ns::A`: passes;
- bare `A`: still none.

So `canonicalise_class`, `member_next_provider` and the `::Ns::A` keys all work once B and C exist.

## Fix: `nextto-namespaced-definition-body.patch`
The patch changes one site in `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs`
(`command_inner`). If `regions()` fails for a command whose schema has a **TclOO** definition-body grammar,
the walk now calls `graph.record_uncertainty(&native.words)` instead of `graph.widen(...)`. Every other
failure still widens.

How it fits the constraints:
- Registry data only: it uses `ResolvedInvocation::authored_source_definition_body_grammar()` and
  `DefinitionBodyGrammar.family`. No command names appear.
- Rationale: the definition script runs in the definer's namespace (`::oo::define`), and the
  definition-member inventory owns its members. What remains (absolute effects from non-member commands) is
  what any unresolved command leaves, and this walker already records that as uncertainty.
- Unchanged: the TclOO resolution order in `canonicalise_class` (object ns, `::oo::Helpers`, `::`, never the
  class namespace), every assertion, empty-body behaviour, and the non-TclOO definers.
- No `#[allow]`.
- `git apply --check` passes against the current tree, and `rustfmt --edition 2024 --check` on the patched
  file is clean.

## Verification (patch not compiled; emulated under gdb)
`rune.gdb` / `emulate.py`, then the exact version `rune2.gdb`: at `AdviceGraph::widen` entry, when the return
address is the regions-failure site in `command_inner`, gdb calls the real
`authored_source_definition_body_grammar(&schema)` (schema at command_inner `rsp+0x930`). If that returns
non-null, gdb redirects `$pc` to `record_uncertainty`, which has an identical signature. This is the patch's
behaviour except for the family filter; every grammar in these fixtures is TclOO.

The four nextto tests plus the helpers-path test, baseline vs emulated (runs/emu2-*.txt):

| test | baseline | with patch (emulated) |
|---|---|---|
| jump_to_nextto_named_class_method | ok | ok (0 redirects) |
| jump_to_nextto_namespaced_class_method (bare, expects none) | ok | ok (30 redirects; B and C now exist and the result is still none) |
| jump_to_nextto_relative_qualified_class_method | FAIL :7114 | **ok** |
| jump_to_nextto_fully_qualified_class_method | FAIL :7127 | **ok** |
| nextto_refuses_a_class_name_reached_through_the_helpers_path | ok | ok (refused through the shadow check: `::oo::Helpers::A` resolves) |

Blast-radius sample, base vs emulated, comparing pass/fail and the failure message:
- 25 tcl_lsp_core tests (blast.txt) and 36 tcl_compiler tests that mix `namespace eval` with non-empty
  class bodies (blast_c.txt, `tcl_compiler-30b36f5fc9611193`).
- **Zero differences.**
- Several of those tests already fail at baseline in this tree, with identical messages before and after
  (rename_safety, implementation, type_hierarchy, and many compiler diagnostics tests). Those compiler tests
  were run with the scratch directory as cwd.

## Open items
- Build the patch and run the full capture. The gdb emulation does not replace a compile.
- I added no regression test. Suggested one: a compiler-level check that the second relative
  `oo::class create` after a non-empty body in `namespace eval` gets `source_class_declaration_at`.
- snit and itcl definition bodies probably have the same over-wide widen. They are left out of this patch
  because they are not tclsh-verified.
- The more precise long-term fix is to enter TclOO definition bodies in the registry's definition namespace
  (`native_class_factory_recipe(...).definition_namespace(Instance)`). That would also walk member commands,
  and member bodies could widen inside the branch, so it is not minimal.
