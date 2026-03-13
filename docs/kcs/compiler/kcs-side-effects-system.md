# KCS: Side-effects classification system

## Symptom

A contributor needs to understand how the compiler determines what a command reads, writes, and touches — or needs to add side-effect metadata for a new or existing command.

## Operational context

The side-effects system is the single source of truth for all effect classification in the compiler. It is consumed by:

- **GVN** (`core/compiler/gvn.py`) — kill safety for common subexpression elimination.
- **Interprocedural analysis** (`core/compiler/interprocedural.py`) — procedure summaries across call boundaries.
- **iRules flow checker** (`core/compiler/irules_flow.py`) — response-commit and connection-drop tracking.
- **Execution intent** (`core/compiler/execution_intent.py`) — purity classification for command substitution intent.
- **Core analyses** (`core/compiler/core_analyses.py`) — purity checks for constant propagation.

All classification flows through a single function: `classify_side_effects()` in `core/compiler/side_effects.py`.

## Architecture

### Enums (the vocabulary)

Four enums describe the dimensions of a side effect:

| Enum | Describes | Example values |
|------|-----------|----------------|
| `SideEffectTarget` | *What* resource is touched | `HTTP_HEADER`, `SESSION_TABLE`, `VARIABLE`, `POOL_SELECTION` |
| `StorageScope` | *Where* the data lives / stability | `PROC_LOCAL`, `GLOBAL`, `EVENT`, `CONNECTION`, `SESSION_TABLE` |
| `ConnectionSide` | *Which* F5 proxy side | `CLIENT`, `SERVER`, `BOTH`, `GLOBAL`, `NONE` |
| `StorageType` | *Shape* of the data | `SCALAR`, `LIST`, `DICT`, `ARRAY` |

### Dataclasses

**`SideEffect`** — one discrete read or write:

```python
SideEffect(
    target=SideEffectTarget.HTTP_HEADER,  # what
    reads=True,                            # reads from it?
    writes=True,                           # writes to it?
    scope=StorageScope.CONNECTION,         # where it lives
    connection_side=ConnectionSide.CLIENT,  # F5 proxy context
    storage_type=StorageType.SCALAR,       # data shape
    namespace="HTTP",                      # protocol namespace
    key="Host",                            # specific key (if literal)
    dialect="irules",                      # dialect context
    subtable=None,                         # F5 subtable name
)
```

Only `target` is required. All other fields have sensible defaults (`reads=False`, `writes=False`, `scope=UNKNOWN`, `connection_side=NONE`, etc.).

**`CommandSideEffects`** — the complete profile for one invocation:

```python
CommandSideEffects(
    effects=(effect1, effect2, ...),  # tuple of SideEffect
    pure=False,                        # no observable side effects?
    deterministic=False,               # same inputs → same outputs?
    dynamic_barrier=False,             # contains eval/uplevel?
    dialect="irules",                  # dialect context
)
```

Convenience properties: `reads_any`, `writes_any`, `targets`, `write_targets`, `read_targets`, `scopes`, `affects_target(t)`, `writes_target(t)`, `reads_target(t)`, `effects_in_scope(s)`, `effects_on_side(s)`.

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

Add `side_effect_hints` to a `CommandSpec` to declare the default effects for a command:

```python
CommandSpec(
    name="pool",
    # ...
    side_effect_hints=(
        SideEffect(
            target=SideEffectTarget.POOL_SELECTION,
            writes=True,
            scope=StorageScope.CONNECTION,
            connection_side=ConnectionSide.SERVER,
        ),
    ),
)
```

### Subcommand-level hints

For commands with subcommands that have different effect profiles, declare hints on each `SubCommand`. Subcommand hints take precedence over command-level hints:

```python
subcommands={
    "add": SubCommand(
        name="add",
        arity=Arity(),
        mutator=True,
        side_effect_hints=(
            SideEffect(
                target=SideEffectTarget.SESSION_TABLE,
                reads=False,
                writes=True,
                scope=StorageScope.SESSION_TABLE,
                connection_side=ConnectionSide.BOTH,
            ),
        ),
    ),
    "lookup": SubCommand(
        name="lookup",
        arity=Arity(),
        pure=True,
        side_effect_hints=(
            SideEffect(
                target=SideEffectTarget.SESSION_TABLE,
                reads=True,
                writes=False,
                scope=StorageScope.SESSION_TABLE,
                connection_side=ConnectionSide.BOTH,
            ),
        ),
    ),
}
```

Key interaction: when a subcommand is marked `pure=True` and has a hint, the classifier includes the hint effects as **read-only** (writes forced to `False`). This allows read-only subcommands like `table lookup` or `session count` to carry target/scope metadata without being classified as writers.

### Hint resolution order

`REGISTRY.side_effect_hints(command, subcommand, dialect)` resolves hints in this order:

1. Subcommand-level hints (if subcommand matches and has hints)
2. Command-level hints (fallback)

## Classification flow

`classify_side_effects(command, args)` follows this priority chain:

