# KCS: Taint analysis — sources, sinks, colours, and interprocedural propagation

## Symptom

A contributor needs to understand how the taint engine tracks untrusted data
from HTTP inputs to dangerous sinks, how taint colours model sanitisation, or
needs to add taint rules for a new command.

## Context

The taint engine tracks whether values originate from untrusted sources (user
input) and whether they reach dangerous sinks (XSS, injection, SSRF).
`TaintLattice` models the taint state of each SSA value; `TaintColour` flags
model which sanitisation properties hold.  Interprocedural propagation extends
taint tracking across procedure boundaries.

Source: [`core/compiler/taint/`](../../../core/compiler/taint/) —
[`_lattice.py`](../../../core/compiler/taint/_lattice.py),
[`_sinks.py`](../../../core/compiler/taint/_sinks.py),
[`_propagation.py`](../../../core/compiler/taint/_propagation.py),
[`_api.py`](../../../core/compiler/taint/_api.py),
[`core/commands/registry/taint_hints.py`](../../../core/commands/registry/taint_hints.py)

## Content

### TaintLattice

```
UNTAINTED ─────────── TAINTED(colours) ─────────── TAINTED_TOP
   (safe)            (partially sanitised)          (fully tainted, no safety)
```

- `UNTAINTED`: literal or constant — definitely safe.
- `TAINTED(colours)`: tainted but with known safety properties (e.g. URL-encoded).
- `TAINTED_TOP`: tainted with no safety guarantees.
- Join: `taint_join(a, b)` — intersection of colours (only properties shared
  by all incoming paths survive).

### TaintColour flags

| Flag | Meaning |
|------|---------|
| `CRLF_FREE` | No carriage return or linefeed characters |
| `URL_ENCODED` | URI-encoded (percent-encoding) |
| `HTML_ESCAPED` | HTML entity-escaped |
| `B64_ENCODED` | Base64-encoded |
| `INT_SAFE` | Known to be an integer |
| `URI_COMPONENT` | Result of `URI::decode` component extraction |

Colours compose with `|` (bitwise OR) and join by `&` (intersection).

### Source and sink classification

**Sources**: Commands whose `TaintHint.source` is set on their registry spec:
- `HTTP::host`, `HTTP::uri`, `HTTP::query`, `HTTP::header value`, `HTTP::cookie`

**Sinks**: Argument positions classified by `_classify_sink()`:
- Code execution: `eval`, `uplevel`, `subst`
- HTTP response body: `HTTP::respond` body arg → IRULE3001 (XSS)
- Headers/cookies: `HTTP::header insert/replace` → IRULE3002 (injection)
- Log output: `log` → IRULE3003 (log injection)
- Redirect: `HTTP::redirect` → IRULE3004 (open redirect)
- Network address: `connect`, HTTP::geturl → T104 (SSRF)

### Taint transforms

Some commands transform taint colours:
- `string tolower` on tainted data: passes taint through unchanged (no sanitisation).
- `URI::encode` on tainted data: adds `URL_ENCODED` colour.
- `b64encode` on tainted data: adds `B64_ENCODED` colour.
- `HTML::encode` on tainted data: adds `HTML_ESCAPED` colour.

`_derive_transform_colours()` in `_propagation.py` applies these transforms.

### Interprocedural taint propagation

`_solve_interprocedural_taints()` extends taint tracking across procedure
boundaries using `ProcTaintSummary` objects:
1. Analyse each procedure in isolation.
2. For each call site, propagate caller taints into callee parameters.
3. Propagate callee return taint back to the caller's result variable.
4. Iterate until fixpoint.

### Worked example — `HTTP::header value Host` → `HTTP::respond`

```tcl
set host [HTTP::header value Host]
set lower [string tolower $host]
HTTP::respond 200 content "<h1>$lower</h1>"
```

1. `HTTP::header value` is a taint source → `host` is `TAINTED_TOP`.
2. `string tolower $host` → passes taint through → `lower` is `TAINTED_TOP`.
3. `HTTP::respond` body arg is a sink → `IRULE3001` ("XSS: tainted value
   in HTTP response body").

### Diagnostic codes

| Code | Category |
|------|----------|
| T100 | Dangerous code-execution sink |
| T101 | Tainted output |
| T102 | Option injection (tainted arg without `--`) |
| T103 | Regex injection / ReDoS |
| T104 | SSRF (network address sink) |
| T105 | Cross-interpreter code injection |
| T106 | Double-encoding (informational) |
| T200 | Collect without release |
| T201 | Release without collect |
| IRULE3001 | XSS in HTTP response body |
| IRULE3002 | Header/cookie injection |
| IRULE3003 | Log injection |
| IRULE3004 | Open redirect |

## Decision rule

- To add taint tracking for a new command: set `TaintHint.source=True` on its
  `CommandSpec` for sources, or add a case to `_classify_sink()` for sinks.
- To add a new sanitiser: add a transform rule in `_derive_transform_colours()`.
- Taint colours join by intersection — if any path is unsanitised, the colour
  is lost at the merge point.

## Related docs

- [Example 12 in walkthroughs](../../example-script-walkthroughs.md#example-12-taint-analysis--httpheader-to-httprespond-subcommand-flow-and-spec)
- [GLOSSARY.md — Taint analysis](../../GLOSSARY.md#taint-analysis)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
