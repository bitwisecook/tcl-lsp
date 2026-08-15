# Side-effects classification system

How the compiler describes what a command reads, writes, and touches, and how
to add side-effect metadata for a new or existing command.

The side-effects system is the source of truth for effect classification in
the compiler. `classify_side_effects` (`rust/tcl-compiler/src/side_effects.rs`)
is the classifier; its direct callers are:

- **GVN** (`gvn.rs`) — kill safety for common subexpression elimination, via
  `is_pure_command` and the `EffectRegion` bridge.
- **Interprocedural analysis** (`interprocedural.rs`) — procedure summaries
  across call boundaries.
- **Dead-store elimination** (`optimiser/elimination.rs`).
- **The analyser's command dispatch** (`analyser/commands.rs`).

Two consumers read the same data by other routes and are worth knowing about:

- **iRules flow checker** (`irules_checks.rs`) reads the *registry's* raw
  `SideEffect` declarations directly, looking for
  `SideEffectTarget::ResponseCommit` with `writes: true`; it does not call
  the classifier.
- **SCCP** (`sccp.rs`) and the optimiser's load forwarding
  (`optimiser/propagation.rs`) consult purity through
  `gvn::is_pure_command_with_traces`, which wraps the classifier with the
  execution-trace gate described below.

## Architecture

### Enums (the vocabulary)

Four enums describe the dimensions of a side effect:

| Enum | Describes | Where | Example values |
|------|-----------|-------|----------------|
| `SideEffectTarget` | *What* resource is touched | `tcl-registry` | `HttpHeader`, `SessionTable`, `Variable`, `PoolSelection` |
| `ConnectionSide` | *Which* F5 proxy side | `tcl-registry` | `Client`, `Server`, `Both`, `Global`, `None` |
| `StorageScope` | *Where* the data lives / stability | `tcl-compiler` | `ProcLocal`, `Namespace`, `Global`, `Upvar`, `Event`, `Connection`, `Static`, `SessionTable`, `Persistence`, `DataGroup`, `FileSystem`, `NetworkSocket`, `LogOutput`, `Unknown` |
| `StorageType` | *Shape* of the data | `tcl-compiler` | `Scalar`, `List`, `Dict`, `Array`, `Unknown` |

`SideEffectTarget` and `ConnectionSide` are declared in
`rust/tcl-registry/src/side_effects.rs` so spec literals can name them;
`StorageScope` and `StorageType` are compiler-side only, inferred by the
classifier rather than declared on a spec.

### Structs

There are **two** `SideEffect` structs. The registry's
(`rust/tcl-registry/src/side_effects.rs`) is the `'static`, `Copy` shape a
spec literal writes:

```rust
pub struct SideEffect {
    pub target: SideEffectTarget,
    pub reads: bool,
    pub writes: bool,
    pub connection_side: ConnectionSide,
    pub dialects: Option<DialectSet>,
}
```

The compiler's (`rust/tcl-compiler/src/side_effects.rs`) is the richer,
per-invocation form the classifier produces — `lift_registry_effect` widens a
registry effect into it:

```rust
pub struct SideEffect {
    pub target: SideEffectTarget,
    pub reads: bool,
    pub writes: bool,
    pub storage_type: StorageType,     // data shape
    pub scope: StorageScope,           // where it lives
    pub connection_side: ConnectionSide, // F5 proxy context
    pub namespace: Option<String>,     // protocol namespace
    pub dialect: Option<String>,       // dialect context
    pub key: Option<String>,           // specific key (if literal)
    pub subtable: Option<String>,      // F5 subtable name
}
```

`SideEffect::new(target, reads, writes)` builds one with every other field at
its default (`StorageType::Unknown`, `StorageScope::Unknown`,
`ConnectionSide::None`, `None`), for chaining with struct-update syntax.

**`CommandSideEffects`** — the complete profile for one invocation:

