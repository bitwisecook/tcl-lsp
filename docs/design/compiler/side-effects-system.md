# KCS: Side-effects classification system

## Symptom

A contributor needs to understand how the compiler determines what a command reads, writes, and touches — or needs to add side-effect metadata for a new or existing command.

## Operational context

The side-effects system is the single source of truth for all effect classification in the compiler. It is consumed by:

- **GVN** (`rust/tcl-compiler/src/gvn.rs`) — kill safety for common subexpression elimination.
- **Interprocedural analysis** (`rust/tcl-compiler/src/interprocedural.rs`) — procedure summaries across call boundaries.
- **iRules flow checker** (`rust/tcl-compiler/src/irules_checks.rs`) — response-commit and connection-drop tracking.
- **Execution intent** (`rust/tcl-compiler/src/execution_intent.rs`) — purity classification for command substitution intent.
- **Optimiser elimination / propagation** (`rust/tcl-compiler/src/optimiser/elimination.rs`, `.../propagation.rs`) — purity checks for dead-store elimination and load forwarding.

The model is split across two crates:

- `rust/tcl-registry/src/side_effects.rs` holds the *lightweight declared*
  types the command metadata carries — the `SideEffect` a `CommandSpec` /
  `SubCommand` declares, plus `SideEffectTarget`, `ConnectionSide`, and
  `StorageType`.
- `rust/tcl-compiler/src/side_effects.rs` holds the *richer inferred* types
  and all classification. Everything flows through one function,
  `classify_side_effects()`.

## Architecture

### Enums (the vocabulary)

