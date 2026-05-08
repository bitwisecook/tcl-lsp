# KCS: Tcl 9.0.3 corner cases for variable handling

> **Audience:** Developer
> **Type:** Reference

## Applies to

analyser, lexer, completion, diagnostics

## Profiles

default

## Purpose

This document records the *empirically observed* behaviour of real
Tcl 9.0.3 (built from `tmp/tcl9.0.3/` and run as `tclsh`) on a wide
range of variable-handling corner cases.  It is the **source of truth**
that our analyser, lexer, and completion provider should match.  Every
finding is reproducible via `tests/data/tcl_probes_full.tcl`.

Use this document when deciding whether the analyser's behaviour is
correct, when triaging a "looks like a bug" report, or when adding new
diagnostics.

## How to reproduce

Build `tclsh` from the bundled Tcl 9.0.3 source:

```sh
cd tmp/tcl9.0.3/unix && ./configure --prefix=/tmp/tcl9-local && make tclsh
LD_LIBRARY_PATH=$PWD ./tclsh ../../../tests/data/tcl_probes_full.tcl
```

The probe file's last line prints a `PROBE SUMMARY: N pass, …` so you
can spot regressions if the version of Tcl ever changes.

## Findings

### Variable substitution — bare `$name`

| Probe | Behaviour | Reason |
|---|---|---|
| `$foo` | substitutes scalar `foo` | classic |
| `$foo_bar` | substitutes `foo_bar` | underscore is a name char |
| `$foo123` | substitutes `foo123` | digits are name chars |
| `$::g` | substitutes global `g` (`::g`) | leading `::` allowed |
| `$::ns::x` | substitutes namespace var | `::` is the only multi-char sep |
| `$foo.bar` | substitutes `foo`, leaves `.bar` literal | `.` is not a name char |
| `$foo-bar` | substitutes `foo`, leaves `-bar` literal | `-` is not a name char |
| `$$x` | literal `$` followed by `$x` | first `$` has no valid name (next char is `$`) |
| `$:` (alone) | literal `$:` | `:` followed by non-`:` -> first `$` has no name |
| `$::` (alone) | error: `can't read "::"` | `::` parses as the var name; no var named `::` |
| `$x::` | error: `can't read "x::"` | trailing `::` is taken as part of the name |

### Variable substitution — brace `${name}`

