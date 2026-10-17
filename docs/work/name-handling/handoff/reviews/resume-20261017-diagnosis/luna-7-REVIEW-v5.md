# Luna-7 delta review: Sol-5 `same-document-order-v5.patch` (V4 → V5)

**Verdict: ADMIT** (static review). The v4 blocking item is resolved, and I found no new issues.

The remaining conditions are not code defects. They are listed under "Conditions" at the end.

## Review basis

- **Patch:** sha256 `998b5bca9efc981461a131621b98d6bc59148b4a0a6e58305621b926fdf3dfce`.
- **Baseline:** 9 files, and every `a/` copy matches HEAD `1433f2ffe`. `git apply --check` passes.
- **No stray files:** there are no `*.orig` or `*.rej` files in `b/` or in the patch.
- **Delta:** relative to v4, only `source_graph.rs` and §8.1 changed. `namespace_import.rs`, `workspace_index.rs` and `definition.rs` are identical to v4.

## Checks

1. **Shared parser call** (`b/rust/tcl-lsp-core/src/source_graph.rs`, `expression_may_complete_early`): correct.
   - It calls `tcl_syntax::expr::parser::parse_expr_for_profile(text, Some(self.input.analyser_profile()))`.
   - That is the shared expression owner (`rust/tcl-syntax/src/expr/parser.rs:678`), and the function is the profile-respecting entry point.
   - `ResolvedAnalysisInput::analyser_profile` returns `&'static DialectProfile` (`rust/tcl-compiler/src/analyser/input.rs:100`), so the dialect is the one the analysis used.
   - A dialect difference can only turn a parse into `Raw`, which withholds the end, so the error direction is conservative.
2. **Every call is found.**
   - `evaluation_nodes()` (`rust/tcl-syntax/src/expr/ast.rs:637-662`) walks `Binary`, `Unary`, `Ternary` and `Call` arguments.
   - The remaining variants are leaves: `Literal`, `String`, `CompiledWord`, `Var`, `Command` and `Raw`.
   - Any `Call` withholds the end, and so does `Raw`, the parser's give-up fallback.
3. **Non-literal `Expr` words withhold the end.** When the literal is `None`, or is not valid UTF-8, the function returns `true`. That covers `expr $e` and `if $c`.
4. **A retained region is still required for `[`.** The `Expr` branch also returns true when the literal contains `[` and no region is retained. Commands inside a retained region are examined by `record_exits`. The empty-body exemption now applies only to non-`Expr` roles.
5. **The new controls match my v4 probes:**
   - `expr {f(1)}` with `proc ::tcl::mathfunc::f {x} {return -code return}` (my m_ret);
   - `if {f(1)} {…}` (my m_if);
   - `proc unknown {args} {return -code return}` with `expr {nosuchfn(1)}` (my t_unknown).

   In each, the ordering of the two statements stays known: `if` has a retained `Body` and `expr` has no `Body`, so the positional cutoff is not triggered. Only the document end is withheld.

   The guard test's `if {$x}` parses as a `Var`, so its `END` assertion still holds. Tests 400, 404 and 406 contain no expressions.
6. **§8.1 wording is accurate.**
   - It drops the false claim that braced expressions run nothing.
   - It states the parse rule: any call, a parse failure or a non-literal operand withholds the end.
   - It says variable reads need no exclusion, which my v4 trace probes confirmed.
   - It extends "Known limits" with a built-in math function overridden from another document, and correctly notes that no allowlist depends on it.
   - The controls list includes the three new shapes.
7. **Rules:** no `#[allow]` and no command-name matching. `may_complete_early` now takes `&self` and uses it.

## Conditions (unchanged, not code defects)

- Run the proof gate, `xtask owner-resolution` and clippy.
- Measure timing for:
  - `settling_invocations_does_not_scale_with_imports_times_exports` (< 2 s);
  - test-172;
  - references on a large corpus.

  The capture now also parses every `Expr` operand outside definition bodies, with the uncached parser.
- Get sign-off for the second and third 1b45c540e controls.
- In the capture, confirm that tests 400, 404 and 406 flip. That depends on `Selected` words being returned inside root `namespace eval` bodies. If they are not, the failure is closed and no answer is wrong.