```rust
pub struct CommandSideEffects {
    pub effects: Vec<SideEffect>,
    pub pure: bool,             // no observable side effects?
    pub deterministic: bool,    // same inputs → same outputs?
    pub dynamic_barrier: bool,  // contains eval/uplevel?
    pub dialect: Option<String>,
}
```

Convenience methods: `reads_any()`, `writes_any()`, `affects_target(t)`,
`writes_target(t)`, `reads_target(t)`, `effects_in_scope(s)`,
`effects_on_side(s)`, `to_effect_regions()`; plus the constructors
`pure()`, `unknown_write()`, and `dynamic_barrier()`.

### Scope stability semantics

`StorageScope` encodes not just *where* data lives but *how long* it is stable:

| Scope | Stability | Examples |
|-------|-----------|----------|
| `Connection` | Immutable for the life of the TCP/UDP flow | `IP::client_addr`, `TCP::client_port` |
| `Event` | Stable within a single `when` block; may change between events | `HTTP::uri`, `IP::server_addr`, `SSL::cert` |
| `Static` | System-wide, survives across connections | `static::` variables |
| `SessionTable` | Keyed, with explicit lifetime/timeout | `table` entries |
| `Persistence` | F5 persistence records | `session`/`persist` entries |

For a `Variable` effect the scope is derived from the name by
`scope_from_varname`: a `static::` prefix gives `Static`; a `::NAME` with no
further qualification gives `Global` with namespace `"::"`; a
`::NS::…::VAR` gives `Namespace` with the leading segments as the namespace;
everything else gives `ProcLocal`.

Key distinctions and what causes values to change:

- **Client-side transport** (`IP::client_addr`, `TCP::client_port`) uses `CONNECTION` — the client address and port are properties of the inbound TCP flow and never change for the life of the connection.
- **Server-side transport** (`IP::server_addr`, `TCP::server_port`) uses `EVENT` — stable within an event, but if the iRule selects a different pool or node between events, BIG-IP tears down and reconnects the server side, changing these values.
- **TLS state** (`SSL::cert`, `SSL::cipher`) uses `EVENT` — the TLS session is stable within an event, but an event handler can trigger a renegotiation (e.g. `SSL::authenticate`), which may produce a different client certificate or cipher suite in the next event.
- **HTTP request state** (`HTTP::uri`, `HTTP::header`, `HTTP::method`) uses `EVENT` — stable within `HTTP_REQUEST` or `HTTP_RESPONSE`, but each new HTTP transaction on a keepalive connection delivers entirely new request/response state.

For compiler analysis (which operates within a single event handler), both `CONNECTION` and `EVENT` scopes are effectively pure — the value cannot change during the analysis window. The scope annotation preserves the semantic distinction for documentation, cross-event reasoning, and future analyses that may need to track what persists across event boundaries.

### EffectRegion bridge

GVN and interprocedural analysis use coarse bitflag regions for fast kill checks. The `to_effect_regions()` method on `CommandSideEffects` maps structured effects to `EffectRegion`:

The mapping lives in `target_to_region(target, scope)`:

| EffectRegion | Mapped from |
|-------------|-------------|
| `HTTP_STATE` | `HttpHeader`, `HttpBody`, `HttpStatus`, `HttpUri`, `HttpCookie`, `HttpMethod`, `Http2State` |
| `RESPONSE_LIFECYCLE \| HTTP_STATE` | `ResponseCommit` — committing the response is also an HTTP-state write |
| `GLOBAL_STATE` | `Variable` with `Global` or `Namespace` scope |
| `NONE` | `Variable` in any other scope, and `FileIo` / `NetworkIo` / `LogIo` — external I/O does not mutate compiler-tracked in-memory state |
| `UNKNOWN_STATE` | Everything else, plus `dynamic_barrier` (added to the *writes* set by `to_effect_regions`) |

A `NONE` region is why a proc that only calls `puts` is impure
(`writes_any()` is true) yet has no tracked effect region.

