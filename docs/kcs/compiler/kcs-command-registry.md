# KCS: Command registry infrastructure

## Symptom

A contributor needs to add a new command definition, understand how
`CommandSpec` metadata feeds the compiler, or debug why arity/taint/purity
information is not reaching a downstream pass.

## Context

Every Tcl command is defined as a `CommandDef` subclass whose `spec()`
classmethod returns a `CommandSpec`.  The `@register` decorator adds
definitions to a dialect's registry list, and the singleton
`CommandRegistry` merges specs into a unified lookup table.  Core specs
(Tcl, stdlib, tcllib) are always present; dialect-specific packs (Tk,
iRules, iApps, EDA, Expect) are loaded lazily on first access for that
dialect.  Registry metadata drives IR lowering, SCCP, GVN, taint,
side-effects, diagnostics, and code completion.

Source: [`core/commands/registry/models.py`](../../../core/commands/registry/models.py),
[`core/commands/registry/_base.py`](../../../core/commands/registry/_base.py),
[`core/commands/registry/signatures.py`](../../../core/commands/registry/signatures.py),
[`core/commands/registry/taint_hints.py`](../../../core/commands/registry/taint_hints.py)

## Content

### Architecture

```
CommandDef subclass (per command)
    │
    └─► spec() → CommandSpec
            │
            ├─► forms: tuple[FormSpec, ...]
            │     └─ kind, synopsis, arity, options, pure, mutator, side_effect_hints
            │
            ├─► subcommands: dict[str, SubCommand]
            │     └─ arity, pure, mutator, return_type, options, taint_transform,
            │        codegen, lowering, handler, validation_hook, ...
            │
            ├─► validation: ValidationSpec (overall arity)
            │
            ├─► arg_roles: dict[int, ArgRole]
            │
            ├─► taint: TaintHint (source, sinks, setter_constraints)
            │
            └─► dialects, event_requires, deprecated_replacement, ...
```

### CommandDef — defining a command

Each dialect (`tcl/`, `irules/`, `iapps/`, `tk/`) has its own `_REGISTRY`
list and `@register` decorator (created by `make_registry()` in `_base.py`).

```python
@register
class StringCommand(CommandDef):
    name = "string"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="string",
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="string option arg ...", ...),),
            subcommands={"length": SubCommand(name="length", arity=Arity(1,1), pure=True, ...), ...},
            validation=ValidationSpec(arity=Arity(1)),
            cse_candidate=True,
        )
```

### FormSpec — invocation forms

A command can have multiple forms (getter vs setter):

| Field | Purpose |
|-------|---------|
| `kind` | `DEFAULT`, `GETTER`, or `SETTER` |
| `arity` | Per-form arg count (None → inherit from command) |
| `pure` | No side effects |
| `mutator` | Modifies external state |
| `side_effect_hints` | Structured `SideEffect` tuples |
| `options` | Valid switch options (`OptionSpec`) |
| `arg_values` | Completable values per arg index |

`CommandSpec.resolve_form(args)` matches actual arguments against per-form
arities to select the right form.

### SubCommand — ensemble commands

Commands like `string`, `dict`, `HTTP::header` use subcommands.  Each
`SubCommand` has its own arity, purity, return type, taint transform,
codegen hook, lowering hook, and validation hook.

Subcommands can be dialect-filtered: `SubCommand.supports_dialect()` checks
the subcommand's own `dialects` set, falling back to the parent.

### Arity

```python
Arity(min=0, max=sys.maxsize)
```

The arity checker emits `W101` (wrong number of arguments) when an
invocation falls outside bounds.  Each `SubCommand` has its own arity.

### ArgRole — argument semantics

| Role | Meaning |
|------|---------|
| `BODY` | Tcl script body — recursively lowered into IR |
| `EXPR` | Expression — parsed into ExprNode AST |
| `VAR_NAME` | Variable written by the command (SSA def) |
| `VAR_READ` | Variable read without modification |
| `PARAM_LIST` | Procedure parameter list |
| `PATTERN` | Pattern or regex argument |
| `SUBCOMMAND` | The subcommand word |
| `OPTION_TERMINATOR` | The `--` terminator |
| `CHANNEL` | Channel identifier |
| `INDEX` | List/string index expression |

For variable-layout commands (`if`, `try`, `switch`), an `ArgRoleResolver`
callback dynamically maps argument values to roles.

