# Contract: the numeric tower & `expr` semantics

> **Status:** First-principles design contract (v2 / "if starting over").
> The numeric value model and the `expr` language a from-scratch runtime must
> fix *before* any arithmetic command exists. Related as-built notes:
> [shimmer-reference-behaviour.md](shimmer-reference-behaviour.md) and the
> bignum/arith modules in the Rust runtime (`runtime/rust/src/bignum.rs`,
> `expr.rs`).

## Two coupled decisions

`expr` is a **separate language** (its own grammar, precedence, operators,
and function namespace), and Tcl integers are **transparently arbitrary
precision**. These are coupled: the promotion rules of the numeric tower are
exactly what `expr`'s operators must implement. Decide both once, centrally,
and route *every* numeric consumer through them — `expr`, `tcl::mathop::*`,
`incr`, `format`/`scan`, `string repeat`, list indices, comparisons, dict
keys, hashing. The chronic failure mode is one consumer rolling its own
integer parse and silently truncating beyond 64 bits, or an `i32`/`i64` cast
panicking on a promoted value (a real bug class in this repo).

## The tower

```
immediate small int   →   wide (i64)   →   bignum (arbitrary precision)   →   double (f64)
        ▲ same logical integer, widened only as needed ▲                    (a distinct type)
```

* **Integers are one logical type** that widens through small→wide→bignum as
  magnitude demands. Every integer op overflow-checks and **promotes**; it
  never wraps. (`expr {2**200}`, factorials, `incr` past 2⁶³ all just work.)
* **Canonicalise downward.** A bignum that fits in a wide is demoted back to
  a wide (and a wide that fits the immediate tag to an immediate). This keeps
  equality, hashing, and the string rep stable and is **observable** — C Tcl
  does it and tests depend on it. Normalisation happens at the *end* of every
  producing operation.
* **`double` is a separate type**, not a wider integer. Mixed int/float ops
  promote the int operand to double; the result is double. `wide`↔`double`
  conversions lose precision deliberately per IEEE-754.
* **String is the lazy fourth rep**: a numeric value carries a cached string
  rep generated on demand (`1e100`, `0xff`, the canonical decimal). The
  parse direction (string → number) and the format direction (number →
  string) must round-trip exactly for the shortest-decimal contract below.

This is the numeric facet of the object model — the typed internal rep plus a
lazily generated string, *not* a string with a parse on every use.

## `expr` is its own language

A separate lexer/parser/evaluator with its own:

* **Grammar & precedence** (high → low): `**` (right-assoc) ; unary `- + ~ !`
  ; `* / %` ; `+ -` ; `<< >>` ; `< > <= >=` ; `== !=` and `eq ne` ; `in ni`
  ; `&` ; `^` ; `|` ; `&&` ; `||` ; `?:` (right-assoc). Get the **`**`
  right-associativity** and the placement of `eq/ne/in/ni` right.
* **Operators string code lacks:** `eq`/`ne` (always string compare),
  `in`/`ni` (list membership), `?:` ternary, short-circuit `&&`/`||`
  (the un-taken branch is **not** evaluated — observable via side effects).
* **Functions via a namespace:** `sin(x)`, `max(...)`, `rand()` dispatch to
  `::tcl::mathfunc::*`. Users can **define or override** them, so function
  dispatch is the namespace command table, not a builtin switch. `tcl::mathop`
  exposes the operators as commands (`+ - * ...`) for `{*}`-style use.

  The dispatch is **caller-namespace-relative, then global**, and the two
  spellings are *not* interchangeable — both verified against tclsh 8.6.16 and
  9.0.4:

  ```tcl
  namespace eval ::tcl::mathfunc { proc g {x} { expr {$x + 1000} } }
  namespace eval ::foo {
      namespace eval tcl::mathfunc { proc f {x} { expr {$x * 100} } }
      proc use  {} { expr {f(2)} }   ;# 200  -> ::foo::tcl::mathfunc::f
      proc useg {} { expr {g(2)} }   ;# 1002 -> ::tcl::mathfunc::g (global)
  }
  proc f {x} { return GLOBAL-PROC-f } ;# never reached from expr
  li {10 20 30} 1                     ;# invalid command name "li"
  ```

  So an ordinary global `proc` of the same bare name never enters the
  resolution, and conversely a proc living in a `tcl::mathfunc` namespace is
  **not** reachable as a bare command — only through `expr`'s function-call
  production or its own fully-qualified name.

  Two availability axes follow from that and must be kept apart: the *`expr`
  grammar* axis (is `NAME(…)` a built-in function here — `abs(…)` is 8.4) and
  the *command table* axis (does the command `::tcl::mathfunc::NAME` exist here
  — the table itself is TIP 232, so 8.5+, even for `abs`). Both live in
  `rust/tcl-registry/src/mathfunc.rs`, with
  `rust/tcl-syntax/src/expr/mathfunc.rs` as the layer-1 fact table it reads;
  no consumer re-derives the `tcl::mathfunc` prefix or a version ceiling for
  itself.
