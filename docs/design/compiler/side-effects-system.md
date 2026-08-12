# Side-effects classification system

How the compiler describes what a command reads, writes, and touches, and how
to add side-effect metadata for a new or existing command.

The side-effects system is the single source of truth for all effect classification in the compiler. It is consumed by:

- **GVN** (`gvn.rs`) — kill safety for common subexpression elimination.
- **Interprocedural analysis** (`interprocedural.rs`) — procedure summaries across call boundaries.
- **iRules flow checker** (`irules_checks.rs`) — response-commit and connection-drop tracking.
- **Execution intent** (`execution_intent.rs`) — purity classification for command substitution intent.
- **Core analyses** (`sccp.rs`, `analyses.rs`) — purity checks for constant propagation.

All classification flows through a single function: `classify_side_effects` in `side_effects.rs`.

## Architecture

### Enums (the vocabulary)

Four enums describe the dimensions of a side effect:

| Enum | Describes | Example values |
|------|-----------|----------------|
| `SideEffectTarget` | *What* resource is touched | `HttpHeader`, `SessionTable`, `Variable`, `PoolSelection` |
| `StorageScope` | *Where* the data lives / stability | `ProcLocal`, `Global`, `Event`, `Connection`, `SessionTable` |
| `ConnectionSide` | *Which* F5 proxy side | `Client`, `Server`, `Both`, `Global`, `None` |
| `StorageType` | *Shape* of the data | `Scalar`, `List`, `Dict`, `Array` |

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
| `CONNECTION` | Immutable for the life of the TCP/UDP flow | `IP::client_addr`, `TCP::client_port` |
| `EVENT` | Stable within a single `when` block; may change between events | `HTTP::uri`, `IP::server_addr`, `SSL::cert` |
| `STATIC` | System-wide, survives across connections | `static::` variables |
| `SESSION_TABLE` | Keyed, with explicit lifetime/timeout | `table` entries |
| `PERSISTENCE` | F5 persistence records | `session`/`persist` entries |

Key distinctions and what causes values to change:

- **Client-side transport** (`IP::client_addr`, `TCP::client_port`) uses `CONNECTION` — the client address and port are properties of the inbound TCP flow and never change for the life of the connection.
- **Server-side transport** (`IP::server_addr`, `TCP::server_port`) uses `EVENT` — stable within an event, but if the iRule selects a different pool or node between events, BIG-IP tears down and reconnects the server side, changing these values.
- **TLS state** (`SSL::cert`, `SSL::cipher`) uses `EVENT` — the TLS session is stable within an event, but an event handler can trigger a renegotiation (e.g. `SSL::authenticate`), which may produce a different client certificate or cipher suite in the next event.
- **HTTP request state** (`HTTP::uri`, `HTTP::header`, `HTTP::method`) uses `EVENT` — stable within `HTTP_REQUEST` or `HTTP_RESPONSE`, but each new HTTP transaction on a keepalive connection delivers entirely new request/response state.

For compiler analysis (which operates within a single event handler), both `CONNECTION` and `EVENT` scopes are effectively pure — the value cannot change during the analysis window. The scope annotation preserves the semantic distinction for documentation, cross-event reasoning, and future analyses that may need to track what persists across event boundaries.

### EffectRegion bridge

GVN and interprocedural analysis use coarse bitflag regions for fast kill checks. The `to_effect_regions()` method on `CommandSideEffects` maps structured effects to `EffectRegion`:

| EffectRegion | Mapped from |
|-------------|-------------|
| `HTTP_STATE` | `HTTP_HEADER`, `HTTP_BODY`, `HTTP_STATUS`, `HTTP_URI`, `HTTP_COOKIE`, `HTTP_METHOD`, `HTTP2_STATE` |
| `RESPONSE_LIFECYCLE` | `RESPONSE_COMMIT` |
| `GLOBAL_STATE` | `VARIABLE` with `GLOBAL` or `NAMESPACE` scope |
| `UNKNOWN_STATE` | Everything else, plus dynamic barriers |

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

Key interaction: when a subcommand is marked `pure: true` and declares
effects, the classifier includes them as **read-only** (writes forced to
`false`). This allows read-only subcommands like `table lookup` or
`session count` to carry target metadata without being classified as writers.

### Hint resolution order

`dialect_side_effect_hints(registry, command, subcommand, dialect)`
(`rust/tcl-compiler/src/side_effects.rs`) resolves effects in this order,
skipping any spec that is not in the active dialect:

1. The matched `SubCommand`'s own non-empty `side_effects`
2. The `CommandSpec`'s `side_effects` (fallback)

## Classification flow

`classify_side_effects(command, args)` follows this priority chain:

