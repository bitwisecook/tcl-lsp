# Splitting tcl-registry: a core plus one crate per surface

Status: designed 26 November 2026. Plan item 261 leads it, with steps 262–265.
It is a companion to [splitting tcl-compiler](../compiler/crate-split-design.md),
and runs ahead of the other stages for the same reason: build and iteration
time. It builds on the four-layer model in
[dialects, packages and environments](dialect-and-package-registry-redesign.md#0-the-model-in-four-layers).

## Problem

`tcl-registry` is 583k lines and sits under every Tcl-facing crate. The
registry rule sends every per-command fix there. As a result:

- An edit to one iRules command spec rebuilds `tcl-compiler` and the whole
  stack above it.
- Every worktree compiles all of it, whatever part of the stack it works on.

| Part | Lines | What it is |
|---|---|---|
| `commands/` | 185k | per-surface command data: `irules` 64k, `tcl` 48k, `tcllib` 27k, `tk` 22k, `stdlib` 12k, `expect`, `iapps`, `spectcl`, `sslictcl`, `bpf`, `thread`, `itcl`, `argparse`, `ticklecharts` |
| `bigip/`, `base_objects/`, `f5/` | 174k | BIG-IP schema data (item 118) |
| `native_*` | 61k | interpreter algorithms (item 117) |
| the rest | ~163k | the vocabulary (`CommandSpec`, roles, arity, evaluation contexts, effects, forms, hooks), lookup and resolution, the dialect, package and environment glue, `value_transfer` and `resolved_invocation` (35k, which item 117 moves to the analysis layer) |

The command data is already one module per surface, behind
`commands::SpecPack { id, specs: fn() -> Vec<CommandSpec> }`. Two couplings
keep it inside the core:

1. **The surface list is in the core.** `registry.rs` (the `shared_group!`
   cells) and `registry/lookup.rs` name every surface.
2. **The core calls into the Tcl surface, at about 60 sites.**
   - `pack_hooks/native_catalogues.rs` maps hook names to `commands::tcl::fold_*`
     constant folds.
   - `native_try.rs` and `registry/control_arms.rs` read `NATIVE_TRY_GRAMMAR`.
   - `substitution.rs` reads `subst_::OPTIONS`.
   - `definer/property_accessors.rs` and `native_tcloo_info.rs` use TclOO kinds.
   - `native_compilation/*` reads the `if` shape checks.
   - `variable_output.rs`, `value_transfer/regex.rs` and `spec/subcommand.rs`
     call individual helpers.

   No surface names another surface: the `tcl` module has no reference to
   `irules` or `bigip`.

## Design

The crates follow the model's units. A dialect core profile and a package are
both providers of surface declarations, so each gets a crate. The core holds
the vocabulary and the machinery that reads them.

| Crate | Takes | Depends on |
|---|---|---|
| `tcl-registry` (core) | the vocabulary; lookup and resolution over a given set of surfaces; the hook API; the dialect, package and environment model | `tcl-dialect`, `tcl-lexer`, `tcl-syntax`, `tcl-cmd-core`, `tcl-runtime-api` |
| `tcl-surface-tcl` | the Tcl core profile's commands (8.4–9.1) and their hooks, folds and grammars | core, `tcl-cmd-core` |
| `tcl-surface-f5` | the `f5-tcl`, `f5-irules` and iApps/tmsh surfaces | core, `tcl-surface-tcl`, the BIG-IP schema crate (item 118) |
| `tcl-surface-tk`, `tcl-surface-tcllib`, `tcl-surface-stdlib`, `tcl-surface-expect` | one package surface each | core, `tcl-surface-tcl` where a surface extends Tcl's |
| `tcl-surface-ext` | the small surfaces: `thread`, `itcl`, `argparse`, `ticklecharts`, `bpf`, `sslictcl`, `spectcl`'s own (each under 3k lines; split further only if a measurement shows the need) | core |
| `tcl-surfaces` | the shipped catalogue: the list of every surface crate, and the compiled-in environments that place them | every surface crate |

Jim's surface stays SpecTcl-authored in `tcl-spectcl`, as Q1/Q6 ruled.

### Rules

- **The core names no surface.** It builds its tables from the surfaces it
  is given: `Registry::from_surfaces(&[Surface])`. A `Surface` is today's
  `SpecPack` plus the surface's hook table.
- **A surface's behaviour lives with its data.**
  - The `fold_*` folds, `NATIVE_TRY_GRAMMAR`, `subst_` options and TclOO kinds
    move into `tcl-surface-tcl`, which registers them through its hook table.
  - Where one is a pure command kernel, it moves to `tcl-cmd-core` instead
    (item 81's plan).
- **The registry rule is unchanged.** Per-command knowledge lives in a surface
  crate, which is still "the registry" for that rule. `registry-axes` treats
  the surface crates as registry.
- **Generic consumers depend on the core only.** `tcl-compiler` (and, after
  item 90, `tcl-binding`, `tcl-diagnostics` and the rest) depends on
  `tcl-registry`, never on a surface crate.
  - Only entry points select surfaces: binaries, the unit crate's session, and
    tests. Entry points depend on `tcl-surfaces`, or on just the surfaces they
    need.
  - Tests of generic crates take `tcl-surface-tcl`, or the surface they
    exercise, as a dev-dependency. No surface crate depends on a generic crate,
    so this causes no cascade (see step 0 of the compiler split).
- **Selection is explicit.** The process-wide registry is installed once by
  the entry point from the surfaces it links.
  - An uninstalled registry fails loudly rather than silently serving an empty
    table.
  - `tcl-surfaces` offers the full set; tests and isolated tools pass the subset
    they need.

### What it buys

| Edit | Rebuilds today | Rebuilds after |
|---|---|---|
| one iRules command spec | `tcl-registry`, `tcl-compiler` and everything above | `tcl-surface-f5`, `tcl-surfaces`, the binaries |
| one Tk spec | the same | `tcl-surface-tk`, `tcl-surfaces`, the binaries |
| the analyser | `tcl-compiler` and above | unchanged by this split; item 90 covers it |

A worktree on the analyser or binding compiles the registry core and
`tcl-surface-tcl` (for tests), not 359k lines of other surfaces' and BIG-IP
data.

## Steps

The step designs are written when each step starts, building on the previous
one. The tree may stay broken between steps. Gates and failset comparisons
run when the crates build again.

1. **Item 262: invert the surface list.**
   - The core takes surfaces as input, and the `shared_group!` cells and
     `lookup.rs`'s list move to the entry point's assembly.
   - Everything stays in one crate for now; this is the cut that makes the move
     possible.
2. **Item 263: move behaviour to its surface.** The roughly 60 core → `tcl`
   references become hook-table registrations or move to `tcl-cmd-core`.
3. **Item 264: extract the surface crates and `tcl-surfaces`.** Every consumer
   is moved to the core plus the surfaces it names, and `registry-axes` is
   re-pointed.
4. **Item 265: dependency hygiene.** A gate rejects a generic crate's
   normal dependency on a surface crate. `tcl-compiler`'s cone (and after item
   90 each split crate's) contains no surface crate.

Items 117 (native algorithms out) and 118 (BIG-IP schema crate) are
independent of these steps and can run in parallel. `tcl-surface-f5` depends on
118's crate.

## Risks

- **Install order.** Code that reads the registry before an entry point
  installs it. Mitigation: the loud failure above, and one test per entry point
  that it installs before first use.
- **Cross-surface helpers.** A surface that today borrows another's helpers
  (Tk's from Tcl, `f5-tcl`'s from Tcl) depends on that surface crate, never on a
  copy.
- **Lazy construction.** Today's lazily built, leaked spec groups keep their
  laziness per surface. The SpecTcl compiled-surface work
  ([plan](spectcl-compiled-surface-plan.md)) is unaffected, because it reads
  surfaces through the same core.
