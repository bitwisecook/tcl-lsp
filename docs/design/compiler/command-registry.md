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

Source: `compiler/registry/models.py`,
`compiler/registry/_base.py`,
`compiler/registry/signatures.py`,
`compiler/registry/taint_hints.py`

## Content

### Architecture

```
CommandDef subclass (per command)
    |
    +-> spec() -> CommandSpec
            |
            +-> forms: tuple[FormSpec, ...]
            |     +- kind, synopsis, arity, options, pure, mutator, side_effect_hints
            |
            +-> subcommands: dict[str, SubCommand]
            |     +- arity, pure, mutator, return_type, options, taint_transform,
            |        codegen, lowering, handler, validation_hook, ...
            |
            +-> validation: ValidationSpec (overall arity)
            |
            +-> arg_roles: dict[int, ArgRole]
            |
            +-> taint: TaintHint (source, sinks, setter_constraints)
            |
            +-> dialects, event_requires, deprecated_replacement, ...
```

### CommandDef -- defining a command

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

### CommandSpec field reference

#### Identity and availability

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `name` | `str` | *(required)* | Command name (e.g. `"lappend"`, `"dict"`) |
| `dialects` | `frozenset[str] \| None` | `None` | Which dialects have this command.  `None` = all dialects |
| `required_package` | `str \| None` | `None` | Only show in completions when this package has been `package require`d |
| `tcllib_package` | `str \| None` | `None` | Tcllib package that provides this command (per-document activation) |
| `warn_missing_import` | `bool` | `True` | Whether W120 fires when used without `package require`.  `False` for Tk commands (auto-loaded by `wish`) |

#### Documentation

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `hover` | `HoverSnippet \| None` | `None` | Man-page summary, synopsis, snippet, and examples for hover/signature help |
| `forms` | `tuple[FormSpec, ...]` | `()` | Invocation forms (getter vs setter variants).  See FormSpec section |
| `validation` | `ValidationSpec \| None` | `None` | Overall arity constraint.  Drives W101 (wrong number of arguments) |

#### Subcommands

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `subcommands` | `dict[str, SubCommand]` | `{}` | Ensemble subcommand registry.  See SubCommand section |
| `allow_unknown_subcommands` | `bool` | `False` | Suppress W102 for unrecognised subcommands (e.g. user-defined `oo::class` methods) |

#### Compiler traits

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `creates_dynamic_barrier` | `bool` | `False` | Lowered to `IRBarrier` -- blocks optimisations across this call |
| `has_loop_body` | `bool` | `False` | Command has a loop body (affects dead-code analysis) |
| `never_inline_body` | `bool` | `False` | Body arguments must not be inlined by the optimiser |
| `loop_list_header` | `bool` | `False` | CFG header carries list-expression args evaluated once before the loop body (foreach, lmap) |
| `is_control_flow` | `bool` | `False` | Command is a control-flow statement (break, continue, return) |
| `needs_start_cmd` | `bool` | `False` | Bytecode control flow: needs a `startCmd` instruction |
| `creates_scope_alias` | `bool` | `False` | Creates a scope alias (upvar-like binding) |
| `structurally_checked_arity` | `bool` | `False` | Registry `arity` is a descriptive floor only; a `clause_shape_check` hook owns real arity + shape validation, so the generic E002/E003 floor/ceiling check steps aside (`if`) |

#### Purity and optimisation

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `pure` | `bool` | `False` | No side effects -- safe for SCCP to propagate through |
| `cse_candidate` | `bool` | `False` | Result can be cached by GVN (common subexpression elimination) |

