# Taint analysis — sources, sinks, colours, and interprocedural propagation

How the taint engine tracks untrusted data from its sources to dangerous
sinks, how taint colours model sanitisation, and where to add taint rules for
a command.

The taint engine tracks whether values originate from untrusted sources (user
input) and whether they reach dangerous sinks (XSS, injection, SSRF).
`TaintLattice` models the taint state of each SSA value; `TaintColour` flags
model which sanitisation properties hold.  Interprocedural propagation extends
taint tracking across procedure boundaries.

Source: `rust/tcl-compiler/src/taint.rs` — `TaintColour`, `TaintLattice`,
`propagate_taints`, `find_taint_warnings` / `find_taint_warnings_for_cu`,
`classify_network_interp_sinks`, `emit_double_encode_warnings`,
`find_destructive_file_warnings`, `find_setter_constraint_warnings`;
`rust/tcl-registry/src/taint.rs` — `TaintColour`, `SetterConstraint`,
`is_taint_source`, `taint_source_colour`, `is_sanitiser`,
`classify_taint_sinks`, `TaintSinkInfo`

### TaintLattice

`TaintLattice` is a single field: a `TaintColour` bag. The `TAINTED` bit
says "may be tainted"; every other bit is a proven mitigation.

```rust
TaintLattice::clean()                                    // no TAINTED bit set
TaintLattice::tainted()                                  // tainted, no mitigation proven
TaintLattice::tainted().with(TaintColour::URL_ENCODED)   // tainted, but URI-encoded
```

- `clean()`: literal or constant — definitely safe.
- Tainted with mitigation colours: tainted but with known safety properties.
- `tainted()`: tainted with no safety guarantees.
- Join: `TaintLattice::join` unions the taint bits (may-have) and
  intersects the mitigation colours (must-have) across operands that
  actually carry taint. A clean operand is the join identity, so it never
  dilutes the other operand's mitigations.

### TaintColour flags

`TaintColour` is a `bitflags` set over `u32`, with its atomic declarations in
the registry-owned `TaintColourAtom` enum at `rust/tcl-registry/src/taint.rs`.
Spec data (`taint_transform`,
`taint_double_encode_colour`, `taint_sink_safe_colour`) can name colours.
`rust/tcl-compiler/src/taint.rs` declares a same-width mirror. Its bridge
exhaustively matches the registry-owned `TaintColourAtom` enum in both
directions, then asserts the mapped mask is bit-identical; adding a registry
colour therefore breaks the compiler match until it is handled deliberately.

| Flag | Meaning |
|------|---------|
| `TAINTED` | May be tainted — the one non-mitigation bit |
| `CRLF_FREE` | No carriage return or linefeed characters |
| `URL_ENCODED` | URI-encoded (percent-encoding) |
| `HTML_ESCAPED` | HTML entity-escaped |
| `LIST_CANONICAL` | Canonical Tcl list representation |
| `SHELL_ATOM` | Safe as a single shell word |
| `REGEX_LITERAL` | Regex-quoted literal |
| `PATH_PREFIXED` / `PATH_NORMALISED` / `PATH_BOUNDED` | Path-shape mitigations |
| `NON_DASH_PREFIXED` | Cannot be read as an option flag |
| `HEADER_TOKEN_SAFE` | Safe as an HTTP header token |
| `IP_ADDRESS` / `PORT` / `FQDN` | Validated network-address shapes |
| `PATH_JOINED` | Assembled by `[file join]` (portable, not canonicalised) |
| `CHANNEL` | A channel handle |

Colours compose with `|` (bitwise OR); mitigations join by `&`
(intersection).

Named masks on the compiler's `TaintColour`: `ALL` (every mirrored bit),
`T102_SAFE` (`PATH_PREFIXED | NON_DASH_PREFIXED | IP_ADDRESS | PORT | FQDN`),
`CRLF_SAFE` (`CRLF_FREE | IP_ADDRESS | PORT | FQDN`), and `REDIRECT_SAFE`
(`PATH_PREFIXED | PATH_NORMALISED`).

### Source and sink classification

**Sources** — `tcl_registry::taint::is_taint_source` accepts four kinds of
evidence, all registry data:

- `Traits::TAINT_SOURCE` on the `CommandSpec` (`gets`, `read`, `exec`,
  `socket`);
- `Traits::UNNORMALISED_HTTP_GETTER` on the `CommandSpec` (`HTTP::uri`,
  `HTTP::path`, `HTTP::query`);
- `Traits::TAINT_SOURCE` on a resolved `SubCommand` (`chan gets`,
  `chan read`, `encoding convertfrom`) — resolved prefix-aware, so an
  abbreviation cannot dodge it;
- the registry's dialect-agnostic taint-source index
  (`CommandRegistry::taint_source`), which holds the iRules namespace getters
  (`HTTP::header`, `HTTP::cookie`, `IP::client_addr`, …) with each spec's own
  `taint_source` colour.

`taint_source_colour` returns the colour the source stamps, augmented with
derived safety properties.

**Sinks**: classified by `tcl_registry::taint::classify_taint_sinks`, which
returns a `TaintSinkInfo` built from the spec's declared sink fields —
`taint_output_sink` (narrowed by `taint_output_sink_subcommands`),
`taint_log_sink`, `taint_network_sink_args`, and
`taint_interp_eval_subcommands` — plus a trait-driven `is_code_sink` from
`Traits::TAINT_SINK | Traits::EVALUATES_CODE`. The most specific fact wins: a
command with a declared sink code uses that code, and only a command with no
declared code falls back to the T100 code-execution classification.

- Code execution: `eval`, `exec`, `uplevel`, `subst`, unbraced `expr` → T100
- Output: `puts` (`taint_output_sink = "T101"`) → T101
- HTTP response body: `HTTP::respond` → IRULE3001 (XSS)
- Headers/cookies: `HTTP::header` / `HTTP::cookie` insert/replace forms →
  IRULE3002 (injection)