## How hints are declared

### Command-level hints

Set `side_effects` on a `CommandSpec` to declare the default effects for a command:

```rust
CommandSpec {
    name: "pool",
    // ...
    side_effects: &[SideEffect {
        target: SideEffectTarget::PoolSelection,
        reads: false,
        writes: true,
        connection_side: ConnectionSide::Server,
        dialects: None,
    }],
    ..CommandSpec::DEFAULT
}
```

### Subcommand-level hints

For commands with subcommands that have different effect profiles, declare
effects on each `SubCommand`. Subcommand effects take precedence over
command-level ones:

```rust
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::at_least(0),
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];
```

Key interaction: on a command carrying `Traits::PURE`, a non-mutator
subcommand's declared effects are returned **read-only** (`reads` forced
`true`, `writes` forced `false`). This lets read-only subcommands like
`HTTP::header value` carry target metadata without being classified as
writers.

The ensemble arm behaves differently: when the *command* is not `PURE` but a
resolved subcommand is `pure: true` and not a mutator, the classifier returns
pure with **no effects at all** — that subcommand's declared targets do not
reach the result.

### Hint resolution order

`dialect_side_effect_hints(registry, command, subcommand, dialect)`
(`rust/tcl-compiler/src/side_effects.rs`) walks `registry.specs(command)` in
reverse (last registration wins), skipping any spec the active dialect
profile does not make available, and returns the first match:

1. The matched `SubCommand`'s own non-empty `side_effects`
2. The `CommandSpec`'s `side_effects` (fallback)

Each registry effect is widened by `lift_registry_effect`, which maps the
target and connection side into their compiler-side spellings, stamps the
dialect string, and leaves `storage_type` / `scope` / `namespace` / `key` /
`subtable` at their defaults for the classifier to fill in. The dialect name
`irules` is normalised to `f5-irules` before the availability check.

## Classification flow

```rust
pub fn classify_side_effects(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&str>,
    callee_summary: Option<&CalleeSummary>,
) -> CommandSideEffects
```

It follows this priority chain, returning at the first arm that matches:

1. **Callee summary** — when `callee_summary` is `Some`,
   `classify_from_callee_summary` translates the interprocedural
   `EffectRegion` pair straight into effects and skips the registry entirely.
2. **Unknown command** — `registry.get(command)` misses → conservative
   `Unknown` read+write.
3. **Dynamic barrier** — `Traits::EVALUATES_CODE | Traits::CREATES_BARRIER`
   (`eval`, `uplevel`, …) → `SideEffectTarget::Unknown` read+write with
   `dynamic_barrier: true`.
4. **Pure evaluation** — `Traits::PURE_EVALUATION` (braced `expr`) → pure and
   deterministic, unconditionally, with no subcommand or hint handling.
5. **Command-level purity** — `Traits::PURE`, with two refinements. A
   *mutator* subcommand (`HTTP::header insert …`) downgrades the command and
   falls through to the hints arm; a still-pure command returns its dialect
   hints forced read-only (`reads = true`, `writes = false`), so
   `HTTP::header` the getter keeps its target metadata.
6. **Subcommand-level purity** — an ensemble that is not itself `PURE`
   inherits purity from a resolved `SubCommand` with `pure: true` and
   `mutator: false` (`string length`, `dict get`, `array names`). Returns
   pure with *no* effects.
7. **Variable assignment** — `assigns_variable_at` → `classify_variable_assignment`,
   producing a `SideEffectTarget::Variable` effect with scope, namespace, key,
   and storage type inferred from the variable name and the command's traits.
8. **Procedure definition** — `Traits::DEFINES_PROCEDURE` → a
   `SideEffectTarget::ProcDefinition` write at `StorageScope::Namespace`.
9. **Variable destruction** — `Traits::DESTROYS_VARIABLE` (`unset`) → one
   `Variable` write per `ArgRole::VarWrite` argument, so `unset -nocomplain a b`
   keys both `a` and `b`; an option-only call still emits one untargeted
   `Variable` effect so the command stays impure.
