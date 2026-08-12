# Rendered value properties — string content analysis over SSA

How the compiler determines the string content properties of an SSA value —
path separators, escape sequences, and interpolation — after Tcl backslash
substitution, and how to add a property for a downstream consumer.

The Rendered Value Properties pass computes per-SSA-value properties of the
rendered (post-backslash-subst) string content.  It runs **after SCCP** (so
constant values are resolved) and **before taint propagation** (so downstream
consumers can query properties without re-lexing).

Source: `rust/tcl-compiler/src/rendered_properties.rs`

Primary consumer: `rust/tcl-compiler/src/path_concat.rs` (W201 detection).

## Content

### Lattice design

The analysis uses a **reduced product** of two property groups with different
join semantics:

```
RenderedValueProps(may, must)

may  = union at phi joins      (overapproximate)
must = intersection at phi joins  (underapproximate)
```

- **may** properties: if *any* incoming edge has the property, the merged
  value has it.  Used for "does this value possibly contain X?"
- **must** properties: only survives if *all* incoming edges agree.  Used
  for "does this value always start with X?"

This follows the standard abstract interpretation design of a reduced product
of simple boolean domains (cf. Costantini, Ferrara & Cortesi 2011).

### Property flags

| Flag | Group | Meaning |
|------|-------|---------|
| `HAS_FORWARD_SLASH` | may | Rendered literal text contains `/` |
| `HAS_BACKSLASH` | may | Rendered literal text contains `\` (path separator) |
| `HAS_CRLF` | may | Rendered literal text contains `\r` or `\n` |
| `HAS_INTERPOLATION` | may | Value contains `$var` or `[cmd]` |
| `HAS_DOUBLE_ESCAPE` | may | Rendered text contains already-escaped sequences |
| `HAS_NULL` | may | Rendered text contains `\x00` / `\0` |
| `WAS_UNESCAPED` | may (provenance) | Value passed through `subst` / `encoding convertfrom` |
| `DOUBLE_UNESCAPED` | may (provenance) | Value was already `WAS_UNESCAPED` then unescaped again |
| `FULLY_NORMALISED` | may (provenance) | Value fully canonical — no residual encoding (e.g. `-normalized`) |
| `STARTS_WITH_SLASH` | must | First rendered literal char is `/` |
| `STARTS_WITH_DASH` | must | First rendered literal char is `-` |

### Escape rendering

ESC tokens are rendered via `backslash_subst()` before property detection.
This means:

- `\n`, `\t`, `\r` → resolved to actual control characters (not path seps)
- `\x2f` → resolved to `/` (detected as `HAS_FORWARD_SLASH`)
- `\\` → resolved to single `\` (detected as `HAS_BACKSLASH`)
- `\x61` → resolved to `a` (not a path separator)

### SSA copy propagation

For pure variable references (`set x $y`), the pass propagates properties
from the source SSA value via the `uses` map on `SSAStatement`.  If `y`
has `HAS_FORWARD_SLASH`, `x` inherits it after the copy.

### Unescape provenance tracking

The `WAS_UNESCAPED` and `DOUBLE_UNESCAPED` bits are **provenance** properties
(not in `_MAY_MASK`).  They are only set explicitly by unescape commands and
propagated through copies and phi joins.  Generic unknown commands do NOT
get these bits.

Detection works at two levels:

1. **`_evaluate_rendered_props_for_value()`**: when the value is a pure
   command substitution calling `subst` or `encoding convertfrom`, sets
   `WAS_UNESCAPED` on the result.

2. **`_evaluate_rendered_def()`**: after computing the base result, checks
   whether any SSA input already has `WAS_UNESCAPED`.  If the command itself
   is also an unescape command, escalates to `DOUBLE_UNESCAPED`.

Copy propagation (`set b $a`) inherits `WAS_UNESCAPED` from the source,
so a chain like `set a [subst $x]; set b $a; set c [subst $b]` correctly
tags `c` with `DOUBLE_UNESCAPED`.

Unescape commands (`WAS_UNESCAPED`):
- `subst` — Tcl backslash/variable/command substitution
- `URI::decode` — percent-decoding
- `decode_uri` — legacy alias for `URI::decode`
- `b64decode` — base64 decoding
- `encoding convertfrom` — byte-to-string decoding

Normalised getters (`FULLY_NORMALISED`):
- `HTTP::uri -normalized`, `HTTP::path -normalized`,
  `HTTP::query -normalized` — return fully URI-decoded values with no
  residual encoding.  Still tainted (HTTP input) but encoding-safe.

`FULLY_NORMALISED` suppresses `DOUBLE_UNESCAPED` escalation: decoding a
fully normalised value is harmless since there is nothing left to decode.

### Pipeline placement

```
SCCP  →  Type propagation  →  Rendered properties  →  Taint propagation
                                      ↓
                              stored on FunctionAnalysis.rendered_props
                                      ↓
                              consumed by _find_path_concat_warnings (W201)
```

### Transfer functions

| IR node | Transfer |
|---------|----------|
| `IRAssignConst` | Direct string inspection |
| `IRAssignValue` (pure var ref) | Copy from source SSA value |
| `IRAssignValue` (interpolated) | Lex + render ESC tokens |
| `IRAssignExpr` | Numeric result → no path properties |
| `IRIncr` | Numeric result → no path properties |
| `IRCall` | Conservative (TOP) |
| `IRBarrier` | Conservative (TOP) — defeats analysis |

### W201 integration

The `_find_path_concat_warnings()` function in `path_concat.rs` uses
rendered properties for detection and taint colours for suppression:

**Detection** (both required):
1. `HAS_FORWARD_SLASH` or `HAS_BACKSLASH` in the `may` properties
2. `HAS_INTERPOLATION` in the `may` properties

**Suppression**:
- `PATH_NORMALISED` or `PATH_JOINED` taint colour on the SSA value
- Forward-scan: next assignment to the same variable in the same block
  is `[file normalize $var]`

### Adding a new property

1. Add the flag to `RenderedProperties(Flag)` in `rendered_properties.rs`
2. Add it to `_MAY_MASK` or `_MUST_MASK` depending on join semantics
3. Set it in `_evaluate_rendered_props_for_value()` and/or
   `_evaluate_rendered_props_for_const()`
4. Add unit tests in `rust/tcl-compiler/src/rendered_properties.rs`
5. Consume it in the downstream pass

## Resolution

Refer to this document when modifying the rendered properties pass, adding
new properties, or debugging W201 false positives/negatives.  The lattice
join in `rendered_join()` handles phi nodes automatically.
