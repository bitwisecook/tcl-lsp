# Luna-7 delta review: Sol-5 `same-document-order-v4.patch` (V3 → V4)

**Verdict: HOLD.** One item remains: the `Expr` literal exemption.

- A braced expression with no `[` can still call a user-defined math function. That function can end the load normally while skipping a later removal.
- tclsh 8.6 and 9.0 confirm it.
- Variable traces cannot cause this; they only produce errors.

Every other v3 item is resolved, and the stray `definition.rs.orig` is gone.

## Review basis

- **Patch:** sha256 `af4b5a45f70463a12a8723645ca613cc0340189bfe23d3305350def532e56ebc`.
- **Baseline:** the `a/` copies match HEAD `1433f2ffe`, and `git apply --check` passes.
- **Files:** 9, as `diff -ruN` sections. There is no `*.orig` or `*.rej` in `b/` and none in the patch.
- **Correction to my own v3 review:** I diffed only the source trees and did not catch the stray `definition.rs.orig` in the original v3 patch. The regenerated v3 and v4 do not contain it.
- **Delta reviewed:** `sol-5/v3/b` against `sol-5/b`. Only `source_graph.rs` and §8.1 changed. `namespace_import.rs`, `workspace_index.rs` and `definition.rs` are identical to v3.
- **Probes:** in `scratchpad/agents/luna-7/probe/v4/`. Each one loads, in that order, an import, the shape under test, a forget, and a flag recording that the end was reached. Each is run sourced directly and sourced inside `foreach`.

## Coverage of `may_complete_early` (`b/rust/tcl-lsp-core/src/source_graph.rs:~950-1003`)

| shape | v4 | tclsh (from v3) |
|---|---|---|
| `apply $l` | LambdaLiteral, not a literal, not retained, so the end is withheld | rc 0, forget skipped |
| `lsort -command cmp` | `invokes_prefix` is set because `authored_source_retains_callback()` is not `Some(false)` (option value role `CommandPrefix`), so the end is withheld | rc 0 |
| `lsort -command [list apply …]` | same | rc 0 |
| `expr $e` | Expr, dynamic (literal is `None`), not retained, so the end is withheld | rc 0 |
| `regsub -command` (Tcl 9) | `command_prefix_resolver`, which `authored_source_retains_callback` reads (`rust/tcl-registry/src/resolved_invocation.rs:3932-3972`), so the end is withheld. No role change is needed; the role-resolver follow-up is documented. | 9.0 rc 0 |
| roles unknown | withheld if any possible written role carries a script | — |
| no schema | `invokes_prefix` and `may_end_load` both true, so withheld | — |
| flow `Deferred` | skipped, correctly: the script is stored for later | — |

## Blocking: the `Expr` literal exemption is unsound (`b/source_graph.rs`, in `may_complete_early`)

The exemption reads: `role == ArgRole::Expr && !value.contains(&b'[')` means the operand "runs nothing". But a braced expression can call math functions, and math functions are ordinary Tcl commands under `::tcl::mathfunc`.

Results with `probe/v4/run.tcl` on tclsh 8.6 and 9.0, which agree:

| shape | rc (direct / inside `foreach`) | forget ran? | v4 at the end |
|---|---|---|---|
| `proc ::tcl::mathfunc::f {x} {return -code return}; expr {f(1)}` (m_ret) | 0 / 0 | no; alias alive | **revokes** |
| the same with `return -level 2` (m_lvl) | 0 / 0 | no | **revokes** |
| the same with `return -code break` or `-code continue` (m_brk, m_cont) | 3 or 4 / 0 inside a loop | no | **revokes** |
| `if {f(1)} {…}` (m_if; the condition is also an exempt Expr literal) | 0 / 0 | no | **revokes** |
| a function defined via `namespace eval ::tcl::mathfunc {proc g …}` (m_nsf) | 0 / 0 | no | **revokes** |
| a built-in name overridden in the same document: `rename ::tcl::mathfunc::abs …; proc ::tcl::mathfunc::abs …; expr {abs(1)}` (m_builtin) | 0 / 1 | no | **revokes** |
| an undefined function reaching a user `proc unknown {args} {return -code return}`: `expr {nosuchfn(1)}` (t_unknown) | 0 / 0 | no | **revokes** |

The same gap covers expressions that contain `[`. Their substitution regions are retained, so `retains` answers true, and the function calls beside them are never examined. Example: `expr {f(1) + [set x]}`.

### Variable traces: not a path, no change needed

On both versions, a read trace whose callback does `return -code return`, `-level 2` or `-code break` turns the read into an error ("can't read …"). That holds for scalar and array reads, and for both `expr {$x}` and `set y $x` (probes t_read, t_readlvl, t_readbrk, t_array, t_readset). A write trace behaves the same way (v3 probe x_trace).

So a braced expression that reads variables cannot end the load normally. Excluding expressions with variable reads is unnecessary, and would also be pointless: any `$x` in any command can fire a trace.

### Required fix (registry and shared-owner data, end-only)

For **every** `Expr` operand, whether exempt or retained, withhold the end when the expression parsed by the shared expr owner (`tcl_syntax::expr`, which has a `Call` node at `ast.rs:415`) contains any of the following:

- a function call whose name is not in the dialect's built-in math-function catalogue (`tcl_lexer::expr_lexer::math_functions`, `rust/tcl-lexer/src/expr_lexer.rs:321`, which `tcl_syntax::expr::operators` guards against drift);
- a call to a name this document declares: a procedure or class with that simple name, such as `proc ::tcl::mathfunc::abs`;
- an expression that does not parse.

A simpler alternative that is also acceptable is to withhold the end for any `Call` node at all.

Add m_ret, m_if and t_unknown to `document_end_is_after_normal_completion_only`.

A built-in math function overridden from another document belongs with the cross-document hijack residual. Add it to §8.1 "Known limits".

None of this affects test-400, test-404 or test-406, which contain no expressions. The guard test's `if {$x} {set branch 3}` keeps its `END` assertion, because it contains no call.

## Docs

§8.1 states that "a braced expression without command substitution run[s] nothing". That is false, per the probes above. Correct it together with the fix.

## Unchanged from v3

- Positioned queries remain sound, and so does the deferred marking.
- `rename myeval eval` still relies on the source-transition-tape barrier.
- Still open:
  - the proof gate, owner-resolution and clippy have not been run;
  - timing is still needed (the scaling test under 2 s, test-172, large-corpus references);
  - sign-off for the second and third 1b45c540e controls.