#### Argument semantics

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `arg_roles` | `dict[int, ArgRole]` | `{}` | Static arg roles: `BODY`, `EXPR`, `VAR_NAME`, `VAR_READ`, `PATTERN`, etc. |
| `arg_role_resolver` | `ArgRoleResolver \| None` | `None` | Dynamic arg-role resolution for variable-layout commands (if, try, switch) |
| `clause_shape_check` | `ClauseShapeChecker \| None` | `None` | Validates a clause-chain shape a plain `min..=max` arity can't express (if's `elseif`/`else` chain -- see `tcl_registry::clause_shape`); the compiler dispatches on the hook's presence, not the command name |
| `arg_types` | `dict[int, ArgTypeHint]` | `{}` | Per-argument type expectations (e.g. `INT`, `LIST`).  Drives shimmer detection |
| `return_type` | `TclType \| None` | `None` | Return type of the command |
| `keyword_completions` | `KeywordCompletionProvider \| None` | `None` | Keyword+scaffold completions for structural commands |

#### Variable assignment

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `assigns_variable_at` | `int \| None` | `None` | Arg index of the variable this command writes to (e.g. 0 for `set varName value`) |
| `var_write_typing` | `VarWriteTyping` | `ReturnValue` | How the type-inference pass types the variable(s) this command *writes*, distinct from `return_type` (which types the value it *returns*).  See below |
| `safe_on_uninit` | `frozenset[str] \| None` | `None` | Whether the command safely creates an uninitialised variable.  `None` = not safe (W210 fires).  Empty frozenset = safe in all dialects.  Non-empty frozenset = safe only in listed dialects.  Use `dialects_since("tcl8.5")` for version-gated behaviour (e.g. `incr` is safe in 8.5+ but errors in 8.4 and iRules) |
| `inferred_storage_type` | `StorageType \| None` | `None` | Inferred type for the target variable: `DICT`, `LIST`, or `ARRAY` |
| `defines_procedure` | `bool` | `False` | Command defines a procedure (proc, method, etc.) |

##### `var_write_typing` — return type vs written-variable type

A variable a command writes is not always typed by the command's
`return_type`.  `append` / `lappend` store exactly what they return, so the
return type describes both.  But a *destructuring* writer returns one thing
while writing another: `lassign` returns the leftover list yet writes list
*elements*; `scan` / `regexp` / `binary scan` return a match/convert *count*
yet write parsed pieces; `gets chan line` returns the character count yet
writes the *line*.  Broadcasting the return type onto those targets is the
S100 / W126 false-positive source of issue #867 (a `lassign` target wrongly
typed `List`, a `regexp` capture wrongly typed `Int`).

`VarWriteTyping` (in `tcl_registry::types`) captures the distinction so the
type-inference pass reads it per command / subcommand rather than keying on
the command name (it replaced a compiler-side `defs.len() > 1` heuristic that
mistyped every single-target destructure):

| Variant | Written variable receives | Commands |
|---------|---------------------------|----------|
| `ReturnValue` (default) | the command's `return_type` | `append`, `lappend`, `ledit`, `lset`, `dict set` |
| `Fixed(TclType)` | a fixed intrep, independent of the return value | `gets` → `String` (the line), `lpop` → `List` (the shortened list) |
| `Destructured` | element-/parse-dependent pieces, typed *overdefined* (unknown) | `lassign`, `scan`, `regexp`, `regsub`, `binary scan` |

The consumer is `tcl_compiler::type_infer::evaluate_type_def`, which resolves
the call (`ResolvedCall::var_write_typing`, so a subcommand like `binary scan`
overrides its parent) and maps the variant to the def's lattice type.  The
`return_type` path is unchanged: a captured result (`set left [lassign …]`)
still types `left` from `return_type`.

#### iRules event model

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `excluded_events` | `tuple[str, ...]` | `()` | Events where this command is explicitly forbidden |
| `event_requires` | `EventRequires \| None` | `None` | Transport, profile, and connection-side requirements.  Drives IRULE1001 |

