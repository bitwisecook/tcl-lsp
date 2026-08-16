# Contract: the numeric tower & `expr` semantics

> The numeric value model and the `expr` language every arithmetic consumer in
> the stack routes through. Related as-built notes:
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
panicking on a promoted value.

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

## The numeral grammar is release-dependent

Everything above — the tower, the promotion rules, the operators — is
release-invariant. **How a numeral is spelled is not.** This is the one axis of
the numeric contract that is keyed to the Tcl release, and it is the axis that
has repeatedly caused silent wrong answers, because most numerals read the same
under every grammar and only a few disagree.

| spelling | 8.4 | 8.5 / 8.6 | 9.0+ |
|---|---|---|---|
| `0x1f` hex | ✓ | ✓ | ✓ |
| `0b1011` binary, `0o17` octal | — bareword | ✓ | ✓ |
| `0d99` decimal | — bareword | — bareword | ✓ |
| `1_000` digit separators | — bareword | — bareword | ✓ |
| `0755` leading zero | **octal → 493** | **octal → 493** | **decimal → 755** |
| `08` leading zero | — invalid octal | — invalid octal | ✓ 8 |

The leading-zero row is the dangerous one: the same text is a *valid number
with a different value*, so nothing errors — the answer is just wrong. C settles
it at build time in `generic/tclStrToD.c`, which carries `#undef KILL_OCTAL` up
to 8.6 (the comment reads "Define `KILL_OCTAL` to suppress interpretation of
numbers with leading zero as octal") and defines it from 9.0. `changes.md` for
9.0 records: "`0NNN` format is no longer octal interpretation. Use `0oNNN`."

Consequences that are *not* obvious, and are pinned by tests:

* **A level word inherits it.** `upvar`/`uplevel` read their level through
  `Tcl_GetIntFromObj`, so `uplevel 010` goes **8 frames up on 8.6 and 10 on
  9.0**, and `uplevel 0d1`/`1_0` error before 9.0. Any behaviour reached through
  an integer conversion inherits the grammar the same way — completion codes
  (`return -code 010`), list indices, `incr` amounts.
* **`bad level` is ambiguous.** C reports it both for a word it cannot parse
  and for a level past the top of the stack. Any experiment comparing releases
  needs a call chain deeper than the largest value under test, or a divergence
  hides as a shared error.

### The release also moves the lexeme *boundary*

The grammar decides what a numeral **means**; it also decides where a numeral
**ends**, and that is a separate fact that has to be settled in the lexer —
once a boundary is drawn, no later pass can redraw it. C draws it in
`ParseLexeme` (`generic/tclCompExpr.c`; 8.4 in `tclParseExpr.c`), and three
rules there are release-sensitive or easy to get subtly wrong. All of the
below is pinned against tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1b0.

**1. `_` never starts a lexeme — on any release.** The last gate in
`ParseLexeme` is `if (!TclIsBareword(*start) || *start == '_')`, yielding
`INVALID`, not `BAREWORD`; 8.4 gets there differently (it requires `isalpha`
to start a lexeme at all) and agrees. So `_x` is `invalid character "_"`,
never `invalid bareword "_x"`, while `abc_1` and `a__b` are ordinary
barewords. The C source is blunt about why: *"We reject leading underscores in
bareword. No sensible reason why."*

**2. Which word operators exist moves the boundary.** `eq`/`ne` are 8.4+,
`in`/`ni` are 8.5+ (TIP 201), `lt`/`le`/`gt`/`ge` are 9.0+ (TIP 461). A
spelling the release does not have is not a lexeme, so the bareword scan
swallows it whole:

| input | 8.6 | 9.0 |
|---|---|---|
| `1 lt_ 2` | `invalid bareword "lt_"` | `invalid character "_"` |
| `1 eq_ 2` | `invalid character "_"` | `invalid character "_"` |

This is why the operator set lives in `tcl_dialect::EXPR_WORD_OPERATORS`,
below the lexer, rather than in `tcl-syntax` alone — and why a drift guard
ties it to `OperatorSpec::expr_grammar_min_version`. Note C guards only the
*trailing* side of a word operator, with `isalpha` and not `TclIsBareword`:
`1 eq2` really is `1 eq 2`.

**The gate applies only where it moves a boundary.** When a word operator is
followed by whitespace — `$a lt $b`, the shape real code takes — the token
boundary is identical either way, and the version check is settled *above* the
lexer: the parser rejects a `lt` the dialect lacks and reports
`invalid bareword "lt"`, exactly as tclsh does. So the lexer still emits the
operator token there, and only withholds it when a bareword byte (`_` or a
digit) follows and the run would otherwise fuse. This is not merely an
optimisation: **W003 exists to say "this operator is Tcl 9.0+ (TIP 461)", and
it can only say that about an operator it can still see.** Gating the token
unconditionally silenced the diagnostic on precisely the dialects it targets —
nine e2e tests and the iRules `in`/`ni` suite caught it. A diagnostic that
reports unavailability needs the *maximal* vocabulary; only the boundary needs
the release's actual one.

**3. A number against barewords is one bareword.** When `TclParseNumber`
stops on a `TclIsBareword` byte, C rescans `[A-Za-z0-9_]*` from the start as a
single `BAREWORD` — `1_eq`, `1abc`, `12x`, `1e_0`, `9_ne` all report whole.
Two exceptions put it back on the number path: a non-bareword character in the
run (so `1.5abc` names only `abc`, and 8.6's `1.0_2` is `invalid character
"_"` — the `.` is what splits these from `1_0`, which is one bareword), and a
following binary word operator (`1eq 2` is `1 eq 2`).

The through-line: **abstain toward keeping the lexeme whole.** An unversioned
dialect takes the newest grammar, like every other `LexerGrammar` knob — the
operator set only ever grows, and inventing a split for a release that has no
such operator reports a boundary that exists nowhere.

The lower owner of those boundaries is
`tcl_dialect::scan_expr_number` (`rust/tcl-dialect/src/expr_number.rs`). It
takes the paired `NumberSyntax` and expression-word grammar, returns an
exclusive byte end for a numeric candidate, and deliberately does not decide
whether that candidate has a value. `tcl-lexer` uses it to create a `NUMBER`
token; `tcl_syntax::number::is_expr_number` uses it again before its
`TclParseNumber` port validates the complete spelling. This preserves the
dependency direction (`tcl-dialect` below lexer and syntax) while ensuring a
bad `0o8`, `1_eq`, `12x`, fractional separator, radix run, or `Inf`/`NaN`
spelling cannot acquire separate boundaries in the two consumers. The scanner
is source-derived from `ParseLexeme` in Tcl 8.4, 8.5, 8.6, 9.0, and 9.1; its
mutation-sensitive rows are oracle-checked with tclsh 8.6.17 and 9.0.3.

An explicit radix prefix has a real numeric boundary only after an available
prefix and at least one radix-valid digit. Then the shared word-operator probe
applies: `0b1ne 1` is `0b1` / `ne` / `1` on 8.5+, and `0d9lt 10` is
`0d9` / `lt` / `10` from 9.0. `0o8ne` and a pre-9.0 `0d9ne` instead remain
whole invalid bareword candidates. The release table is
`NumberSyntax::explicit_radix`, shared by the value parser and lower scanner.

Tcl 8.4 is intentionally a separate branch in that owner. Its `GetLexeme`
first accepts the prefix returned by `TclParseInteger` and never executes the
8.5+ bareword rescan: `1_eq` is literal `1` then invalid `_`, `12x` is `12`
then `x`, and `0x1p2` is `0x1` then `p2`. For 8.5+, the same lower owner also
owns `NaN(...)`: `TclParseNumber` accepts ASCII whitespace among one through
thirteen hex digits, but a fourteenth digit invalidates the parenthesised form
instead of truncating it. `tcl_syntax::number` calls `scan_nan_payload` for the
decoded value, so tokenization and semantic validation share that ceiling.

### One facility, dialect-parameterised

`rust/tcl-syntax/src/number.rs` is the **only** numeral value parser in the
workspace. It takes a `NumberSyntax` (`Tcl84` / `Tcl85` / `Tcl90`) and
implements the table above. The expression-specific byte boundary is its
separate, lower `tcl_dialect::scan_expr_number` owner; every other consumer
asks one of those two facilities, never re-derives a prefix or boundary.

How the grammar reaches a consumer depends on what the consumer is:

* **Compiler, analyser, LSP, codegen — pass it down.** The dialect is a
  top-level property of the compile: `Module.dialect` → `CodegenCtx.numbers`,
  and analysis passes take it as a parameter. These tools handle documents of
  *different* dialects in one process, so ambient state would let one
  document's grammar contaminate another's results.
* **A runtime — install it once.** A runtime is *built for* one release and
  does not switch mid-execution, exactly as C's build-time `#define` does, so
  the interpreter installs the ambient grammar
  (`number::set_runtime_syntax`) when its release is pinned and every
  `ParseFlags::default()` call site inside it becomes correct at once.
  Switching it mid-run is unsupported: values already converted keep the
  numbers they were read as, so a flip would make one script text mean two
  things.

Codegen decides only **whether a literal is a numeral at all** under the target
release. It does *not* fold one: C's `CompileExprTree` pushes the source
spelling and lets the root's `tryCvtToNumeric` convert it in a runtime built for
the same release (`expr {0xFF}` compiles to `push "0xFF"; tryCvtToNumeric`).
Only a compound constant subtree folds (`expr {1+1}` → `push "2"`). Folding a
lone literal gets the right answer with the wrong bytecode.

### When no release is in hand: abstain, don't guess

Some consumers genuinely cannot name a release — a `ConstFoldFn` carries no
version, an optimiser gate runs before a target is fixed, a registry predicate
answers a shape question about a bare word. The rule there is **unanimity**:
resolve under every grammar and answer only if they agree.

Which direction "abstain" points depends on what the caller decides, and getting
that backwards is the trap:

| consumer | decides | abstains by |
|---|---|---|
| `FrameLevel::parse` | which frame a level word names | `Dynamic` — real frame, unknown which |
| `const_fold::parse_index` | whether to fold `lindex` | declining to fold |
| `node_provably_numeric` | may a numeric rewrite fire | requiring **every** release to agree it is a number |
| `is_numeric_or_boolean_string` | may a string promotion fire | accepting if **any** release reads a number (blocks the rewrite) |
| `DefaultFormFirstWord::matches` | which argument form was used | reporting no match |
| the package-resolver guard | is a branch reachable | `Guard::Unknown` |

The last two rows of the middle pair are the same predicate pointing opposite
ways: one claims "this **is** a number" and so needs proof under every release;
the other refuses an optimisation because a value "could still be a number" and
so needs only one release to say yes. Both were previously the ambient grammar,
which is neither, and under 9.0 rules `08` reads as numeric — so an
arithmetic-identity rewrite could fire on an operand that is a `bad octal` error
on an 8.x target.

Abstaining is cheap: an unfolded expression is evaluated later by something that
*does* know its release. Guessing is not — it bakes one release's answer into a
program built for another.

`cargo xtask number-drift` enforces both ownership rules: it fails on
hand-rolled radix-prefix recognition outside the value parser, and it checks
that the lexer and syntax classifier call the one lower expression-boundary
scanner rather than declaring another `scan_expr_number`; it applies the same
check to the shared NaN-payload scanner. It also requires the named
mutation-sensitive owner/consumer regression rows (legacy 8.4, radix/operator,
and NaN payload), but this is wiring evidence rather than a proof that arbitrary
source text cannot duplicate semantics; code review and those executed tests
remain the semantic backstop. Parsing a prefix out
of something that is *not* Tcl script text — a packet field, a hex colour, a
hex-encoded byte string — is legitimate and carries a `// number-drift-ok:`
waiver. The value-parser gate exists because the rule was broken six independent
times, each with the identical shape: strip two characters, call
`from_str_radix`.

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
| `Inf`/`NaN`/`NaN(hex)`, booleans | **Contract** |
| Literal radices, `_` separators, leading-zero octal | **Contract, keyed to the release** — see [the numeral grammar](#the-numeral-grammar-is-release-dependent). Not invariant: `0755` is 493 up to 8.6 and 755 from 9.0. |
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
  (including canonicalisation) **and the target release's numeral grammar**, or
  the compiled and interpreted results diverge. `expr {0755 + 1}` folds to 494
  for an 8.6 target and 756 for a 9.0 one.
* Route `mathfunc` calls through the command table so user overrides win,
  even in compiled `expr`.
* Emit an operator only when the **target release's `expr` grammar has it**.
  The opcode tables are total — every `BinOp` maps to an instruction — so
  nothing stops a compile for 8.4 from emitting `expon` or `strLt` and the
  emulating VM from executing it, while the same source reaching the
  interpreted `exprStk` path is rejected by
  `RuntimeExprSurface::validate`. Codegen asks the same registry surface
  before specialising and, when the answer is no, refuses to specialise:
  the whole expression falls back to `exprStk` and the engines' one gate
  produces C Tcl's own diagnostic. Constant folding is downstream of that
  check, or `expr {2 ** 3}` folds to `8` for an 8.4 target that cannot parse
  `**` (issue #1435).

## See also

- [shimmer-reference-behaviour.md](shimmer-reference-behaviour.md) — how the
  number↔string boundary is observed.
- [../dialect-profile-model.md](../dialect-profile-model.md) — the dialect
  profile that carries `NumberSyntax`, and how one is resolved and threaded.
- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
  [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — the other two day-one contracts.
