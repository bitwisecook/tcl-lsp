# KCS: Rendered Value Properties — string content analysis over SSA

## Symptom

A contributor needs to understand how the compiler determines string content
properties (path separators, escape sequences, interpolation) of SSA values
after Tcl backslash substitution, or needs to add a new property for a
downstream consumer.

## Context

The Rendered Value Properties pass computes per-SSA-value properties of the
rendered (post-backslash-subst) string content.  It runs **after SCCP** (so
constant values are resolved) and **before taint propagation** (so downstream
consumers can query properties without re-lexing).

Source: [`core/compiler/rendered_properties.py`](../../../core/compiler/rendered_properties.py)

Primary consumer: [`core/compiler/taint/_path_concat.py`](../../../core/compiler/taint/_path_concat.py) (W201 detection).

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

The `_find_path_concat_warnings()` function in `_path_concat.py` uses
rendered properties for detection and taint colours for suppression:

**Detection** (both required):
1. `HAS_FORWARD_SLASH` or `HAS_BACKSLASH` in the `may` properties
2. `HAS_INTERPOLATION` in the `may` properties

**Suppression**:
- `PATH_NORMALISED` or `PATH_JOINED` taint colour on the SSA value
- Forward-scan: next assignment to the same variable in the same block
  is `[file normalize $var]`

### Adding a new property

1. Add the flag to `RenderedProperties(Flag)` in `rendered_properties.py`
2. Add it to `_MAY_MASK` or `_MUST_MASK` depending on join semantics
3. Set it in `_evaluate_rendered_props_for_value()` and/or
   `_evaluate_rendered_props_for_const()`
4. Add tests in `tests/test_rendered_properties.py`
5. Consume it in the downstream pass

## Resolution

Refer to this document when modifying the rendered properties pass, adding
new properties, or debugging W201 false positives/negatives.  The lattice
join in `rendered_join()` handles phi nodes automatically.