10. **Hint fallback** — `dialect_side_effect_hints` returns the spec's
    structured `side_effects` (subcommand first).
11. **Conservative fallback** — `SideEffectTarget::Unknown` read+write.

There is no separate "protocol namespace" arm: an `HTTP::header` /
`SSL::cert` call is classified by arm 5 (pure getter, hints forced read-only)
or arm 10 (mutator subcommand, hints as declared).

### Execution traces are a separate gate, not part of classification

`classify_side_effects` takes no trace parameters and composes no trace
effects. The composition is a *purity gate* wrapped around it (issue #251):

- `trace add execution cmdName ops body` registers a Tcl-language hook
  that fires before/after every call to `cmdName`. A registry-pure `set` is
  no longer pure once `set` is traced, because the trace body runs around
  every invocation.
- Captured at lowering time: `Module::traced_commands`
  (`BTreeSet<String>`) is the *net active* set — a target is included when
  adds outnumber removes, modelling the global command-table state at
  end-of-script. `Module::has_dynamic_trace` (`bool`) reports whether any
  `add` had a non-literal target; when true, every command must be
  pessimised because we cannot prove a particular call is not traced.
- Optimisation passes that consult purity (GVN, partial redundancy,
  loop-invariant motion, optimiser propagation) thread the module's trace
  facts into `gvn::is_pure_command_with_traces`, which answers `false` for
  any command in `traced_commands`, and for *every* command when
  `has_dynamic_trace` is set, before delegating to `is_pure_command`. The
  gate is a flat "not pure"; it does not append an `Unknown` effect or
  rewrite the `CommandSideEffects` a classification returns.
- `trace remove execution` undoes the propagation by cancelling the
  matching `add` in the net count. `trace add command` and
  `trace add variable` are **not** captured here — they have different
  semantics (rename/delete and variable-access traces respectively)
  and do not compose into the traced command's per-call effects.
  `trace add variable` is instead unioned into SCCP's `escaping` set, and a
  dynamic variable-trace target makes `is_externally_mutable` answer `true`
  for every name.

## Examples of different side effect profiles

### Read-only data store access

```rust
// class command: reads data groups, never writes
SideEffect {
    target: SideEffectTarget::DataGroup,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::Global,
    dialects: None,
}
```

### Write-only connection control

```rust
// drop command: terminates the connection
SideEffect {
    target: SideEffectTarget::ConnectionControl,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Both,
    dialects: None,
}
```

### Response commit

```rust
// HTTP::respond: commits the HTTP response
SideEffect {
    target: SideEffectTarget::ResponseCommit,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Client,
    dialects: None,
}
```

### Read+write with arity-dependent behaviour

For HTTP namespace commands where getter vs setter depends on arguments, the
command-level effect declares `reads: true, writes: true` (conservative), and
the `mutator` flag on subcommands narrows the writes:

```rust
// HTTP::header — command-level effect (conservative)
SideEffect {
    target: SideEffectTarget::HttpHeader,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::Client,
    dialects: None,
}
// Subcommand "value" is pure → classifier narrows to read-only
// Subcommand "replace" has mutator: true → classifier keeps writes: true
```

### Logging / output

```rust
// log command: writes to log output
SideEffect {
    target: SideEffectTarget::LogIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}
```

### Load balancing

```rust
// pool command: selects a pool member
SideEffect {
    target: SideEffectTarget::PoolSelection,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Server,
    dialects: None,
}
```

## How to add hints to a new command

1. **Identify the target** — what external resource does the command touch? Pick from `SideEffectTarget`.
2. **Determine reads/writes** — does it read, write, or both? For commands with subcommands, does it vary?
3. **Choose the connection side** — which F5 proxy side does it act on? Pick
   from `ConnectionSide`: `Client`, `Server`, `Both`, `Global`, or `None`.
   `StorageScope` and `StorageType` are inferred by the classifier, not
   declared on the spec.
4. **Add to CommandSpec** — set `side_effects: &[SideEffect { … }]` on the spec.
5. **Add SubCommand entries if needed** — if subcommands have different read/write profiles, set `side_effects` on each `SubCommand`.
6. **Mark pure subcommands** — set `pure: true` on read-only subcommands. On a `PURE` command the classifier forces its hints read-only; on an ensemble a pure subcommand makes the call pure with no effects.
7. **Mark mutator subcommands** — set `mutator: true` on write subcommands. This is what stops a `PURE` command's mutating subcommand from being classified pure, sending it to the hints arm instead.

### Checklist

- [ ] `side_effects` slice on `CommandSpec`
- [ ] Subcommand hints where read/write varies
- [ ] `pure: true` on read-only subcommands
- [ ] `mutator: true` on write subcommands

## File-path anchors

- `rust/tcl-compiler/src/side_effects.rs` — `StorageType` / `StorageScope`,
  the compiler-side `SideEffect` and `CommandSideEffects`,
  `classify_side_effects`, `dialect_side_effect_hints`,
  `lift_registry_effect`, `scope_from_varname`, and the `target_to_region`
  `EffectRegion` bridge
- `rust/tcl-registry/src/side_effects.rs` — `SideEffectTarget`,
  `ConnectionSide`, and the registry-side `SideEffect`
- `rust/tcl-registry/src/spec.rs` — `CommandSpec::side_effects`, `SubCommand::side_effects`
- `rust/tcl-compiler/src/ir.rs` — `Module::traced_commands` and `Module::has_dynamic_trace`
- `rust/tcl-compiler/src/lowering/mod.rs` — `trace add`/`trace remove execution` capture
- `rust/tcl-compiler/src/gvn.rs` — `is_pure_command`, and the trace gate
  `is_pure_command_with_traces`
- `rust/tcl-compiler/src/interprocedural.rs` — interprocedural consumer
- `rust/tcl-compiler/src/irules_checks.rs` — response-commit derivation, reading
  the registry `SideEffect` declarations directly
- `rust/tcl-compiler/src/sccp.rs` — purity consumer via the GVN trace gate
- `rust/tcl-compiler/src/optimiser/propagation.rs` — load-forwarding consumer
  (threads `traced_commands` via `ctx.ir_module`)
- `rust/tcl-compiler/src/optimiser/elimination.rs` — dead-store consumer

## Failure modes

- **Missing effects** — command falls through to conservative `SideEffectTarget::Unknown` read+write. GVN will not optimise around it. Fix: add `side_effects` to the command's registry spec.
- **Missing subcommand effects** — read-only subcommand inherits the command's conservative read+write effect. Fix: add per-subcommand `side_effects` with `pure: true` on read-only subcommands.
- **Wrong `connection_side`** — iRules flow checker may fail to track response commits or connection drops on the correct side. Fix: set `connection_side` to match the F5 documentation.
- **Pure subcommand without effects** — classifier returns `pure: true` with no effects. Target metadata is lost. Fix: add `side_effects` to the subcommand so the classifier can include read-only effects.
- **Hint on dynamic barrier command** — hints are ignored; dynamic barriers always produce `UNKNOWN` read+write. This is correct — do not add hints to `eval`/`uplevel`.

## Tests

- `rust/tcl-compiler/src/side_effects.rs` unit tests — the classification arms
- `rust/tcl-compiler/src/gvn.rs` unit tests — the `traced_commands` / `has_dynamic_trace` purity gate (issue #251)
- `rust/tcl-compiler/src/lowering/mod.rs` unit tests — `trace add` / `trace remove execution` capture

## See also

- [Compiler KCS index](README.md)
- [KCS index](../README.md)
- [Pipeline overview](../../../docs/design/compiler/compiler-pipeline-overview.md)
