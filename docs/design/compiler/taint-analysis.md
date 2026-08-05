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

Source: `compiler/taint/` —
`_lattice.py`,
`_sinks.py`,
`_propagation.py`,
`_api.py`,
`compiler/registry/taint_hints.py`

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

`solve_interprocedural_taints` extends taint tracking across procedure
boundaries using `ProcTaintSummary` objects:
1. Analyse each procedure in isolation.
2. For each call site, propagate caller taints into callee parameters.
3. Propagate callee return taint back to the caller's result variable.
4. Iterate until fixpoint.

#### What a summary costs, and why (#1187)

A summary is a **transfer function**, not a single value: a clean base return
taint, plus — for each parameter — one return taint per colour basis, so a
caller can ask "what comes back if I pass a value tainted *this* way?".  There
are 15 bases, so inferring a summary the direct way is `1 + 15P` complete
dataflow solves over the procedure's control-flow graph, for `P` parameters.
That is the dominant cost of the whole pass: about 80% of `run_all_checks` on
tcllib's `practcl.tcl`.

Two prunes cut it down.  Both are **proofs that a solve would return a value
already in hand**, not approximations, so summaries stay bit-identical — which
is what lets the debug fixpoint guard and the `compiler_check` corpus
differential keep validating the inference unchanged:

- **Constant return** — `collect_return_taint` reads the taint map only through
  `var_taint`, reachable only from `word_taint`'s three substitution branches, and
  all three require the return word to contain a `$` or a `[`.  A procedure whose
  executable returns are all value-less or all substitution-free therefore
  returns `clean` whatever the map holds, so the whole summary is the clean one:
  **0** solves instead of `1 + 15P`.  This is the common case in real Tcl — a
  procedure that returns nothing, a literal, or a braced constant.
- **Unread parameter** — `seed_entry_taints` skips a name that is not interned in
  the SSA, so seeding a parameter the body never reads leaves the initial taint
  map bit-identical to the clean base run's.  The scenario *is* `return_base`:
  **0** solves instead of 15, per such parameter.

What remains is `1 + 15 × (parameters actually read, in a procedure whose return
value is substitution-bearing)`.  Collapsing that last `15×` needs a different
representation — one multi-colour symbolic traversal carrying a
per-`(parameter, basis)` dependency bitset — which changes what the solver
computes rather than skipping work it can prove redundant, and is not attempted.

Measured with `cargo run --release -p tcl-lsp-db --example tail_profile` on
tcllib's `practcl.tcl` (8,463 lines, 116 functions), same machine, same binary:

| phase | before | after |
|---|---:|---:|
| `solve_interprocedural_taints` (whole unit) | 211.5 ms | 143.4 ms |
| `run_all_checks` | 260.1 ms | 155.9 ms |

The taint solve keeps its place as the dominant phase (about 92% of
`run_all_checks`); it is simply a third cheaper, and the checks tail as a whole
drops 40%.

#### Convergence

`converge_summaries_with` drives the summary fixpoint from a dirty set: a
procedure is re-inferred only when one of its callees' summaries moved.  The
reverse call graph comes from two sources unioned — the declared
`InterproceduralAnalysis::direct_calls`, plus `resolved_callees`, which scans the
very CFG statements the inference resolves calls from and so supplies the edges
`direct_calls` drops for a callee reached through a command substitution
(`symbolNodeOf` in `set n [$t get [symbolNodeOf …] …]`, a self-call inside
`[expr {[fib …]}]`).

A missed edge is still not a wrong answer: the worklist is followed by an
unconditional round-robin completion sweep that re-queues any procedure whose
summary still moves, and the lattice is monotone over a finite domain, so the
true least fixpoint is always reached.  **The sweep is deliberately retained.**
Removing it would require *proving* the dependency graph complete, and neither
source is: `direct_calls` misses nested substitutions, and `resolved_callees`
scans CFG statements rather than the raw words `word_taint` recurses into.  An
under-converged fixpoint is a taint **false negative** — a silently missed
security diagnostic — which is a much worse failure than one extra `O(F)` sweep,
now that the prunes above have made each inference cheap.

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
| T200 | Reserved (migrated to IRULE1007 — collect without release) |
| T201 | Reserved (migrated to IRULE1008 — release without collect) |
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

- [Example 12 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-12-taint-analysis--httpheader-to-httprespond-subcommand-flow-and-spec)
- [GLOSSARY.md — Taint analysis](../../GLOSSARY.md#taint-analysis)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