1. **Callee summary** — interprocedural summaries bypass registry lookup entirely.
2. **Dynamic barrier** — commands like `eval`, `uplevel`, `subst` produce `UNKNOWN` read+write with `dynamic_barrier=True`.
3. **Purity** — pure commands return `pure=True`. Protocol namespace pure commands include a read effect. Pure subcommands with hints include read-only effects.
4. **Variable assignment** — commands with `assigns_variable_at` produce a `VARIABLE` effect with scope/key/storage_type inferred from the variable name and command.
5. **Protocol namespace** — `HTTP::header`, `SSL::cert`, etc. are classified using hint target + subcommand mutator flags.
6. **Procedure definition** — `proc`, `rename` produce `PROC_DEFINITION` write.
7. **Hint fallback** — if a hint exists, use it directly.
8. **Conservative fallback** — `UNKNOWN` read+write.

## Examples of different side effect profiles

### Read-only data store access

```python
# class command: reads data groups, never writes
SideEffect(
    target=SideEffectTarget.DATA_GROUP,
    reads=True,
    writes=False,
    scope=StorageScope.DATA_GROUP,
    connection_side=ConnectionSide.GLOBAL,
)
```

### Write-only connection control

```python
# drop command: terminates the connection
SideEffect(
    target=SideEffectTarget.CONNECTION_CONTROL,
    writes=True,
    scope=StorageScope.CONNECTION,
    connection_side=ConnectionSide.BOTH,
)
```

### Response commit

```python
# HTTP::respond: commits the HTTP response
SideEffect(
    target=SideEffectTarget.RESPONSE_COMMIT,
    writes=True,
    connection_side=ConnectionSide.CLIENT,
)
```

### Read+write with arity-dependent behaviour

For HTTP namespace commands where getter vs setter depends on arguments, the command-level hint declares `reads=True, writes=True` (conservative), and the `mutator` flag on subcommands narrows the writes:

```python
# HTTP::header — command-level hint (conservative)
SideEffect(
    target=SideEffectTarget.HTTP_HEADER,
    reads=True,
    writes=True,
    connection_side=ConnectionSide.CLIENT,
)
# Subcommand "value" is pure → classifier narrows to read-only
# Subcommand "replace" has mutator=True → classifier keeps writes=True
```

### Logging / output

```python
# log command: writes to log output
SideEffect(
    target=SideEffectTarget.LOG_IO,
    writes=True,
    scope=StorageScope.LOG_OUTPUT,
    connection_side=ConnectionSide.NONE,
)
```

### Load balancing

```python
# pool command: selects a pool member
SideEffect(
    target=SideEffectTarget.POOL_SELECTION,
    writes=True,
    scope=StorageScope.CONNECTION,
    connection_side=ConnectionSide.SERVER,
)
```

## How to add hints to a new command

1. **Identify the target** — what external resource does the command touch? Pick from `SideEffectTarget`.
2. **Determine reads/writes** — does it read, write, or both? For commands with subcommands, does it vary?
3. **Choose scope** — where does the data live? Pick from `StorageScope`.
4. **Set connection side** — for F5 commands, which proxy side? `CLIENT`, `SERVER`, `BOTH`, or `GLOBAL`.
5. **Add to CommandSpec** — set `side_effect_hints=(SideEffect(...),)` on the spec.
6. **Add SubCommand entries if needed** — if subcommands have different read/write profiles, add per-subcommand hints with `SubCommand(side_effect_hints=(...))`.
7. **Mark pure subcommands** — set `pure=True` on read-only subcommands. The classifier will automatically narrow their hints to read-only.
8. **Mark mutator subcommands** — set `mutator=True` on write subcommands. The protocol namespace classifier uses this to upgrade writes.

### Checklist

- [ ] `side_effect_hints` tuple on `CommandSpec`
- [ ] Subcommand hints where read/write varies
- [ ] `pure=True` on read-only subcommands
- [ ] `mutator=True` on write subcommands
- [ ] Run `tests/test_side_effects.py` to verify classification

## File-path anchors

- `core/compiler/side_effects.py` — enums, dataclasses, `classify_side_effects()`, `EffectRegion` bridge
- `core/commands/registry/models.py` — `CommandSpec.side_effect_hints`, `SubCommand.side_effect_hints`
- `core/commands/registry/command_registry.py` — `REGISTRY.side_effect_hints()` lookup
- `core/compiler/gvn.py` — GVN consumer (6 call sites)
- `core/compiler/interprocedural.py` — interprocedural consumer
- `core/compiler/irules_flow.py` — response-commit and drop-command derivation
- `core/compiler/execution_intent.py` — purity consumer
- `core/compiler/core_analyses.py` — purity consumer

## Failure modes

- **Missing hint** — command falls through to conservative `UNKNOWN` read+write. GVN will not optimise around it. Fix: add `side_effect_hints` to the command's registry spec.
- **Missing subcommand hint** — read-only subcommand inherits the command's conservative read+write hint. Fix: add per-subcommand hints with `pure=True` on read-only subcommands.
- **Wrong `connection_side`** — iRules flow checker may fail to track response commits or connection drops on the correct side. Fix: set `connection_side` to match the F5 documentation.
- **Pure subcommand without hint** — classifier returns `pure=True` with no effects. Target/scope metadata is lost. Fix: add a hint to the subcommand so the classifier can include read-only effects.
- **Hint on dynamic barrier command** — hints are ignored; dynamic barriers always produce `UNKNOWN` read+write. This is correct — do not add hints to `eval`/`uplevel`.

## Test anchors

- `tests/test_side_effects.py`

## Discoverability

- [Compiler KCS index](README.md)
- [KCS index](../README.md)
- [Pipeline overview](kcs-compiler-pipeline-overview.md)
- [Execution intent model](kcs-execution-intent-model.md)