* **Literals:** `0x`/`0o`/`0b` radices, `_` digit separators (`1_000`),
  decimal/scientific floats, `Inf`/`NaN` (case-insensitive, with the
  `NaN(hex)` payload form for `binary`/round-trip), booleans `true/false/
  yes/no/on/off`.

### The braced vs. unbraced split (compile/interpret + security)

* `expr {…}` — the braced body is parsed once and **compiled** to guarded
  native arithmetic. This is the safe, fast form.
* `expr $x …` — the operands are substituted **first**, then the *result* is
  re-parsed as an expression. This double-substitution is both a performance
  cliff and an **injection risk** (`expr "$user + 1"`); it must be
  interpreted at runtime. Lint/diagnostics should steer users to braces, but
  the runtime must implement both.

## Numeric semantics that are tested verbatim

* **Integer `/` truncates toward −∞? No — toward zero is *not* it either.**
  Tcl integer division and `%` are defined so that `%` has the sign of the
  **divisor** and `a == (a/b)*b + (a%b)`. Pin the exact rounding/sign rule.
* **`/` on a float operand is float division;** all-integer `/` is integer.
* **Bit operators (`& | ^ ~ << >>`) are integer-only** (incl. bignum); a
  float operand is an error. Shift counts and huge shifts have defined
  behaviour through bignum.
* **Overflow promotes to bignum** (never wraps) — contract.
* **Comparisons:** `==`/`!=` compare numerically when both sides look
  numeric, else as strings; `eq`/`ne` always string. `in`/`ni` use list
  membership with string equality.
* **Float → string** uses the **shortest decimal that round-trips**
  (C Tcl 9's `tcl_precision`-free default); `format`/`%g`/`%.17g` and the
  default rep must match byte-for-byte, including `-0.0`, `Inf`, `NaN`.
* **Errors** are verbatim: `divide by zero`, `domain error`,
  `can't use non-numeric string as operand`, `too many digits` (charset
  parse), etc.

## Contract vs. incompatible-by-design

| Behaviour | Class |
|---|---|
| Integer overflow → bignum (no wrap), canonical demote-when-fits | **Contract** |
| `%`/`/` sign and rounding rules; bit-ops integer-only | **Contract** |
| `**` right-assoc; full precedence table; short-circuit `&&`/`||` | **Contract** |
| `mathfunc`/`mathop` dispatch through the namespace (overridable) | **Contract** |
| Literal radices, `_` separators, `Inf`/`NaN`/`NaN(hex)`, booleans | **Contract** |
| Shortest-round-trip float formatting, `-0.0`/`Inf`/`NaN` text | **Contract** |
| `eq`/`ne`/`in`/`ni`, numeric-vs-string `==` rule | **Contract** |
| Error wording (`divide by zero`, `domain error`, …) | **Contract** |
| Which internal rep a value currently holds (`tcl::unsupported::representation`) | **Incompatible-by-design** — shimmering is observable but rep identity is not a from-scratch contract (W9-internal). |
| Bignum limb layout / refcounts via test hooks | **Incompatible-by-design** |

## AOT compiler implications

* Compile braced `expr` to native ops with a **type guard ladder**: try the
  immediate/wide fast path, fall to bignum on overflow, to double on float
  operands — the same ladder the tower defines. Never emit a wide-only op
  without the overflow→bignum escape.
* Constant-fold only when the fold uses the *exact* tower semantics
  (including canonicalisation), or the compiled and interpreted results
  diverge.
* Route `mathfunc` calls through the command table so user overrides win,
  even in compiled `expr`.

## See also

- [shimmer-reference-behaviour.md](shimmer-reference-behaviour.md) — how the
  number↔string boundary is observed.
- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
  [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — the other two day-one contracts.
