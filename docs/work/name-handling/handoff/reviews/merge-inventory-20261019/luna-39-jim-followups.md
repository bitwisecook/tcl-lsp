# Luna-39 review: Jim follow-ups on `work` (2026-10-19)

Read-only review of three uncommitted changes, plus a drift check of the applied
`sol-34_jim-core-store-v2.patch`. I did not run cargo; the findings come from
reading the code and diffing the files.

| # | Change | Verdict |
|---|--------|---------|
| 1 | `tcl-registry/src/resolved_invocation.rs` `extend_repeated_roles` | **ADMIT** |
| 2 | `tcl-spectcl/tests/jim_own_surface.rs` `a_jim_document_resolves_every_declared_command` | **ADMIT-WITH-CHANGES** (pin the override set) |
| 3 | `tcl-runtime-api/src/native_literal.rs` `#[allow(clippy::too_many_arguments)]` x2 | **ADMIT** |
| - | sol-34 v2 patch vs working tree | **No drift** |

## 1. `extend_repeated_roles`: optional leading word without a frame effect

**Verdict: ADMIT.**

### Users of `optional_leading_word` / `-optional-leading`
I searched every `.rs` file under `rust/`, every `.tclspec` under `rust/`, `specs/`,
`runtime/`, `ai/`, `editors/`, `samples/`, `tests/`, `grammars/` and `scripts/`, and
every tracked non-Rust file.

| User | Frame effect? | Path after change |
|------|---------------|-------------------|
| `tcl/upvar_.rs` (`UPVAR_REPEATED_ARGS`, `strided(VarWrite,1,2)` + optional leading) | yes, `FrameEffectSpec::UPVAR` | frame path, **unchanged** |
| Jim `loop` (`jim-own-surface.tclspec:128`, `repeat Body -from 2 -stride 250 -optional-leading`) | no | `layout.indices(count)` |
| Jim `proc` (`jim-own-surface.tclspec:238-239`, `StaticVarList ... -exclude-trailing 1` and `Body`, both `-optional-leading`) | no | `layout.indices(count)` |
| `docs/design/spec-dsl-examples/upvar.tclspec:43` (doc example) | yes (`frame_effect -level-word ArityParity -layout AliasPairs`) | frame path |
| `tcl-spec-studio` (render/coverage/draft), `tcl-spectcl/src/loader.rs`, `commands/spectcl/rows.rs` | n/a. These are tooling and the parser, not command rows | - |

- `uplevel` declares no repeated layout. `namespace upvar` does not set the flag.
- No other spec sets `optional_leading_word` with a frame effect, so the
  frame-effect behaviour stays exactly as it was.
