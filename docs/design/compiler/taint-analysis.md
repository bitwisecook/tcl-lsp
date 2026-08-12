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

Source: `rust/tcl-compiler/src/taint.rs` (the lattice, propagation, and sink
checks), `rust/tcl-compiler/src/taint_interproc.rs` (procedure summaries and
their fixpoint), `rust/tcl-registry/src/taint.rs` (the colour set and the
source / sanitiser / sink tables)

## Content

### TaintLattice

`TaintLattice` is a one-field struct — a bag of colours — not a three-rung
chain:

```rust
pub struct TaintLattice {
    /// Bag of colours. `TAINTED` membership means "may be tainted".
    pub colours: TaintColour,
}
```

- `TaintLattice::clean()` — no taint, no mitigations proven.
- `TaintLattice::tainted()` — the bare `TAINTED` bit, no mitigations.
- `is_tainted()` — whether the `TAINTED` bit is present.
- `with(colour)` adds a mitigation; `sanitised()` clears `TAINTED` while
  keeping the mitigations.

`TaintLattice::join(other)` implements the lattice join: taint is
**may-have** (union of the `TAINTED` bit), mitigations are **must-have**
(intersection). A clean operand is the join *identity*, not an annihilator —
treating it as one would strip proven-safe colours from the other side and
re-fire T102.

### TaintColour flags

`TaintColour` is a registry-owned `bitflags!` set (`u32`), so spec data can
name colours and the compiler re-exports it rather than keeping a parallel
copy:

| Flag | Meaning |
|------|---------|
| `TAINTED` | Value is attacker-controlled |
| `PATH_PREFIXED` | Always starts with `/` (`HTTP::uri`, `HTTP::path`) |
| `NON_DASH_PREFIXED` | Provably starts with a non-`-` literal |
| `CRLF_FREE` | Proven to contain no CR/LF characters |
| `SHELL_ATOM` | Token-safe atom (no shell metachar splitting) |
| `LIST_CANONICAL` | Canonical Tcl list representation |
| `REGEX_LITERAL` | Regex-escaped literal payload |
| `PATH_NORMALISED` | Path has been normalised (no raw traversal form) |
| `PATH_BOUNDED` | Normalised path verified within an intended directory |
| `HEADER_TOKEN_SAFE` | Valid HTTP header-token charset |
| `HTML_ESCAPED` | HTML-escaped text context |
| `URL_ENCODED` | URL-encoded text context |
| `IP_ADDRESS` | IPv4 or IPv6 address |
| `PORT` | Integer 0–65535 |
| `FQDN` | Fully qualified domain name |
| `PATH_JOINED` | Assembled via `[file join]` (portable, not canonicalised) |
| `CHANNEL` | I/O channel handle (`open`, `socket`, `chan create`, `HSL::open`) |

Colours compose with `|` (bitwise OR) and join by `&` (intersection).
`augment_source_colours` adds the conservative implications: `PATH_PREFIXED`
implies `NON_DASH_PREFIXED`; any of `IP_ADDRESS` / `PORT` / `FQDN` implies
`NON_DASH_PREFIXED | CRLF_FREE | SHELL_ATOM`.

### Source and sink classification

Both are **registry-driven** — the compiler matches no command names.

**Sources**: `tcl_registry::taint::is_taint_source` /
`taint_source_colour` cover the trait-driven sources (`gets`, `read`,
`exec`, `socket`), the subcommand sources (`chan gets`, `chan read`,
`encoding convertfrom`), each spec's own `taint_source: Option<TaintColour>`
(e.g. `HTTP::header` declares `Some(TaintColour::TAINTED)`), the iRules
`UNNORMALISED_HTTP_GETTER` trait, and the iRules namespace-prefix fallback
`IRULES_TAINT_SOURCE_PREFIXES` (`HTTP::`, `URI::`, `IP::`, `TCP::`, `UDP::`,
`SSL::`, `STREAM::`).

**Sinks**: `taint::classify_sink` delegates to
`tcl_registry::taint::classify_taint_sinks`, which reads `Traits::TAINT_SINK`
/ `Traits::EVALUATES_CODE` plus each spec's declared output / log sink code:
- Code execution: `eval`, `uplevel`, `subst`, `expr`, `exec` → T100
- Output: `puts` → T101
- HTTP response body → IRULE3001 (XSS)
- Headers/cookies → IRULE3002 (injection)
- Log output → IRULE3003 (log injection)
- Redirect → IRULE3004 (open redirect)
- Network address (`taint_network_sink_args`) → T104 (SSRF)
- Cross-interpreter (`taint_interp_eval_subcommands`) → T105

Sink and source classification key off
`Statement::canonical_command_or_source`, so a call through a
`rename`d or `interp alias`ed name resolves to whatever it now denotes, and
a namespace-scoped user `proc` shadowing a builtin is not misclassified as
that builtin (`shadowed_builtin_names`).

### Taint transforms

A spec's `taint_transform: Option<TaintColour>` (on the command or on a
matching subcommand) is stamped onto the tainted result during propagation by
`taint::transform_colour` — `uri::encode` ⇒ `URL_ENCODED`, and so on. A
command with no transform (e.g. `string tolower`) passes taint through
unchanged: it is not a sanitiser.

`taint_double_encode_colour` drives T106: a value that re-enters a command
whose double-encode colour it already carries (`uri::encode [uri::encode
$x]`) is flagged by `emit_double_encode_warnings`.
`taint_sink_safe_colour` is the colour that suppresses a given sink.
`tcl_registry::taint::is_sanitiser` covers the fixed-numeric-return
sanitisers (`string length`, `string is integer`, …), which clear `TAINTED`
outright.

Two further conservative widenings: a `trace add variable` /
`trace variable` target is forced fully tainted module-wide
(`apply_module_variable_traces`), because a runtime handler can rewrite the
value invisibly.

### Interprocedural taint propagation

`solve_interprocedural_taints` (`rust/tcl-compiler/src/taint_interproc.rs`)
extends taint tracking across procedure
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

1. `HTTP::header value` is a taint source (its spec declares
   `taint_source: Some(TaintColour::TAINTED)`) → `host` carries `TAINTED`
   with no mitigations.
2. `string tolower $host` declares no `taint_transform` and is not a
   sanitiser → the colours pass through unchanged, so `lower` still carries
   bare `TAINTED`.
3. `HTTP::respond`'s body argument is a sink → `IRULE3001` ("XSS: tainted
   value in HTTP response body").

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

- To add taint tracking for a new command, put the fact in the **registry
  spec**, not in the compiler: `taint_source: Some(TaintColour::…)` for a
  source, `Traits::TAINT_SINK` plus the spec's declared sink code for a sink.
  `classify_sink` has no per-command branch to add a case to.
- To add a new sanitiser: set `taint_transform: Some(TaintColour::…)` on the
  spec (or the matching subcommand) so `transform_colour` stamps it, or make
  it a fixed-numeric-return sanitiser that `is_sanitiser` recognises.
- Taint colours join by intersection — if any tainted path is unsanitised,
  the colour is lost at the merge point. A *clean* path is the join
  identity and does not strip colours.

## Related docs

- [Example 12 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-12-taint-analysis--httpheader-to-httprespond-subcommand-flow-and-spec)
- [GLOSSARY.md — Taint analysis](../../GLOSSARY.md#taint-analysis)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