#### Security and taint analysis

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `unsafe` | `bool` | `False` | Command is dangerous in iRules (IRULE2003) |
| `taint_sink` | `bool` | `False` | Command is a taint sink (T100) |
| `taint_output_sink` | `str \| None` | `None` | Output sink diagnostic code (e.g. `"IRULE3001"` for XSS) |
| `taint_output_sink_subcommands` | `frozenset[str] \| None` | `None` | Subcommands that are output sinks |
| `taint_log_sink` | `str \| None` | `None` | Log injection sink diagnostic code |
| `taint_network_sink_args` | `tuple[int, ...] \| None` | `None` | Arg indices that are network sinks |
| `taint_interp_eval_subcommands` | `frozenset[str] \| None` | `None` | Subcommands that eval untrusted input |
| `taint_transform` | `TaintColour \| None` | `None` | Colour bits added to tainted output |
| `taint_double_encode_colour` | `TaintColour \| None` | `None` | Colour for double-encoding detection |
| `taint_sink_safe_colour` | `TaintColour \| None` | `None` | Colour that suppresses T100 for this sink |
| `credential_options` | `frozenset[str] \| None` | `None` | Option flags that carry secrets (e.g. `-password`) |
| `sensitive_headers` | `frozenset[str] \| None` | `None` | Header names whose values are secrets |
| `password_option_command` | `bool` | `False` | Command has a password option |
| `warn_without_terminator` | `bool` | `False` | W304 fires even for non-dynamic positional values (e.g. regexp) |

#### Side effects

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `side_effect_hints` | `tuple[SideEffect, ...] \| None` | `None` | Static effect hints overriding heuristic classification.  Each `SideEffect` declares target (VARIABLE, CHANNEL, etc.), reads/writes, and connection side |

#### Deprecation and diagnostics

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `deprecated_replacement` | `type[CommandDef] \| str \| None` | `None` | Replacement command for deprecation warnings |
| `deprecation_fixer` | `DeprecationFixer \| None` | `None` | Code action for deprecated usage |
| `validation_hook` | `ValidationHook \| None` | `None` | Command-specific diagnostics beyond arity |

#### Execution and compilation hooks

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `handler` | `CommandHandler \| None` | `None` | VM execution hook |
| `codegen` | `CodegenHook \| None` | `None` | Bytecode specialisation hook |
| `lowering` | `LoweringHook \| None` | `None` | IR lowering hook |

#### Cross-cutting

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `diagram_action` | `bool` | `False` | Include in diagram extraction |
| `xc_translatable` | `bool \| None` | `None` | XC translatability.  `None` = follow default rules |
| `format_string_type` | `FormatType \| None` | `None` | Format string metadata (e.g. `format`, `scan`) |
| `pattern_type` | `PatternType \| None` | `None` | Pattern metadata (e.g. glob, regex) |
| `defines_symbol` | `SymbolDef \| None` | `None` | Command binds a navigable definition *name* the outline lists (`tcltest::test` → test case, `tcltest::testConstraint` → constraint, `tcltest::customMatch` → match mode).  `SymbolDef` carries the name argument index, an optional description-argument index, an optional `requires_arg` (record only when that argument is present — so a `testConstraint NAME value` setter defines but the `testConstraint NAME` getter does not), and the outline category (`DefinedSymbolKind`: `Test` / `Constraint` / `Matcher`).  Every symbol consumer (document + workspace symbols) reads it generically — no command-name check.  Distinct from `traits.DEFINES_PROCEDURE` / `definition_body`, which carry the richer proc / class records |

### SubCommand field reference

SubCommand shares many fields with CommandSpec but at the subcommand level.
Only fields unique to SubCommand or with different semantics are listed;
shared fields (`arg_roles`, `return_type`, `var_write_typing`, `arg_types`,
`pure`, `mutator`, `side_effect_hints`, `taint_transform`, `safe_on_uninit`,
etc.) have the same meaning as on CommandSpec.  A subcommand's
`var_write_typing` overrides its parent's when the call resolves to that
subcommand (`binary scan` destructures where the bare `binary` does not).

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `name` | `str` | *(required)* | Subcommand name (e.g. `"set"`, `"length"`) |
| `arity` | `Arity` | *(required)* | Arg count after the subcommand word |
| `detail` | `str` | `""` | Short description for completion items |
| `synopsis` | `str` | `""` | Usage synopsis for completion/hover |
| `dialects` | `frozenset[str] \| None` | `None` | Override parent's dialect set.  `None` = inherit |
| `destructive` | `bool` | `False` | Destructive operation (e.g. `file delete`) |
| `credential_arg` | `int \| None` | `None` | Arg index that carries a secret |
| `taint_output_sink` | `str \| None` | `None` | Per-subcommand output sink diagnostic code |
| `xc_operation` | `str \| None` | `None` | XC translation operation |
| `forms` | `tuple[FormSpec, ...]` | `()` | Per-subcommand getter/setter forms |

