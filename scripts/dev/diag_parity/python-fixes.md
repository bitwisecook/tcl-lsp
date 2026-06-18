# Python-side fixes (where Rust is *more* correct than Python)

The repo's parity bar is "match the Python CLI byte-for-byte". But the parity
harness (`scripts/dev/diag_parity/`) also surfaces cases where the **Rust**
analyser is the more correct of the two — Rust emits a true-positive
diagnostic that Python misses, or Rust positions/folds something Python gets
wrong. The standing rule for these is: **do not regress Rust to match Python.
Fix Python instead**, and document the Python bug here (the same precedent as
the Python-side bugs already noted in `docs/rust-cli-port.md`).

Each entry below is a self-contained prompt for a Python-side change.

---

## 1. I230 — constant-condition fold misses `==` / `!=` on string operands

**Symptom (EXTRA_FIRE in the harness, Rust-only):**

```tcl
set x foo
if {$x == "foo"} { puts hi }   ;# Rust: I230 "always true"; Python: nothing
if {$x eq "foo"} { puts hi }   ;# both: I230 "always true"
```

With `x` provably `"foo"`, `$x == "foo"` is always true, so I230 (constant
branch condition; the alternate branch is unreachable) is a **true positive**.
Rust fires it; Python fires it for `eq` but **not** for `==`.

**Root cause (Python):** the constant-condition evaluator folds `eq`/`ne`
(structured `ExprBinary` STR_EQ/STR_NE path) but the `==`/`!=` path
(`BinOp.EQ`/`BinOp.NE`) attempts a *numeric* comparison and bails on
non-numeric operands instead of falling through to the string comparison.
The string-aware fallback already exists and handles both operators
identically — `compiler/core_analyses.py:918`:

```python
if op in ("==", "eq"):
    return lv_val == rhs_val
if op in ("!=", "ne"):
    return lv_val != rhs_val
```

— but the structured `==`/`!=` arm returns `None` (or an UNKNOWN lattice
value) for two string constants before that fallback is reached, so I230 never
fires. Tcl semantics: `expr {"foo" == "foo"}` → `1`; the operands are compared
as strings when neither is numeric.

**Fix:** make the structured constant-fold for `BinOp.EQ`/`BinOp.NE` compare
string-valued constants (mirroring the existing STR_EQ/STR_NE handling) when
both operands fold to constants and at least one is non-numeric, so
`_evaluate_constant_condition` returns a definite bool. Verify with the two
snippets above that Python's I230 then matches Rust (fires for both `==` and
`eq`), and re-run the parity harness to confirm the I230 EXTRA_FIRE divergence
clears.

**Do NOT** suppress Rust's I230 for `==` — Rust is correct here.
