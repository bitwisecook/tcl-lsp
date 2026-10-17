# Sol-5 v5: same-document source order, after Luna-7's v4 HOLD

**Patch:** `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-5/same-document-order-v5.patch`
- sha256 `998b5bca9efc981461a131621b98d6bc59148b4a0a6e58305621b926fdf3dfce`.
- 9 files, built with `diff -ruN a b`. `a/` is byte-identical to HEAD `1433f2ffe`.
- `git apply --check` passes against the working tree and with `--cached`.
- `rustfmt --edition 2024 --check` is clean on all four Rust files.
- No `#[allow]` was added, and there is no stray `.orig` or `.rej` file in `b/` or in the patch.
- **Static only:** nothing was compiled or run. The proof gate, `xtask owner-resolution` and clippy are unverified.

Everything except the `Expr` rule is unchanged from v4. See NOTES-v4.md for that, and NOTES-v3.md and NOTES-v2.md before it.

## The blocking item: math functions are commands

`SequenceCapture::may_complete_early` is now a method. It handles every `Expr` operand, whether retained or not, through a new `expression_may_complete_early`:
- **Not a literal word.** An unbraced `$e`, or a word with substitutions, withholds the end, because it is substituted twice.
- **Literal word.** The shared expression owner parses it under the input's analyser profile, via `tcl_syntax::expr::parser::parse_expr_for_profile`.
  - If the tree contains any function call (`ExprNode::Call`), the end is withheld.
  - If parsing fails (`ExprNode::Raw`), the end is withheld.
  - Calls are found by walking `evaluation_nodes()`.
  - There is no built-in allowlist: *every* call counts. That covers a user `::tcl::mathfunc` procedure, a built-in overridden in the same document or in another one, and an undefined name reaching a user `unknown`.
- **Literal containing `[`.** It must additionally have a retained region, as before.
- **Bodies.** The empty-literal exemption remains only for non-`Expr` bodies.
- **Variable reads.** Nothing is excluded. Luna-7 verified that a trace callback's non-OK return completes as an error.

Two consequential changes:
- `record_exits` now takes the precomputed `completes_early` flag. The capture loop computes it with the capture context.
- The positional cutoff is unchanged.

**Controls added to `document_end_is_after_normal_completion_only`:**
- `proc ::tcl::mathfunc::f {x} {return -code return}; expr {f(1)}`
- the same procedure with `if {f(1)} {set y 1}`
- `proc unknown {args} {return -code return}; expr {nosuchfn(1)}`

In each one, `set before`→`set after` stays `KNOWN`, and both points are not `KNOWN` against the document end.

**Unaffected tests:**
- Tests 400, 404 and 406 contain no expressions.
- The guard test's `if {$x} {set branch 3}` parses as a `Var` with no call, so its `END` assertion is unchanged.

## Docs (§8.1)

- **Corrected sentence:** "An empty literal body runs nothing." It is followed by the shared-parser rule, the statement that any call or parse failure (and any non-literal operand) withholds the end, and the note that variable reads need no exclusion.
- **Known limits:** now also lists a built-in math function overridden under `::tcl::mathfunc` by another document. It notes that every call already withholds the end, so no allowlist depends on it.
- **Controls list:** extended with the three new shapes.
- **Doc comment:** the `DocumentSequence` comment mentions expression calls.

## Expected flips and risks

These are unchanged from v3 and v4.
- Tests 400, 404 and 406 flip, provided `Selected` words come back inside root `namespace eval` bodies.
- Pass→fail is limited to the three 1b45c540e controls. The second and third still need sign-off.

What the new rule changes:
- Any top-level expression that calls a function now withholds the document end, including a built-in call such as `expr {abs($x)}`.
- This only affects end queries, and no production consumer issues one.

**Timing:** the capture now also parses each `Expr` operand outside definition bodies, using the uncached profile parser. Timing is still needed for:
- `settling_invocations_does_not_scale_with_imports_times_exports` (< 2 s);
- test-172;
- references on a large corpus.
