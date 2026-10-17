# Sol-11 v2: `nextto-namespaced-definition-body-v2.patch`

Base: `e46e88617` (integration39). Scratch trees `a/` and `b/` were built from
`git -C /home/user/tcl-lsp show e46e88617:<path>`. The repository was not modified. This is a static
revision: nothing was compiled, and no new test runs were made for v2.

Applying and formatting:
- `git apply --check` passes against the live tree.
- `rustfmt --edition 2024 --check` is clean on both patched `.rs` files.
- No `#[allow]` was added.
- Since v1, the three touched files have not changed upstream (`git diff 1433f2ffe e46e88617` is empty for them).

## Changes relative to v1 (Luna-11 HOLD items)

### R1: code comment
In `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` (`command_inner`, after
the `regions()` failure), the comment is now Luna's text:
- The script runs in a new frame in the definer's namespace: `::oo::define`, `::oo::objdefine`, or a
  configurable support namespace.
- Its remaining reach is `uplevel`/`upvar` to the caller frame and absolute names, which is what an
  unresolved command can also do.
- The development-history clause is gone.

The code is unchanged from v1. When `regions()` fails and the schema has a
`DefinerFamily::TclOo` definition-body grammar, the walk calls `record_uncertainty`. Every other
failure still calls `widen`.

### R2: contract
`docs/design/contracts/name-handling.md` has two edits:
- **At about :156**, the end of the "Immediate child mutation coverage…" paragraph, one added sentence:
  > An unentered `TclOO` definition body (a `DefinerFamily::TclOo` definition-body grammar) runs in a new
  > frame in its definer's namespace, so it records the unresolved-command uncertainty instead; its
  > `uplevel`/`upvar` and absolute-name reach keeps `UnknownEarlierMutation` on later advice (tclsh 8.6.18,
  > 9.0.4 and 9.1.0).
- **At about :203**, the "Immediate script bodies … keep their existing effect projection" sentence gains
  the clause ", except the unentered `TclOO` definition body above, which records uncertainty".

Evidence: Luna's `luna-11/probe/q2.tcl`, re-run today on 8.6, 9.0 and 9.1:
- Case 1 prints `1 later oo::class -> P-oo-class create B {}`: an `uplevel 1 {namespace path ::P}` in a
  class body redirects a later relative `oo::class` in the enclosing namespace.
- Case 3 shows that a relative `namespace path` inside the body sets `::oo::define`'s path, not the
  enclosing namespace's path.
- Cases 4 and 6 show that the same `uplevel` reach applies to `oo::define` and `oo::objdefine`.
- My own `probe/p1.tcl` shows that `namespace current` inside `oo::class create K {…}` is `::oo::define`.

### R4: regression test
New test in `rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs` (`tests`):
`c_source_namespace_class_after_unentered_definition_body_keeps_unknown_mutation`.

It builds this source, where `B` is the second relative `oo::class create`:
```
namespace eval ::P {}
namespace eval outer {oo::class create A {<body>}; oo::class create B {}}
```
It runs each `<body>` under `tcl8.6` and `tcl9.0`. The bodies are:
- `method m {} {}`: the plain case;
- `uplevel 1 {namespace path ::P}`: the companion that pins the soundness boundary.

For each body and dialect it asserts:
- `source_class_declaration_at` (at the last `oo::class create`) returns a declaration;
- its `source_class(&analysis)` is `::outer::B`;
- its `factory.obligations()` contains `UnknownEarlierMutation`.

`::P` is created first so that the companion also runs as valid Tcl.

What makes the test expectations credible:
- **Without the patch**, the declaration is None. This is the v1 diagnosis: `c_source_head_key` refuses
  `oo::class` because `::outer` is in `unknown_paths` after the widen.
- **With the patch**, the declaration exists. The v1 gdb emulation, run on the namespaced nextto fixtures
  of the same shape, recorded `::Ns::B` and `::Ns::C`.
- **The obligation comes from the code.** `schema_advice` (source_transition_advice.rs:2641-2643) pushes
  `UnknownEarlierMutation` whenever `graph.uncertain_operations` is non-empty, and `record_uncertainty`
  adds the class-body words to that list. `source_class_declaration_attempt` builds the factory from
  `schema_advice`.
- **Not executed.** I could not emulate the test itself under gdb: the literal could not be rewritten at
  the same length, and the test is not in the build-38/39 binaries.
- I avoided the `original_completed_command_world()` assertions. The two existing sibling tests contradict
  each other on it, and `c_source_namespace_class_declarations_retain_original_body_and_conditional_scope`
  fails at baseline on `:1039`.

### F3: SpecTcl loader family default (not hardened)
`tcl-spectcl/src/loader.rs` defaults `family` to `TclOo` for any `definition_body` block without a family line.
- Changing that default affects every user or vendor spec pack, so it is not trivial.
- No bundled pack takes this path.
- The residual stays bounded by the unresolved-command uncertainty.
- If hardening is wanted, it should be a registry definition-namespace datum on `CommandSpec`, or a
  provenance check on the stock `tcl` pack. That belongs in a separate change.

## Still required
- **R3:** build and run the full tcl-compiler, tcl-lsp-core and tcl-lsp-server capture against baseline.
  This includes the new test and the 181 unsampled TclOO-in-`namespace eval` tests Luna listed.
- **Watch for negative flips** in TN/FP guards that used to refuse "for free" after a class body: the
  `issue923_class_refs`, `tcloo_navigation` and `references_rename` sibling-namespace tests, and
  `analyser.rs::a_relative_superclass_does_not_cross_link_to_another_namespace`.