| Probe | Behaviour | Reason |
|---|---|---|
| `${foo}` | scalar `foo` | classic |
| `${weird-name}` | reads var literally named `weird-name` | brace form accepts non-word chars |
| `${with space}` | reads var named `with space` | spaces allowed |
| `${a.b}` | reads var named `a.b` | dots allowed |
| `${café}` | reads `café` | UTF-8 allowed |
| `${}` | reads the empty-named variable | yes, Tcl allows `set "" 1` |
| `${a` *newline* `b}` | reads `a\nb` (with literal newline) | newlines allowed |
| `${arr(x)}` | array element `x` of `arr` | brace form accepts `(idx)` |
| `${a\}b}` | reads var named `a\}b` (4 chars, **including** the literal `\`) | tclParse.c §`Tcl_ParseVarName`: `\X` consumes 2 chars, `}` doesn't close |
| `${a{b}c}` | reads var named `a{b}c` | inner `{...}` are tracked with brace counting |

### `${name}` vs `::${name}` -- adjacency

These look similar but parse very differently:

| Form | Parse | Result |
|---|---|---|
| `${::tracevar}` | one VAR token, name = `::tracevar` | qualified-global lookup |
| `::${tracevar}` | literal `::` + VAR(`tracevar`) | concatenation: `"::" + value` |
| `::${::tracevar}` | literal `::` + VAR(`::tracevar`) | `"::" + value-of-::tracevar` |

The `::` *outside* `${...}` is plain literal text, not part of the variable name.  Cross-checked on tclsh 9.0.3 -- ``set ::tracevar GLOB; puts "::${tracevar}"`` prints `::<value-of-local-tracevar>` (NOT the global), while ``puts ${::tracevar}`` prints `GLOB`.

### Mixed bare/brace namespace forms don't compose

| Form | Tcl 9.0.3 result |
|---|---|
| `$::myns::x` (all-bare with qualified name) | reads `::myns::x` correctly |
| `${::myns::x}` (all-brace) | reads `::myns::x` correctly |
| `$::myns::${suffix}` (bare prefix + brace) | **fails** -- bare form lookups `::myns::` (with trailing `::`) and errors |

The bare-form parser stops at the `$` of the brace form (since `$` isn't a name char), so `$::myns::${suffix}` is two separate tokens: a bare-form lookup of `::myns::` (which fails) and a brace-form lookup of `suffix`.  To compute a qualified name dynamically, build the full name via ``set name "::myns::${suffix}"; set $name`` instead.

**Key surprise:** the Tcl(n) man page says "There is no further
substitution or modification" inside `${name}`, but in practice the
parser **does** recognise `\X` (2-char escape) and **does** track
inner `{...}`.  The substitution that's blocked is only `$` and `[`
substitution.

### Brace form vs. bare for array element substitution

| Form | Substitutes index? |
|---|---|
| `$arr(idx)` | yes |
| `$arr($k)` | yes — `$k` substitutes at runtime |
| `${arr(idx)}` | no — the entire ``arr(idx)`` is the literal name |
| `${arr($k)}` | **no** — looks up element literally named `$k` (W216) |
| `${arr}(idx)` | scalar `${arr}` followed by literal `(idx)` (W216) |

### `set` indirection (the only escape from W215/W216 unreachables)

```tcl
set "weird}name" 42
[set "weird}name"]                   ;# 42 (works for any name)
[set $name_holding_the_real_name]    ;# works for fully dynamic names
```

`set` parses its argument via the command parser, which substitutes
`$`-vars but treats everything else literally — so an indirected
`$foo` inside the index keeps working.

### Globals and namespaces

| Probe | Behaviour |
|---|---|
| Inside proc, bare `$g` reads local `g` only | locals don't auto-promote to globals |
| `$::g` from any scope | always the global |
| `global g` | local alias `g` → `::g` |
| `global ::g` | also creates local alias `g` (qualifier stripped) |
| `global ::ns::x` | **also works** — local alias `x` → `::ns::x`  *(despite docs hinting at global-only)* |
| `variable v` inside `namespace eval ::ns` | creates `::ns::v` |
| `variable v` inside a proc | aliases the namespace var as local `v` |
| `variable $name 7` | name *is* substituted — `$name` becomes its value, then var created with that name |
| Same name as both global and namespace | works — `::same` (var) and `::same::x` (ns var) coexist |

### upvar / namespace upvar

| Form | Behaviour |
|---|---|
| `upvar 1 var local` | alias `local` to caller's `var` |
| `upvar var local` | same — default level is 1 |
| `upvar 0 x y` | alias `y` to `x` in the **same** (current) scope |
| `upvar #0 ::g local` | alias `local` to global (absolute frame 0) |
| `upvar #N var local` | absolute frame N from bottom |
| `namespace upvar ::ns x local` | local alias to namespace var |
| Multiple pairs: `upvar 1 a la b lb` | each pair becomes an alias |

### uplevel

| Form | Behaviour |
|---|---|
| `uplevel #0 body` | runs body in the global frame |
| `uplevel 1 body` | runs body in the caller's frame |
| `uplevel body` | default level 1 |
| `uplevel "0" body` | string `"0"` works the same as `#0` |
| `uplevel #0 body1 body2` | bodies are concatenated and run together |

### dict

| Form | Behaviour |
|---|---|
| `dict with var body` | each dict key becomes a local var during body |
| `dict with` body locals **persist** after body | not strictly "during" — they remain after |
| `dict update var k1 v1 ?k2 v2 …? body` | creates aliases for each `var`; missing keys auto-created |
| `dict for {k v} dict body` | iterates pairs as `k`/`v` locals |
| Funny dict keys (`"two words"`) become locals with that exact name | accessible via `[set "two words"]` |

### Arrays

| Form | Behaviour |
|---|---|
| `set arr(name) 1` | creates element `name` |
| `set arr() 1` | empty index works |
| `set arr($k)` | command-word `$k` subst — element name = value of `k` |
| `set "arr(\$foo)" 1` | literal element name `$foo` (different element!) |
| `set foo(a)(b) 2` | accepted — element name is `a)(b` of array `foo` |
| `$foo((x))` | error "invalid character in array index" |
| `set arr foo` then `set arr(x) 1` | error: variable isn't array |
| `info exists "(x)"` after `set "(x)" 1` | works — empty array name allowed |

### Quoting interactions

| Form | Substitution? |
|---|---|
| `"x is $x"` | yes — quotes substitute |
| `{x is $x}` | no — braces are literal |
| `"literal \$x"` | no — `\$` is escape |
| `"[expr {2+2}] is four"` | yes — `[...]` substitutes |

### Backslash counting -- quotes vs braces

A subtle source of mismatches: how many backslashes survive into the runtime variable name depends on which quoting form encloses the `set` argument.

| Source | Runtime var name (hex) | Why |
|---|---|---|
| `set "back\slash" 1` | `backslash` (9 bytes, no `\`) | quoted-arg parsing drops `\` for unknown escapes |
| `set "back\\slash" 1` | `back\slash` (10 bytes, 1 `\`) | `\\` → `\`, then `\s` → ... wait no, actually `\\` consumed first, then `slash` is plain |
| `set "back\\\slash" 1` | `back\slash` (10 bytes, 1 `\`) | `\\\s` parses as `\\` then `\s`: `\` + `s` = `\s` literal? No: `\\` → `\` (1 byte), then `\s` → `s` (drop `\`).  Final: `back\s slash` -- wait test says 10 bytes total = `back\slash` |
| `set {back\slash} 1` | `back\slash` (10 bytes, 1 `\`) | braces preserve literally |
| `set {back\\\slash} 1` | `back\\\slash` (12 bytes, 3 `\`) | braces preserve literally -- 3 in, 3 out |

**Key takeaway:** `"..."` applies backslash subst (drops `\` before unknown chars, collapses `\\` to `\`).  `{...}` preserves every byte literally.  Three backslashes in a quoted arg collapse to **one**; three backslashes in a braced arg stay as **three**.

Verified empirically against tclsh 9.0.3 -- see `tests/data/tcl_probes_definitive.tcl` §A.

## Known divergences in our analyser/lexer

### Lexer's `${...}` form does NOT recognise `\X` escape

`core/parsing/lexer.py::_parse_var` reads chars until the first `}`,
treating backslashes as literal.  Tcl 9.0.3's parser instead
consumes 2 chars on `\X` (so `\}` does not close the brace) and
tracks inner `{...}` with brace counting.

**Impact:** rare in practice — variable names with literal backslashes
or balanced inner braces are exotic.  Documented here as a known
limitation pending a follow-up issue to align the lexer.

**Test:** `tests/data/tcl_probes_full.tcl §A.brace-backslash-keeps-as-escape`
(currently disabled in the probe runner due to escaping conflicts at
the probe-wrapper level; the standalone case
`/tmp/brace_lexer_probe.tcl` confirms the divergence).

### `${arr}(idx)` analyser scope

W216 detects this and offers a quick fix.  See the dedicated KCS
[W216](codes/kcs-diagnostic-w216-broken-brace-array-element-reference.md).

### `${arr($foo)}` runtime semantics

The brace form does *no* `$` substitution inside the index.  This
is the second W216 pattern; the quick fix uses bare `$arr($foo)`
which *does* substitute.

### Variables with `}` or array indices with `)`

Created via `set "weird}name" 1`, but unreachable via any `$`-form.
W215 alerts the user; see [W215](codes/kcs-diagnostic-w215-variable-name-unreachable-via-substitution.md).

## Related

- [W215 KCS](codes/kcs-diagnostic-w215-variable-name-unreachable-via-substitution.md)
- [W216 KCS](codes/kcs-diagnostic-w216-broken-brace-array-element-reference.md)
- Tcl(n) man page §"Variable substitution"
- `tclParse.c::Tcl_ParseVarName` (Tcl 9.0.3 source)
- `tests/data/tcl_probes_full.tcl` (machine-runnable probe set)