### ObjectClassSpec -- object-method dispatch

A `TclOO` / megawidget class whose instances are dispatched as
`$obj <method> …` is modelled by an `ObjectClassSpec` attached to the class
*command* spec (the factory) via `CommandSpec.object_class`.  For a `TclOO`
class the class name **is** the factory command name (`oo::class create Foo`
binds command `Foo`), so a class spec resolves through the ordinary command
table — no separate index (`CommandRegistry::object_class`).

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `class_name` | `&str` | *(required)* | Fully-qualified class name = factory command name |
| `instance_methods` | `&[SubCommand]` | `&[]` | Methods dispatched on a handle (`Xaxis`, `Add`, …), reusing `SubCommand` (so option / enum / arg-value metadata is shared) |
| `superclasses` | `&[&str]` | `&[]` | Direct superclass names for inherited-method resolution |
| `allow_unknown_methods` | `bool` | `false` | Accept an unrecognised method without complaint |

The class's `new` / `create` constructor returns an object handle of
`class_name`.  Two consumers act on this:

- **Object-handle tracking** (`tcl_compiler::object_types`) harvests
  `set VAR [Class new|create …]` provenance so a variable is known to hold an
  instance of `class_name`; it follows scalar and array-element handles across
  the top level, procedures, and method bodies.  This is *provenance*, not the
  object→class dispatch *lattice* prototyped in
  [`../tcloo-mro-lattice.md`](../tcloo-mro-lattice.md), which measured as a
  negative on real `TclOO` corpora (factory-return receivers dominate the ⊤
  bucket); an un-provenanced (proc-parameter) receiver is deliberately left to
  the generic shape-based option highlighting rather than resolved unsoundly.
- **Semantic tokens** resolve a `$var method …` / `[Class new] method …`
  dispatch against the class's `instance_methods` and colour the method plus
  its declared options exactly like a built-in's — the object-handle half of
  issue #748.  A method whose options are not modelled still resolves as a
  method call; its `-option value` pairs fall through to the generic option
  highlighting.

### FormSpec -- invocation forms

A command can have multiple forms (getter vs setter):

| Field | Purpose |
|-------|---------|
| `kind` | `DEFAULT`, `GETTER`, or `SETTER` |
| `arity` | Per-form arg count (None -> inherit from command) |
| `pure` | No side effects |
| `mutator` | Modifies external state |
| `side_effect_hints` | Structured `SideEffect` tuples |
| `options` | Valid switch options (`OptionSpec`) |
| `arg_values` | Completable values per arg index |

`CommandSpec.resolve_form(args)` matches actual arguments against per-form
arities to select the right form.

### Arity

```python
Arity(min=0, max=sys.maxsize)
```

The arity checker emits `W101` (wrong number of arguments) when an
invocation falls outside bounds.  Each `SubCommand` has its own arity.

### ArgRole -- argument semantics

| Role | Meaning |
|------|---------|
| `BODY` | Tcl script body -- recursively lowered into IR |
| `EXPR` | Expression -- parsed into ExprNode AST |
| `VAR_NAME` | Variable written by the command (SSA def) |
| `VAR_READ` | Variable read without modification |
| `PARAM_LIST` | Procedure parameter list |
| `PATTERN` | Pattern or regex argument |
| `SUBCOMMAND` | The subcommand word |
| `OPTION_TERMINATOR` | The `--` terminator |
| `CHANNEL` | Channel identifier |
| `INDEX` | List/string index expression |

