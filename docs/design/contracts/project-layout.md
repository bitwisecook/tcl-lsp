# KCS: Project layout contracts

## Symptom

A change is hard to place because the boundaries between the language
pipeline, the analyser, the LSP protocol surface, and the developer
tools are unclear.

## Operational context

The Python code is partitioned into **seven concern packages**.  Each
has a defined role in the dependency graph; the direction is enforced
by [`import-linter`](../../../.importlinter) and gated in
[`make ci-fast`](../../../Makefile).

| Package      | Role                                                              |
|--------------|-------------------------------------------------------------------|
| `shared/`    | Leaf utilities — Range, Token, SourcePosition, document buffer, source map, ranges, codes, naming, dialect-agnostic text helpers, diagnostic primitives. |
| `compiler/`  | Tcl pipeline — lexer, parser, IR, lowering, passes, optimiser, codegen, WASM emitter, compiler-internal analyses (taint, var_escape, interprocedural, proc_arg_traits, var_scoping), command registry + per-command runtime, position lookup, Dialect enum. |
| `dialects/`  | Per-dialect command spec packs and dialect-aware data — `stdlib/`, `tcl/`, `tcllib/`, `expect/`, `eda/<vendor>`, `f5/{bigip,irules,iapps,query,xc}/`, `tk/{dialect,specs}/`. |
| `analyser/`  | IDE-facing semantic model + checks — `semantic_model`, `proc_lookup`, `signature_scan`, `class_hierarchy`, MRO, `checks/`, `_analyser/`, `irules_checks`, `conf_wrapped`, `packages/`, `compiler_checks` (the check orchestrator that runs over compiler IR). |
| `server/`    | LSP protocol surface — pygls wiring, feature handlers (`features/`), workspace indexing (`workspace/`), diagnostics pipeline, LSP conversion helpers (`_lsp_conv.py`), `_codes_init.py` side-effect module. |
| `tooling/`   | Developer tools — `tcl/`, `f5/`, `wasm/`, `vm/`, `explorer/`, `debugger/`, `fuzzing/`, `tclpkg/`, `refactoring/`, `formatter/`, `minifier/`, `diagram/`, `irule_test/`. |
| `ai/`        | AI integrations — Claude skills, MCP server, irule context helpers. |

## Decision rules / contracts

The seven import-linter contracts in
[`.importlinter`](../../../.importlinter) are the single source of
truth.  Summary:

1. **`shared/` is a graph leaf.** No outbound import to any other
   top-level concern.  This is what lets every other package import
   from `shared/` without creating cycles.
2. **`compiler/` ↛ `analyser/`, `server/`, `tooling/`, `ai/`.**
   Compiler stays below the analyser-and-up stack.  Two carve-outs
   for tiny lazy helpers (`compiler.irules_flow` →
   `analyser.irules_checks`, `compiler.taint._path_concat` →
   `analyser.checks._helpers`) that are pure-data utilities the
   analyser also publishes for its own diagnostics.
3. **`dialects/` → only `compiler.registry`, `compiler.parsing`, and a
   small set of pure-data compiler modules.**  Reaching into
   `compiler.codegen`, `compiler.optimiser`, `compiler.passes`,
   `compiler.taint`, `compiler.var_escape`, `compiler.interprocedural`,
   `compiler.cfg`, `compiler.ir`, `compiler.lowering`, etc. is
   forbidden — those are compiler internals, not dialect integration
   points.  Two carve-outs: the F5 XC translator
   (`dialects.f5.xc.translator`) legitimately consumes IR / lowering /
   expr-AST primitives because its job is iRules-Tcl → XC compilation;
   the vanilla-Tcl const-fold spec (`dialects.tcl.const_fold`) uses
   `compiler.tcl_expr_eval` to evaluate constant expressions baked
   into command specs.
4. **`dialects/` ↛ `analyser/`, `server/`, `tooling/`, `ai/`.**  One
   carve-out: `dialects.f5.bigip.explain_flow` lazy-imports
   `tooling.irule_test` to optionally drive the iRule test framework
   for a richer trace.
5. **`analyser/` ↛ `server/`, `tooling/`, `ai/`.**  One carve-out:
   `analyser._analyser._proc` lazy-imports
   `tooling.formatter.docstring` as a fallback when the proc has no
   preceding comment.
6. **`server/` ↛ `ai/`.**  Server *may* consume `tooling/` (refactorings,
   formatter, minifier, diagram extractor, explorer pipeline,
   `tclpkg`) to deliver LSP features.  AI integrations sit above the
   LSP, not below it.
7. **`tooling/` ↛ `server/`, `ai/`.**  One carve-out for the
   `f5-query irule context` verb that lazy-imports
   `ai.shared.irule_context` to produce a context bundle from the
   CLI.

## When you add a new module

- New compiler/parsing/analysis passes must expose stable, reusable
  facts.  Pick the package that matches what the module actually does,
  not where the caller lives.
- Editor- or transport-specific adaptation belongs in
  `server/features/`, not in `analyser/` or `compiler/`.
- New developer commands (CLIs, codemodes, debuggers) belong in
  `tooling/<sub-package>/`.  If the command should be on `$PATH` after
  `pip install`, add an entry under `[project.scripts]` in
  [`pyproject.toml`](../../../pyproject.toml).

## When you move behaviour between concerns

- Remove legacy module paths in the same change; do not leave
  compatibility wrappers behind.  The seven concern packages are the
  user-facing import surface — back-compat re-exports cost more than
  they save now that the structure is stable.
- Update all downstream consumers (other concern packages, `tests/`,
  `scripts/`) to import the new path directly.
- Update the relevant `__init__.py` docstrings if a package's role
  shifts.
- Re-run `make lint-imports`; if the move introduces a new edge that
  crosses a contract, either refactor or add an explicit
  `ignore_imports` entry in `.importlinter` with a comment explaining
  why the carve-out is acceptable.

## Anti-patterns

- A leaf concern (`shared/`) importing from a higher concern.  Catch
  early — `lint-imports` runs in `ci-fast`.
- A compiler pass calling into the analyser for a small helper.
  Either lift the helper to `shared/` or to `compiler.registry` if
  it's data, or duplicate it conservatively.
- A `dialects/` spec file reaching into a compiler internal beyond
  the registry/parsing surface.  Spec files are data — they should
  be reload-safe and not depend on compilation state.
- "Compatibility shims" or re-export modules that exist solely so
  legacy code keeps working.  Rewrite the callers.

## File-path anchors

- [`.importlinter`](../../../.importlinter)
- [`pyproject.toml`](../../../pyproject.toml) — `[project.scripts]` +
  `[tool.hatch.build.targets.wheel].packages`.
- [`AGENTS.md`](../../../AGENTS.md) — Repository layout section.

## Test anchors

- `tests/test_server_config.py` — server-side smoke.
- `tests/test_compilation_unit_parity.py` — compiler pipeline parity.
- `tests/test_workspace_index.py` — workspace indexing.

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) "Repository layout" section —
  per-package one-line summaries.
- [`shared-utility-contracts.md`](shared-utility-contracts.md) —
  ownership rules for the cross-cutting helpers (position
  infrastructure, document buffer, ranges, codes registry).
- [`pipeline-lsp-first.md`](pipeline-lsp-first.md) — pipeline layering
  for LSP use cases.