Four enums describe the dimensions of a side effect (the compiler-side
spellings; the registry's declared subset uses the same variant names):

| Enum | Describes | Example variants |
|------|-----------|------------------|
| `SideEffectTarget` | *What* resource is touched | `HttpHeader`, `SessionTable`, `Variable`, `PoolSelection` |
| `StorageScope` | *Where* the data lives / stability | `ProcLocal`, `Global`, `Event`, `Connection`, `SessionTable` |
| `ConnectionSide` | *Which* F5 proxy side | `Client`, `Server`, `Both`, `Global`, `None` |
| `StorageType` | *Shape* of the data | `Scalar`, `List`, `Dict`, `Array` |

`StorageScope` is compiler-only — the registry does not declare it; it is
inferred during classification from the variable-name prefix and the
command.

### Structs

**`SideEffect`** (compiler) — one discrete read or write:

```rust
pub struct SideEffect {
    /// What category of resource is touched.
    pub target: SideEffectTarget,
    /// Whether this effect includes a read from the target.
    pub reads: bool,
    /// Whether this effect includes a write to the target.
    pub writes: bool,
    /// Data shape of the target (scalar, list, dict, array).
    pub storage_type: StorageType,
    /// Where the data resides (proc-local, global, F5 table, …).
    pub scope: StorageScope,
    /// F5 proxy context for this effect.
    pub connection_side: ConnectionSide,
    /// Tcl namespace or F5 protocol namespace (e.g. `"HTTP"`).
    pub namespace: Option<String>,
    /// Dialect this effect applies to (`"irules"`, `"tcl"`, …).
    pub dialect: Option<String>,
    /// Optional key identifying the specific target.
    pub key: Option<String>,
    /// F5 session-table subtable name, if applicable.
    pub subtable: Option<String>,
}
```

`SideEffect::new(target, reads, writes)` builds one with every remaining
field at its default (`StorageType::Unknown`, `StorageScope::Unknown`,
`ConnectionSide::None`, `None` elsewhere), so classifier code can chain it
with struct-update syntax.

The registry's declared counterpart is deliberately smaller and `Copy` — it
carries only `target`, `reads`, `writes`, `connection_side`, and an optional
`dialects: Option<DialectSet>` narrowing (for effects that exist only in
some of the command's dialects), with a `SideEffect::DEFAULT` constant for
`..`-updates. `lift_registry_effect` widens it into the compiler type.

**`CommandSideEffects`** — the complete profile for one invocation:

```rust
pub struct CommandSideEffects {
    /// Individual side effects produced by this invocation.
    pub effects: Vec<SideEffect>,
    /// No observable side effects (reads from immutable state OK).
    pub pure: bool,
    /// Same inputs always produce the same outputs.
    pub deterministic: bool,
    /// Contains `eval`/`uplevel`/`call` — effects are unknowable.
    pub dynamic_barrier: bool,
    /// Dialect context in which this classification was made.
    pub dialect: Option<String>,
}
```

Constructors: `CommandSideEffects::pure()`, `::unknown_write()`,
`::dynamic_barrier()`.  Query methods: `reads_any()`, `writes_any()`,
`affects_target(t)`, `writes_target(t)`, `reads_target(t)`,
`effects_in_scope(s)`, `effects_on_side(s)`, `to_effect_regions()`.

### Scope stability semantics

`StorageScope` encodes not just *where* data lives but *how long* it is stable:

| Scope | Stability | Examples |
|-------|-----------|----------|
| `Connection` | Immutable for the life of the TCP/UDP flow | `IP::client_addr`, `TCP::client_port` |
| `Event` | Stable within a single `when` block; may change between events | `HTTP::uri`, `IP::server_addr`, `SSL::cert` |
| `Static` | System-wide, survives across connections | `static::` variables |
| `SessionTable` | Keyed, with explicit lifetime/timeout | `table` entries |
| `Persistence` | F5 persistence records | `session`/`persist` entries |

Key distinctions and what causes values to change:

- **Client-side transport** (`IP::client_addr`, `TCP::client_port`) uses `Connection` — the client address and port are properties of the inbound TCP flow and never change for the life of the connection.
- **Server-side transport** (`IP::server_addr`, `TCP::server_port`) uses `Event` — stable within an event, but if the iRule selects a different pool or node between events, BIG-IP tears down and reconnects the server side, changing these values.
- **TLS state** (`SSL::cert`, `SSL::cipher`) uses `Event` — the TLS session is stable within an event, but an event handler can trigger a renegotiation (e.g. `SSL::authenticate`), which may produce a different client certificate or cipher suite in the next event.
- **HTTP request state** (`HTTP::uri`, `HTTP::header`, `HTTP::method`) uses `Event` — stable within `HTTP_REQUEST` or `HTTP_RESPONSE`, but each new HTTP transaction on a keepalive connection delivers entirely new request/response state.

For compiler analysis (which operates within a single event handler), both `Connection` and `Event` scopes are effectively pure — the value cannot change during the analysis window. The scope annotation preserves the semantic distinction for documentation, cross-event reasoning, and future analyses that may need to track what persists across event boundaries.

### EffectRegion bridge

GVN and interprocedural analysis use coarse bitflag regions for fast kill checks. `CommandSideEffects::to_effect_regions()` returns a `(reads, writes)` pair of `EffectRegion` bitflags, mapping each effect through `target_to_region(target, scope)`:

| EffectRegion | Mapped from |
|-------------|-------------|
| `HTTP_STATE` | `HttpHeader`, `HttpBody`, `HttpStatus`, `HttpUri`, `HttpCookie`, `HttpMethod`, `Http2State` |
| `RESPONSE_LIFECYCLE` | `ResponseCommit` (which also sets `HTTP_STATE`) |
| `GLOBAL_STATE` | `Variable` with `Global` or `Namespace` scope |
| `NONE` | `Variable` in any other scope, and `FileIo` / `NetworkIo` / `LogIo` — external I/O does not mutate compiler-tracked in-memory state |
| `UNKNOWN_STATE` | Everything else, plus dynamic barriers |

## How effects are declared

### Command-level declarations

Set `side_effects` on a `CommandSpec` to declare the default effects for a command (`rust/tcl-registry/src/commands/irules/pool.rs`):

```rust
side_effects: &[SideEffect {
    target: SideEffectTarget::PoolSelection,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Server,
    dialects: None,
}],
```

### Subcommand-level declarations

For commands with subcommands that have different effect profiles, declare `side_effects` on each `SubCommand`. Subcommand declarations take precedence over the command-level ones (`rust/tcl-registry/src/commands/irules/table.rs`):

```rust
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        synopsis: "table add ?-notouch? ?-subtable name | -georedundancy? key value ?timeout ?lifetime??",
        mutator: true,
        options: SUBTABLE_OPTIONS,
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
        synopsis: "table lookup ?-notouch? ?-subtable name | -georedundancy? key",
        options: SUBTABLE_OPTIONS,
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

Key interaction: when a subcommand carries `pure: true` and a declaration, the classifier includes those effects as **read-only** (writes forced to `false`). This allows read-only subcommands to carry target metadata without being classified as writers.

### Resolution order

`dialect_side_effect_hints(registry, command, subcommand, dialect)` walks `registry.specs(command)` in reverse (so a later-registered overlay wins), skipping specs that do not apply in the active dialect, and returns the first non-empty declaration it finds:

1. Subcommand-level `side_effects` (when `resolve_subcommand` matches and the slice is non-empty)
2. Command-level `side_effects` (fallback)

Note the dialect alias: a caller passing `Some("irules")` is remapped to the registry's `"f5-irules"` gate.

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

It follows this priority chain:

1. **Callee summary** — a `Some(CalleeSummary)` from interprocedural analysis bypasses registry lookup entirely; its `EffectRegion` pair is translated straight into effects.
2. **Unknown command** — no registry spec ⇒ conservative `Unknown` read+write.
3. **Dynamic barrier** — a spec with `Traits::EVALUATES_CODE` or `Traits::CREATES_BARRIER` produces `Unknown` read+write with `dynamic_barrier: true`.
4. **Pure evaluation** — `Traits::PURE_EVALUATION` (braced `expr`) is unconditionally pure, checked before subcommand or declaration handling.
5. **Purity** — `Traits::PURE` commands return `pure: true`; a pure command with a `mutator: true` subcommand is downgraded to impure, and a still-pure command surfaces its declaration read-only.
6. **Variable assignment** — `assigns_variable_at` produces a `Variable` effect with scope/key/storage type inferred from the variable name and command; `Traits::DESTROYS_VARIABLE` and `Traits::DEFINES_PROCEDURE` have their own arms.
7. **Declaration fallback** — `dialect_side_effect_hints` as above.
8. **Conservative fallback** — `Unknown` read+write.

Full arity-based form resolution and per-subcommand protocol-namespace write modelling are **not** implemented today.

### Execution traces

`classify_side_effects` itself is trace-unaware. Trace composition is a
separate, whole-module layer applied by the purity gate the optimisation
passes call (see issue #251):

- `trace add execution cmdName ops handler` registers a Tcl-language hook
  that fires before/after every call to `cmdName`. The handler's effects
  compose into the traced command's effective per-call side effects: a
  registry-pure `expr` is no longer pure once `expr` is traced, because the
  handler runs around every invocation.
- Captured post-lowering by `populate_trace_facts`
  (`rust/tcl-compiler/src/lowering/mod.rs`), which walks the top-level
  script plus every procedure body, `body_unit` (`namespace eval` / `apply`
  bodies), and `TclOO` method body. It records into two `Module` fields:
  a **literal** target lands in `Module::traced_commands:
  BTreeSet<String>` (leading `::` stripped so the gate's canonical lookup
  hits); a **non-literal** target (`$cmd`, a command substitution) flips
  `Module::has_dynamic_trace: bool`.
- The scan is whole-module and position-independent: a trace installed
  inside a `proc` body counts even though that body may never run before a
  given call site. It resolves the command head through the alias table
  (`canonical_command_or_source`) and accepts C Tcl's unique-prefix
  abbreviation of the type word, so `trace add e foo enter h` is recognised
  the same as the full spelling.
- The consumer is `gvn::is_pure_command_with_traces(registry, command,
  args, dialect, traced_commands, has_dynamic_trace)`, threaded by GVN,
  partial redundancy, and the loop-invariant passes. It is conservative and
  binary: `has_dynamic_trace` makes *every* command impure, and a name in
  `traced_commands` makes that command impure. The trace handler's body is
  not recursively classified.
- Only the `add` subcommand is recorded — there is no net add-minus-remove
  count, so a later `trace remove execution` does not restore purity. This
  is the safe direction.
- Variable traces are a **separate channel** with a different consumer:
  `Module::traced_variables` / `Module::has_dynamic_variable_trace`,
  populated from the registry's `Traits::ESTABLISHES_VARIABLE_TRACE`
  subcommands (covering both the modern `trace add variable` and the
  deprecated `trace variable` / `vdelete` spellings). They inhibit O102
  load forwarding and dead-store elimination rather than command purity.

## Examples of different side effect profiles

All examples are the real declarations, verbatim from
`rust/tcl-registry/src/commands/irules/`.

### Read-only data store access

```rust
// class — reads data groups, never writes
side_effects: &[SideEffect {
    target: SideEffectTarget::DataGroup,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::Global,
    dialects: None,
}],
```

### Write-only connection control

```rust
// drop — terminates the connection
side_effects: &[SideEffect {
    target: SideEffectTarget::ConnectionControl,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
```

### Response commit

```rust
// HTTP::respond — commits the HTTP response
side_effects: &[SideEffect {
    target: SideEffectTarget::ResponseCommit,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::Client,
    dialects: None,
}],
```

### Read+write with arity-dependent behaviour

For HTTP namespace commands where getter versus setter depends on the
arguments, the command-level declaration is conservative (`reads: true,
writes: true`) and the `mutator` flag on subcommands narrows it:

```rust
// HTTP::header — command-level declaration (conservative)
side_effects: &[SideEffect {
    target: SideEffectTarget::HttpHeader,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
// Subcommand "value" has no `mutator` → classifier narrows to read-only
// Subcommand "replace" has `mutator: true` → classifier keeps writes
```

### Logging / output

```rust
// log — writes to log output
side_effects: &[SideEffect {
    target: SideEffectTarget::LogIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
```

Note that `LogIo` maps to `EffectRegion::NONE`: the command is impure
(`writes_any()` is true) but has no compiler-tracked region to kill.

### Load balancing

```rust
// pool — selects a pool member
side_effects: &[SideEffect {
    target: SideEffectTarget::PoolSelection,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Server,
    dialects: None,
}],
```

## How to add effects to a new command

1. **Identify the target** — what external resource does the command touch? Pick a `SideEffectTarget` variant.
2. **Determine reads/writes** — does it read, write, or both? For commands with subcommands, does it vary?
3. **Set connection side** — for F5 commands, which proxy side? `Client`, `Server`, `Both`, or `Global`.
4. **Narrow the dialect if needed** — set `dialects: Some(...)` only when the *effect* applies in fewer dialects than the command itself; `None` inherits the command's gating.
5. **Add to the `CommandSpec`** — set `side_effects: &[SideEffect { … }]` on the spec, using `..SideEffect::DEFAULT` for the fields you do not care about.
6. **Add `SubCommand` entries if needed** — if subcommands have different read/write profiles, give each its own `side_effects` slice.
7. **Mark pure subcommands** — set `pure: true` on read-only subcommands. The classifier narrows their declared effects to read-only.
8. **Mark mutator subcommands** — set `mutator: true` on write subcommands. This is what downgrades an otherwise-`PURE` parent command to impure.

Scope is *not* declared here — `StorageScope` is inferred during
classification, so there is nothing to set on the spec.

### Checklist

- [ ] `side_effects` slice on the `CommandSpec`
- [ ] Subcommand `side_effects` where read/write varies
- [ ] `pure: true` on read-only subcommands
- [ ] `mutator: true` on write subcommands
- [ ] `cargo test -p tcl-compiler side_effects` to verify classification

## File-path anchors

- `rust/tcl-compiler/src/side_effects.rs` — inferred enums, `SideEffect`, `CommandSideEffects`, `classify_side_effects()`, `target_to_region()`, the `EffectRegion` bridge
- `rust/tcl-registry/src/side_effects.rs` — the declared `SideEffect`, `SideEffectTarget`, `ConnectionSide`, `StorageType`, `SideSwitchTarget`
- `rust/tcl-registry/src/spec.rs` — `CommandSpec::side_effects`, `SubCommand::side_effects`, `SubCommand::pure` / `mutator`
- `rust/tcl-registry/src/commands/` — the per-command spec modules that carry the declarations
- `rust/tcl-compiler/src/ir.rs` — `Module::traced_commands`, `has_dynamic_trace`, `traced_variables`, `has_dynamic_variable_trace`
- `rust/tcl-compiler/src/lowering/mod.rs` — `populate_trace_facts` / `walk_for_trace` (`trace add execution` capture)
- `rust/tcl-compiler/src/gvn.rs` — GVN consumer, `is_pure_command_with_traces`
- `rust/tcl-compiler/src/interprocedural.rs` — interprocedural consumer
- `rust/tcl-compiler/src/irules_checks.rs` — response-commit and drop-command derivation
- `rust/tcl-compiler/src/execution_intent.rs` — purity consumer
- `rust/tcl-compiler/src/optimiser/elimination.rs` — dead-store / dead-call purity consumer
- `rust/tcl-compiler/src/optimiser/propagation.rs` — load-forwarding consumer (threads the module's `traced_variables` facts)

## Failure modes

- **Missing declaration** — command falls through to the conservative `Unknown` read+write. GVN will not optimise around it. Fix: add `side_effects` to the command's registry spec.
- **Missing subcommand declaration** — a read-only subcommand inherits the command's conservative read+write declaration. Fix: add per-subcommand `side_effects` with `pure: true` on the read-only ones.
- **Wrong `connection_side`** — the iRules flow checker may fail to track response commits or connection drops on the correct side. Fix: set `connection_side` to match the F5 documentation.
- **Pure subcommand without a declaration** — the classifier returns `pure: true` with no effects, so the target metadata is lost. Fix: add a `side_effects` slice to the subcommand so the classifier can surface read-only effects.
- **Declaration on a dynamic-barrier command** — ignored; a `Traits::EVALUATES_CODE` / `CREATES_BARRIER` command always produces `Unknown` read+write. This is correct — do not declare effects on `eval` / `uplevel`.

## Test anchors

- `rust/tcl-compiler/src/side_effects.rs` — `mod tests` (`classify_side_effects` classification cases)
- `rust/tcl-compiler/tests/side_effects_binding.rs` — registry-declaration binding
- `rust/tcl-compiler/src/lowering/mod.rs` — `mod tests`, the `trace_add_execution_*` cases (module-fact capture, issue #251)
- `rust/tcl-compiler/src/gvn.rs` — `mod tests`, the `is_pure_command_with_traces_*` cases (purity gating)

## Discoverability

- [Compiler KCS index](README.md)
- [KCS index](../README.md)
- [Pipeline overview](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [Execution intent model](../../../docs/design/compiler/execution-intent-model.md)