### Resolution priority

Three mechanisms assign roles to command arguments. They are evaluated in
priority order — the first that provides a mapping wins:

1. **`arg_role_resolver`** (dynamic callback) — inspects the actual argument
   list at analysis time and returns a `dict[int, ArgRole]`. Used for
   variable-arity commands where roles depend on argument count or content.
   Examples: `set` returns `VAR_WRITE` for arg 0 when two arguments are
   present but `VAR_READ` when only one is present; `if` maps body and
   expression positions by scanning for `elseif`/`else` keywords.
2. **`arg_roles`** (static dict) — a fixed mapping on the spec. Sufficient
   when every invocation has the same layout.
3. **`assigns_variable_at`** (legacy shorthand) — marks a single argument
   index as a variable write. Overridden when a dynamic resolver exists.

When a spec carries both `assigns_variable_at` and `arg_role_resolver`, the
resolver is authoritative. The static field remains as a fallback for
consumers that do not invoke the resolver (e.g. simple liveness queries).

### Compound commands and subcommand dispatch

Tcl compound commands (`namespace upvar`, `dict for`, `string map`, etc.)
are tokenised as a base command with a subcommand argument. Different
analysis layers handle these at different levels:

- The **registry** uses `SubCommand` entries on the parent `CommandSpec`.
  The parent's `arg_role_resolver` inspects the subcommand word to assign
  roles to the remaining arguments.
- **Variable scoping** (`compiler/var_scoping.py`) has explicit
  handlers for compound forms like `namespace upvar`, `dict set`,
  `dict update`, and `dict with`.
- **Lowering hooks** (`compiler/lowering_hooks/`) have per-command
  hooks that understand subcommand structure.

When verifying whether a compound command is handled, search all three
layers — the feature module closest to the symptom (e.g.
`server/features/declaration.py`) may intentionally delegate to a deeper
module.

### OptionSpec and option terminators

`OptionSpec(name, takes_value, detail)` declares `-flag` switches.
`OptionSpec(name="--")` on a `SubCommand` or `FormSpec` declares `--` support;
W304 ("use `--` before dynamic pattern") is derived automatically via
`CommandRegistry.resolve_option_terminator()`.

### ArgumentValueSpec -- completions

`ArgumentValueSpec(value, detail, hover)` provides completion text and hover
documentation for specific argument positions.

### HoverSnippet -- documentation

`HoverSnippet(summary, synopsis, snippet, source, examples, return_value)` --
appears on `CommandSpec.hover`, `SubCommand.hover`, `ArgumentValueSpec.hover`.

### ArgTypeHint -- type expectations

`ArgTypeHint(expected, shimmers)` -- declares what Tcl internal representation
a command expects.  Used by type inference and shimmer detection.

### KeywordCompletion -- structural scaffolding

`KeywordCompletion(keyword, detail, snippet)` -- for commands with
keyword-delimited structure (`if`, `try`, `switch`).

### safe_on_uninit -- variable initialisation safety

Some commands safely create an uninitialised variable rather than erroring:
`lappend` creates an empty list, `append` an empty string, `incr` treats
the variable as `0` (8.5+ only), and `dict set`/`dict append`/`dict lappend`/
`dict incr` create an empty dict.

This trait is set on `CommandSpec` (for top-level commands) or `SubCommand`
(for ensemble subcommands like `dict set`).  The value is a frozenset of
dialect strings:

| Value | Meaning |
|-------|---------|
| `None` | Not safe -- W210 fires if variable is read before set |
| `frozenset()` | Safe in **all** dialects |
| `frozenset({"tcl8.5", "tcl8.6", ...})` | Safe only in listed dialects |

For version-dependent behaviour, use `dialects_since()` from
`compiler/registry/dialects.py`:

```python
from ..dialects import dialects_since

# incr: safe in Tcl 8.5+ but errors in 8.4 and iRules (Tcl 8.4.6)
safe_on_uninit=dialects_since("tcl8.5")
```