1. **Callee summary** — interprocedural summaries bypass registry lookup entirely.
2. **Dynamic barrier** — commands like `eval`, `uplevel`, `subst` produce `SideEffectTarget::Unknown` read+write with `dynamic_barrier: true`.
3. **Purity** — pure commands return `pure: true`. Protocol namespace pure commands include a read effect. Pure subcommands with declared effects include read-only effects.
4. **Variable assignment** — commands with `assigns_variable_at` produce a `SideEffectTarget::Variable` effect with scope/key/storage_type inferred from the variable name and command.
5. **Protocol namespace** — `HTTP::header`, `SSL::cert`, etc. are classified using the declared target plus subcommand `mutator` flags.
6. **Procedure definition** — `proc`, `rename` produce a `SideEffectTarget::ProcDefinition` write.
7. **Hint fallback** — if a hint exists, use it directly.
8. **Conservative fallback** — `SideEffectTarget::Unknown` read+write.

After the registry-based classification produces a base result, the
**execution-trace composition rule** runs as a post-processing step
(see issue #251):

- `trace add execution cmdName ops body` registers a Tcl-language hook
  that fires before/after every call to `cmdName`. The trace body's
  effects compose into the traced command's effective per-call
  side-effects: a registry-pure `set` is no longer pure once `set` is
  traced, because the trace body runs around every invocation.
- Captured at lowering time: `Module::traced_commands`
  (`BTreeSet<String>`) is the *net active* set — a target is included when
  adds outnumber removes, modelling the global command-table state at
  end-of-script. `Module::has_dynamic_trace` (`bool`) reports whether any
  `add` had a non-literal target; when true, every command must be
  pessimised because we cannot prove a particular call isn't traced.
- Optimisation passes that consult purity (GVN, optimiser propagation)
  pass `traced_commands` and `has_dynamic_trace` into
  `classify_side_effects`. The composition is conservative today: a
  matching command gets an extra `SideEffectTarget::Unknown` read+write
  effect appended, and `pure` / `deterministic` are forced to `false`.
  A future refinement can replace the catch-all with the trace body's
  recursively-classified effects.
- `trace remove execution` undoes the propagation by cancelling the
  matching `add` in the net count. `trace add command` and
  `trace add variable` are **not** captured here — they have different
  semantics (rename/delete and variable-access traces respectively)
  and do not compose into the traced command's per-call effects.

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
   from `ConnectionSide`. (`StorageScope` and `StorageType` are inferred by
   the classifier, not declared on the spec.)
4. **Set connection side** — for F5 commands, which proxy side? `CLIENT`, `SERVER`, `BOTH`, or `GLOBAL`.
5. **Add to CommandSpec** — set `side_effects: &[SideEffect { … }]` on the spec.
6. **Add SubCommand entries if needed** — if subcommands have different read/write profiles, set `side_effects` on each `SubCommand`.
7. **Mark pure subcommands** — set `pure: true` on read-only subcommands. The classifier will automatically narrow their effects to read-only.
8. **Mark mutator subcommands** — set `mutator: true` on write subcommands. The protocol namespace classifier uses this to upgrade writes.

### Checklist

- [ ] `side_effects` slice on `CommandSpec`
- [ ] Subcommand hints where read/write varies
- [ ] `pure: true` on read-only subcommands
- [ ] `mutator: true` on write subcommands

## File-path anchors

- `side_effects.rs` — the effect enums, `classify_side_effects`, trace composition, and the `EffectRegion` bridge
- `rust/tcl-registry/src/spec.rs` — `CommandSpec::side_effects`, `SubCommand::side_effects`
- `rust/tcl-registry/src/side_effects.rs` — the registry-side `SideEffect`
- `rust/tcl-compiler/src/side_effects.rs` — `dialect_side_effect_hints` lookup, `lift_registry_effect`
- `ir.rs` — `CommandTrace`, the module's traced-command set, and the dynamic-trace flag
- `lowering/` — `trace add`/`trace remove execution` capture
- `gvn.rs` — GVN consumer (threads the module's traced commands)
- `interprocedural.rs` — interprocedural consumer
- `irules_checks.rs` — response-commit and drop-command derivation
- `execution_intent.rs` — purity consumer
- `sccp.rs` — purity consumer
- `rust/tcl-compiler/src/optimiser/propagation.rs` — load-forwarding consumer (threads `traced_commands` via `ctx.ir_module`)

## Failure modes

- **Missing effects** — command falls through to conservative `SideEffectTarget::Unknown` read+write. GVN will not optimise around it. Fix: add `side_effects` to the command's registry spec.
- **Missing subcommand effects** — read-only subcommand inherits the command's conservative read+write effect. Fix: add per-subcommand `side_effects` with `pure: true` on read-only subcommands.
- **Wrong `connection_side`** — iRules flow checker may fail to track response commits or connection drops on the correct side. Fix: set `connection_side` to match the F5 documentation.
- **Pure subcommand without effects** — classifier returns `pure: true` with no effects. Target metadata is lost. Fix: add `side_effects` to the subcommand so the classifier can include read-only effects.
- **Hint on dynamic barrier command** — hints are ignored; dynamic barriers always produce `UNKNOWN` read+write. This is correct — do not add hints to `eval`/`uplevel`.

## Test anchors

- `side_effects.rs` unit tests — `trace add`/`trace remove execution` capture and side-effect composition (issue #251)

## Discoverability

- [Compiler KCS index](README.md)
- [KCS index](../README.md)
- [Pipeline overview](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [Execution intent model](../../../docs/design/compiler/execution-intent-model.md)