### OptionSpec and option terminators

`OptionSpec(name, takes_value, detail)` declares `-flag` switches.
`OptionSpec(name="--")` on a `SubCommand` or `FormSpec` declares `--` support;
W304 ("use `--` before dynamic pattern") is derived automatically via
`CommandRegistry.resolve_option_terminator()`.

### ArgumentValueSpec — completions

`ArgumentValueSpec(value, detail, hover)` provides completion text and hover
documentation for specific argument positions.

### HoverSnippet — documentation

`HoverSnippet(summary, synopsis, snippet, source, examples, return_value)` —
appears on `CommandSpec.hover`, `SubCommand.hover`, `ArgumentValueSpec.hover`.

### ArgTypeHint — type expectations

`ArgTypeHint(expected, shimmers)` — declares what Tcl internal representation
a command expects.  Used by type inference and shimmer detection.

### KeywordCompletion — structural scaffolding

`KeywordCompletion(keyword, detail, snippet)` — for commands with
keyword-delimited structure (`if`, `try`, `switch`).

### Lazy dialect loading

`CommandRegistry.build_default()` loads only core specs (Tcl, stdlib,
tcllib).  Dialect-specific packs are loaded on demand:

1. **Trigger** — any public method that accepts `dialect` calls
   `_ensure_dialect_loaded(dialect)`, which delegates to
   `load_dialect_specs(dialect)`.
2. **Loader dispatch** — `_DIALECT_TO_LOADERS` maps each dialect to the
   loader keys it needs (e.g. `"synopsys-eda-tcl"` needs `"tk"`,
   `"sdc-base"`, and `"synopsys-eda-tcl"`).  Each key resolves via
   `_DIALECT_LOADER_SPECS` to a `(module_name, func_name)` pair loaded
   with `importlib.import_module`.
3. **Merge** — new specs are merged into `specs_by_name` and package
   indexes are updated.
4. **Invalidation** — trait indexes and all derived caches (command names,
   event commands, legality, filtered registries) are rebuilt.  The
   `_on_specs_loaded` callback notifies `runtime.py` to clear its
   `@lru_cache` functions, rebuild role/type hints, and merge taint hints.

**Contract**: any new public `CommandRegistry` method that takes a
`dialect` parameter must call `self._ensure_dialect_loaded(dialect)` before
accessing `specs_by_name`.

### How registry feeds the compiler

| Stage | Registry fields used |
|-------|---------------------|
| IR lowering | `arg_roles` (BODY, EXPR, VAR_NAME), lowering hooks |
| CFG | `creates_dynamic_barrier` → `IRBarrier` |
| SSA/SCCP | `pure` — infer through without invalidating lattice |
| GVN | `cse_candidate`, `pure` — result caching |
| Codegen | `codegen` hooks → specialised bytecode |
| Taint | `taint_hints()` → sources, sinks, transforms |
| Side effects | `side_effect_hints`, `pure`, `mutator` on forms/subcommands |
| Diagnostics | arity → W101, option terminators → W304, events → IRULE1001, deprecation → W300+ |
| Completions | `arg_values`, `keyword_completions`, `options` |

### Purity resolution order

1. `FormSpec.pure` / `FormSpec.mutator` (most specific)
2. `SubCommand.pure` / `SubCommand.mutator`
3. `CommandSpec.pure`

Higher levels override lower ones for the matched invocation form.

## Decision rule

- To add a new command: create a `CommandDef` subclass in the appropriate
  dialect package, implement `spec()`, and use `@register`.  For a new
  dialect pack, add an entry to `_DIALECT_LOADER_SPECS` and
  `_DIALECT_TO_LOADERS` in `command_registry.py`.
- To add taint tracking: implement `taint_hints()` on the `CommandDef`.
- To add special lowering: set `lowering` on the `CommandSpec` or `SubCommand`.
- If arity validation fails to fire, check that `ValidationSpec.arity` is
  set and subcommand arities are correct.
- Purity flows from command → subcommand → form; the most specific level wins.

## Related docs

- [Command infrastructure in walkthroughs](../../example-script-walkthroughs.md#command-infrastructure)
- [kcs-lowering-dispatch.md](kcs-lowering-dispatch.md)
- [kcs-taint-analysis.md](kcs-taint-analysis.md)
- [kcs-side-effects-system.md](kcs-side-effects-system.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