`dialects_since()` resolves against `DIALECT_BASE_VERSION`, a centralised
map of each dialect's runtime Tcl version.  This ensures that derived
dialects (iRules -> 8.4, iApps -> 8.5, EDA -> varies) inherit the correct
behaviour without hardcoding dialect names in command specs.

**Data flow:**

```
Registry spec (safe_on_uninit)
    |
    +-> Lowering reads REGISTRY.is_safe_on_uninit(cmd, sub, dialect)
    |     +-> Stamps IRCall.safe_on_uninit / IRIncr.safe_on_uninit
    |
    +-> Analyser checks getattr(stmt, "safe_on_uninit", False)
          +-> Suppresses W210 when True
```

No command names or dialect names appear in the compiler or analyser --
all knowledge lives in the registry specs and `dialects.py`.

### Lazy dialect loading

`CommandRegistry.build_default()` loads only core specs (Tcl, stdlib,
tcllib).  Dialect-specific packs are loaded on demand:

1. **Trigger** -- any public method that accepts `dialect` calls
   `_ensure_dialect_loaded(dialect)`, which delegates to
   `load_dialect_specs(dialect)`.
2. **Loader dispatch** -- `_DIALECT_TO_LOADERS` maps each dialect to the
   loader keys it needs (e.g. `"synopsys-eda-tcl"` needs `"tk"`,
   `"sdc-base"`, and `"synopsys-eda-tcl"`).  Each key resolves via
   `_DIALECT_LOADER_SPECS` to a `(module_name, func_name)` pair loaded
   with `importlib.import_module`.
3. **Merge** -- new specs are merged into `specs_by_name` and package
   indexes are updated.
4. **Invalidation** -- trait indexes and all derived caches (command names,
   event commands, legality, filtered registries) are rebuilt.  The
   `_on_specs_loaded` callback notifies `runtime.py` to clear its
   `@lru_cache` functions, rebuild role/type hints, and merge taint hints.

**Contract**: any new public `CommandRegistry` method that takes a
`dialect` parameter must call `self._ensure_dialect_loaded(dialect)` before
accessing `specs_by_name`.

### How registry feeds the compiler

| Stage | Registry fields used |
|-------|---------------------|
| IR lowering | `arg_roles` (BODY, EXPR, VAR_NAME), `safe_on_uninit`, lowering hooks |
| CFG | `creates_dynamic_barrier` -> `IRBarrier` |
| SSA/SCCP | `pure` -- infer through without invalidating lattice |
| GVN | `cse_candidate`, `pure` -- result caching |
| Codegen | `codegen` hooks -> specialised bytecode |
| Taint | `taint_hints()` -> sources, sinks, transforms |
| Side effects | `side_effect_hints`, `pure`, `mutator` on forms/subcommands |
| Diagnostics | arity -> W101, `safe_on_uninit` -> W210, option terminators -> W304, events -> IRULE1001, deprecation -> W300+ |
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
- Purity flows from command -> subcommand -> form; the most specific level wins.
- To mark a command as safe on uninitialised variables: set `safe_on_uninit`
  on the `CommandSpec` or `SubCommand`.  Use `frozenset()` for all dialects,
  or `dialects_since("tcl8.X")` for version-gated behaviour.  Never
  hardcode dialect names in the compiler or analyser.
- When adding a new dialect, add it to `DIALECT_BASE_VERSION` in
  `dialects.py` so version-dependent traits resolve correctly.

## Related docs

- [Command infrastructure in walkthroughs](../../../docs/design/example-script-walkthroughs.md#command-infrastructure)
- [kcs-lowering-dispatch.md](../../../docs/design/compiler/lowering-dispatch.md)
- [kcs-taint-analysis.md](../../../docs/design/compiler/taint-analysis.md)
- [kcs-side-effects-system.md](../../../docs/design/compiler/side-effects-system.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [kcs-dialects-events.md](../../../docs/design/compiler/dialects-events.md)