- Log output: `log` (`taint_log_sink = "IRULE3003"`) → IRULE3003 (log injection)
- Redirect: `HTTP::redirect` → IRULE3004 (open redirect)
- Network address: any spec with `taint_network_sink_args`, at exactly the
  positional slots it names → T104 (SSRF)
- Cross-interpreter eval: `taint_interp_eval_subcommands` → T105

`classify_taint_sinks` is called with an empty dialect set from the compiler
side, so a sink the active profile bans (`exec` under `f5-irules`) keeps its
classification if it appears in source anyway.

### Taint transforms

`transform_colour` in `rust/tcl-compiler/src/taint.rs` reads the command's
`taint_transform` colour from the registry, preferring a resolved
subcommand's transform over the bare command's:

- `URI::encode` on tainted data: adds `URL_ENCODED | CRLF_FREE`.
- `HTML::encode` / `HTML::escape` / `htmlencode` on tainted data: adds
  `HTML_ESCAPED | CRLF_FREE`.
- `file normalize` adds `PATH_NORMALISED`; `file join` adds `PATH_JOINED`.

A command with *no* registry classification at all — not a source, sanitiser,
transform, or passthrough — is not a no-op: `TaintLattice::shape_unproven`
keeps the `TAINTED` bit but strips the `T102_SAFE` shape colours, because an
arbitrary transformation (`string tolower`, `lindex [split …]`) cannot be
assumed to preserve a "cannot start with `-`" proof.

### Interprocedural taint propagation

`solve_interprocedural_taints`
(`rust/tcl-compiler/src/taint_interproc.rs`) extends taint tracking across
procedure boundaries using `ProcTaintSummary` values:
1. Analyse each procedure in isolation.
2. For each call site, propagate caller taints into callee parameters.
3. Propagate callee return taint back to the caller's result variable.
4. Iterate until fixpoint.

#### What a summary costs, and why (#1187)

A summary is a **transfer function**, not a single value: a clean base return
taint, plus — for each parameter — one return taint per colour basis, so a
caller can ask "what comes back if I pass a value tainted *this* way?".  There
are 17 bases, so inferring a summary the direct way is `1 + 17P` complete
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
  **0** solves instead of `1 + 17P`.  This is the common case in real Tcl — a
  procedure that returns nothing, a literal, or a braced constant.
- **Unread parameter** — `seed_entry_taints` skips a name that is not interned in
  the SSA, so seeding a parameter the body never reads leaves the initial taint
  map bit-identical to the clean base run's.  The scenario *is* `return_base`:
  **0** solves instead of 17, per such parameter.

What remains is `1 + 17 × (parameters actually read, in a procedure whose return
value is substitution-bearing)`.  Collapsing that last `17×` needs a different
representation — one multi-colour symbolic traversal carrying a
per-`(parameter, basis)` dependency bitset — which changes what the solver
computes rather than skipping work it can prove redundant, and is not attempted.

**Cost note.** Adding `PATH_JOINED` and `CHANNEL` widens the unpruned transfer
matrix from `1 + 15P` to `1 + 17P`: at most two additional solves per read
parameter (about 13% on that inner matrix). The constant-return and unread-
parameter prunes above still apply unchanged; the historical measurements below
predate this domain widening and are not presented as a new benchmark.

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

1. `HTTP::header` is a taint source (registry taint-source index) → `host`
   carries `TAINTED` with its declared source colour.
2. `string tolower $host` carries no registry taint classification, so the
   taint survives and `shape_unproven` strips the `T102_SAFE` shape colours →
   `lower` is still tainted.
3. `HTTP::respond` declares `taint_output_sink = "IRULE3001"` → `IRULE3001`
   ("Tainted data in HTTP response body").

### Diagnostic codes

Every code below is a `DiagCode` variant in
`rust/tcl-core-types/src/diag_code.rs`; the iRules ones are spelled
`DiagCode::Irule3001` in Rust and `"IRULE3001"` in their string form.

| Code | Category |
|------|----------|
| T100 | Dangerous code-execution sink (`eval` / `uplevel` / `subst` / unbraced `expr` / `exec`; also braced-`expr` operands for type coercion) |
| T101 | Tainted output (`puts`) |
| T102 | Option injection (tainted arg without `--`) |
| T103 | Regex injection / ReDoS (internal) |
| T104 | SSRF (network address sink) |
| T105 | Cross-interpreter code injection (`interp eval` / `invokehidden`) |
| T106 | Double-encoding (internal) |
| IRULE1007 | Collection without its registered release on the same connection side |
| IRULE1008 | `*::release` without a matching `*::collect` on the same connection side |
| IRULE3001 | Tainted data in HTTP response body |
| IRULE3002 | Header/cookie injection |
| IRULE3003 | Log injection |
| IRULE3004 | Open redirect (`HTTP::redirect`) |

## Decision rule

- To add taint tracking for a new command: set `taint_source: Some(colour)`
  on its `CommandSpec` for sources, or set the relevant `taint_*_sink*`
  fields (read by `classify_taint_sinks`) for sinks.
- To add a new sanitiser: set `taint_transform: Some(colour)` on the
  sanitising command's spec — `transform_colour` stamps it onto the result.
  A colour above bit 14 will not reach the compiler until its mirror gains
  the bit.
- Taint colours join by intersection — if any path is unsanitised, the colour
  is lost at the merge point.

## Related docs

- [Example 12 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-12-taint-analysis--httpheader-to-httprespond-subcommand-flow-and-spec)
- [GLOSSARY.md — Taint analysis](../../GLOSSARY.md#taint-analysis)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