- Only Jim's `loop` and `proc` set the flag without a frame effect. For both, the
  last word is the right anchor. Their own spec comments state this ("a group anchored
  to the end of the call ... with a stride longer than any call is one word wide, the
  last").

### Arity check (`RepeatedArgLayout::indices`, shift = `(end-start-1) % stride`)
- `loop`, Body from 2, stride 250:
  - 3 words: end 3, shift 0, so **[2]**.
  - 4 words: shift 1, so **[3]**.
  - 5 words: shift 2, so **[4]**.
  - These match the `refine limit|bounds|stepped` rows.
- `proc`, 3 words: StaticVarList gets end 2 <= start, so **[]**. Body is **[2]**.
- `proc`, 4 words: StaticVarList gets end 3, shift 0, so **[2]**. Body gets shift 1, so **[3]**.
- These match the `refine plain|statics` rows.

### Why it was broken
- `loop` and `proc` have no frame effect, so `frame_layout.or_else(frame_effect.map(..))`
  was `None`. That hit `_ => complete = false`.
- `arg_roles()` then fell to the resolver fallback. These rows have no
  `arg_role_resolver`, so it returned `None`.
- As a result, `CommandRegistry::arg_indices_for_role("loop", ...)` returned `[]`, and
  `a_loop_body_is_declared_at_each_arity` failed.
- The change also fixes Jim `proc` StaticVarList/Body roles through `arg_roles()`. That
  extra effect is intended and restores parity with origin.

### Parity with origin/rust
- `origin/rust:registry.rs` `arg_roles_in` goes through `push_repeated_roles`, which
  always calls `layout.indices(tail_len)` (upvar included). Nothing frame-aware happens
  there.
- The new branch reproduces exactly that for layouts without a frame effect.
- For upvar, work still uses the more precise frame-grammar width. That is a deliberate
  work-branch divergence that already existed, and this change does not touch it.
- Coordinates match: work's `count` is taken after `argument_offset`, which corresponds
  to origin's `n - 1` with offset 1 for subcommands.

### Nit (optional, non-blocking)
`frame_layout` is only ever `Some` when `semantics.frame_effect` is `Some`:
- The only `Some` producer is `resolved_invocation.rs:3728-3732`, which maps
  `semantics.frame_effect`.
- The `arg_roles()` fallback passes `None`, and it is already guarded by
  `frame_effect.is_some() => return None`.

So `frame_layout.is_some() ||` is redundant. You could simplify the condition to
`layout.optional_leading_word && self.semantics.frame_effect.is_some()`. Keeping it as a
defensive check is harmless.

## 2. `a_jim_document_resolves_every_declared_command`: native Jim rows win

**Verdict: ADMIT-WITH-CHANGES.**

### The correction is justified
- Since 944d15692 (on `work` only, not an ancestor of origin/rust), `tcl/mod.rs:433-434`
  registers native `alias_::spec()` and `exists_::spec()`. Both are gated
  `core_in(Jim, [("0.84", None)])`.
- The pack rows are `jim 0.76-`.
- `ContextRegistry::resolve_command` (`model/assembly.rs`) ranks nearness first, then the
  **narrowest breadth**, then the last-registered row. So the native row wins at
  0.84 and later, and the pack row still wins at 0.76-0.83. That is the documented
  most-specific-wins rule, not an accident.
- The native `alias` row carries the execution contract the pack cannot express:
  - `state_transitions: CREATES_CALLER_ALIASES` (caller-namespace alias binding,
    `OnOkOnly`).
  - `INSTALLS_NAMED_DEFINITION | REFLECTS_COMMAND_NAMES`.
  - `native_compilation`.
- The native `exists` row carries `native_compilation`, `runtime_backing`, and the Jim
  enum `arg_role_resolver` (VarRead or Name by selector). The pack row has **no** roles
  at all, so for analysis the native `exists` row is strictly better on roles.
- The spectcl loader *refuses* `native_compilation` in packs
  (`loader.rs:6842-6845`, "this pack cannot supply them").
- Resolution should therefore **not** prefer the pack row. Doing so would drop the only
  row that carries the execution and compilation contract.

### Required change: the new disjunct is too broad
As written, any future native Jim row that silently shadows any pack row would pass
unnoticed. Pin the set:

```rust
/// Names whose native registry row (`tcl/alias_.rs`, `tcl/exists_.rs`)
/// carries the execution contract and supersedes the pack row.
const NATIVE_JIM_ROWS: &[&str] = &["alias", "exists"];
...
        assert!(
            std::ptr::eq(resolved, spec)
                || (NATIVE_JIM_ROWS.contains(&spec.name)
                    && own_family(resolved)
                    && resolved.native_compilation.is_some()),
            "{}: the jim row is the one selected",
            spec.name
        );
```

Optionally, also assert that each `NATIVE_JIM_ROWS` name actually resolves to a non-pack
row (`!std::ptr::eq(resolved, spec)`), so the list cannot go stale.

### Follow-up (not blocking this change; for the 944d15692 owner)
When the native row wins, it drops analysis metadata the pack row had:
- **`alias`, arg 1:** the native row has `ArgRole::Name`; the pack has `CommandNameProbe`.
  The pack's role is a navigable command reference, so the alias target loses
  find-references and go-to-definition.
- **`alias`, `defines_command_at`:** the native row lacks `defines_command_at: Some(0)`.
  `authored_source_command_publication()` (`resolved_invocation.rs` ~5523) therefore
  returns `None` for `alias newname ...` at 0.84 and later. Someone should confirm that
  the state-transition path covers source-side publication of the new name.
- **`exists`:** the native row has no `hover` and no option table. Hover, `-option`
  completion and validation from the pack are likely lost at 0.84 and later. I found no
  hover merge across same-name rows in `model/assembly.rs`.
- **`exists`, non-variable selectors:** these yield `Name`, where `CommandNameProbe` would
  match the pack's intent for `-command`, `-proc` and `-alias`.

The fix belongs in the native rows (adopt the pack's roles, `defines_command_at`, hover
and options), not in resolution order.

## 3. `#[allow(clippy::too_many_arguments)]` on `register_using` / `create_using`

**Verdict: ADMIT.**
- `register_using` takes 8 parameters and `create_using` takes 9. Both exceed clippy's
  default of 7, and both are closure-parameterised protocol entry points where a params
  struct would add little.
- The repo already uses `#[allow(clippy::too_many_arguments)]` in 50 places, and in no
  place uses `expect`, so this follows the house style.
- The diff touches nothing else in the crate.

## Drift check: sol-34_jim-core-store-v2.patch

Method:
1. Copied the HEAD versions of the 7 files into a scratch directory.
2. Applied the patch there (it applied cleanly).
3. Diffed the result against the working tree.

Result:
- `model/assembly.rs`, `model/mod.rs`, `tcl-spectcl/src/install.rs`,
  `docs/design/registry/spec-packs.md`, `tcl-mcp/src/tools.rs` and
  `tcl-explorer/src/lib.rs` are **identical** to patch-on-HEAD.
- `tcl-spectcl/tests/jim_own_surface.rs` differs **only** by item 2's hunk: the doc
  comment on `a_jim_document_resolves_every_declared_command` and the widened
  `ptr::eq` assertion.
- `git apply -R --check` also passes for every file.
- **No drift.**
