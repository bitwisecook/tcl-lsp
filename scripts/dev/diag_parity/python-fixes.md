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

**Status (2026-06-18):** implemented in **PR #640** (Python `tcl_expr_eval.py`).
But Codex review on #640 (and a tclsh check) found *both* sides over-coerced:
the numeric-vs-string decision must follow Tcl's equality conversion rules, not
a general literal parser. **Boolean words are not numbers for `==`** (tclsh:
`expr {"true" == "1"}` → 0, a string compare; `expr {"true" + 0}` errors), and
leading-zero `08` is **dialect-dependent** (`"08" == "8"` → 0 in tcl8.x as an
invalid octal, → 1 in tcl9.0 as decimal). The Rust side is now fixed to use a
strict number grammar (no boolean coercion) in `compare_numeric`
(`tcl_expr_eval.rs`), so `"true"=="1"`→0 / `"5"=="5.0"`→1 / `"foo"=="bar"`→0
match tclsh. The Python #640 fix needs the same strict eligibility (Codex P2 on
`tcl_expr_eval.py`). The `08` dialect nuance is now **fixed on the Rust side
and outstanding on Python** — see section 3.

---

## 2. iRules `when` body recursed under a non-iRules dialect

**Symptom (MISSING_FIRE in Rust under tcl8.6 — but Rust is the correct side):**

```tcl
# analysed under --dialect tcl8.6
when HTTP_REQUEST {
  set uri [HTTP::uri]
  log local0. "got $uri"
}
```

Python (tcl8.6) fires W123/W002 on `log` and `HTTP::uri` *inside* the brace;
Rust does not.

**Why Rust is correct:** `when` is an iRules-only builtin. Under a plain-tcl
dialect it is an unknown command that, if it ran at all, would have to be
user-defined — and a user command's braced `{…}` argument is just a string,
not a script the analyser may assume is executed. Python resolves arg roles
from its dialect-agnostic registry, so it applies the iRules `when` BODY role
even under tcl8.6 and recurses into the brace as if it were an event handler —
leaking iRules semantics into non-iRules analysis. (It also mis-lowers the
body, yielding a spurious W210 "read before set" for `uri`, which *is* set
before it is read.)

**Fix (Python, optional / low priority):** when the active dialect does not
enable a command, do not apply that command's body/arg roles — treat its
braced arguments as opaque strings (matching what tcl would do). Equivalent
to gating `_recurse_body_arguments` on dialect command status. This is a
degenerate "wrong dialect" scenario (real iRules analysis uses
`--dialect f5-irules`, where Rust and Python agree), so it is low priority;
the parity harness now matches dialect to file type and no longer flags it.

**Do NOT** make Rust recurse iRules bodies under tcl dialects to match Python.

---

## 3. Leading-zero `==` / `!=` constant-fold ignores the dialect's octal rule

**Symptom (WRONG_MESSAGE — Rust is the correct side):**

```tcl
# analysed under --dialect tcl8.6
set x 08
if {$x == 8} { puts a } else { puts b }
```

- tclsh8.6: `expr {"08" == 8}` → **0** (`08` is an *invalid* octal, so the
  operands compare as strings: `"08" != "8"`). The `if` always takes the
  `else` branch → I230 "always **false**".
- tclsh9.0: `expr {"08" == 8}` → **1** (TIP 472 dropped the octal rule; `08`
  is decimal 8). I230 "always **true**".

Rust now folds both correctly per dialect. **Python folds it to "always true"
under *every* dialect** — it reads `08` as decimal 8 regardless of the active
dialect, so its tcl8.x answer is wrong (and disagrees with tclsh).

**Root cause (Python):** Python's `_parse_literal_value` already preserves the
string identity of a non-round-tripping literal (`"08"` stays the string
`"08"`, not int 8), so the *value* survives — but `tcl_expr_eval.py`'s
`==`/`!=` numeric-eligibility check is **dialect-unaware**: it treats `"08"` as
a number unconditionally instead of consulting the dialect's leading-zero rule
(octal in tcl8.4/8.5/8.6 and the 8.x-derived F5/EDA dialects, where `08`/`09`
are invalid octal → string; decimal only in tcl9.0).

**Fix (Python):** thread the analysis dialect into the constant-condition
evaluator and classify a bare leading-zero operand (`08`, `010`) per dialect:
in 8.x, a *valid* octal (`010` → 8) is a number and an *invalid* one
(`08`/`09`) is a string; in 9.0 it is plain decimal. The Rust implementation
is `tcl_expr_eval.rs::classify_operand` + `parse_octal_literal`, gated by the
registry's `leading_zero_is_octal()` (true for every dialect whose registry
did not load the `tcl9.0` version bit). Verify against tclsh8.6 / tclsh9.0
that Python's I230 direction then matches both.

**Do NOT** revert Rust to the dialect-unaware decimal reading — Rust matches
tclsh per dialect; Python is the side that is wrong. (This divergence is kept
out of the parity corpus on purpose: the corpus oracle treats Python as ground
truth, and adding a case where Python is known-wrong would register a spurious
"defect".)
