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
| `lifecycle` | `Lifecycle` | `UNSPECIFIED` | Introducing / deprecating / **retiring** releases on the owning package's version axis.  See [Lifecycle](#lifecycle----one-contract-for-every-versioned-entity) |

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
| `default_form_first_word` | `Option<DefaultFormFirstWord>` | `None` | Value shape a non-subcommand first word may take to select the command's *default* form (`after 200 ...` — an integer first word is a delay, not an unknown subcommand). Matched via the canonical `tcl-syntax` number parser, so every Tcl integer spelling works |

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
| `arg_presentation` | `&[(u8, ArgPresentation)]` | `&[]` | Formatter layout override per argument index -- see [ArgPresentation](#argpresentation----how-a-formatter-lays-an-argument-out) |
| `repeated_args` | `&[RepeatedArgLayout]` | `&[]` | Roles that recur at a fixed stride over the argument tail (`global a b c`, `foreach v l ... body`) |
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
| `defines_command_at` | `Option<u8>` | `None` | Argument index (0-based, after the command name) whose *literal* value becomes a callable command name once the call runs — `coroutine NAME cmd ?arg …?` binds `NAME` (`TclNRCoroutineObjCmd`, `tclBasic.c`).  Lighter than `creates_instance_at` (no `object_class` method dispatch); consumed generically by the analyser so later calls to the name don't draw W123.  The subcommand-level twin lives on `SubCommand` (`interp create ?-safe? ?--? ?name?`, index relative to the word after the subcommand); an option flag (leading `-`) or dynamic word at the index is never recorded, and a missing name is auto-generated at run time |

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
| `ReturnValue` (default) | the command's `return_type` — but only for a **single** written target; a call that writes several variables under this default (`catch`/`try`'s synthetic result/options + body writes) stays *overdefined*, since one return value cannot be the value of several distinct variables | `append`, `lappend`, `ledit`, `lset`, `dict set` |
| `Fixed(TclType)` | a fixed intrep, independent of the return value | `gets` → `String` (the line), `regsub` → `String` (the substituted result), `lpop` → `List` (the shortened list) |
| `Destructured` | element-/parse-dependent pieces, typed *overdefined* (unknown) | `lassign`, `scan`, `regexp`, `binary scan` |

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
| `inline_codegen_hook` | `Option<InlineCodegenHookId>` | `None` | Inline (value-position `[cmd …]` / catch-body) bytecode specialisation hook — the Rust registry's typed ID dispatched by `tcl_compiler::codegen::{cmd_subst,control_flow}` |
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
| `lifecycle` | `Lifecycle` | `UNSPECIFIED` | Introducing / deprecating / **retiring** releases of this subcommand on the owning package's version axis. Retirement is exclusive (`retired: 10.0.0` ⇒ gone *in* 10.0.0). On iRules commands this is compared with the existing `tclLsp.bigipVersion` / `--bigip-version` keyed BIG-IP floor. See `tcl_registry::lifecycle` |
| `versioned_arg_values` | `&[VersionedArgValue]` | `&[]` | Owning-package release ranges for individual literal values declared in `arg_values`, indexed after the subcommand word (for example, the `mcp` mode of `persist add`) |
| `destructive` | `bool` | `False` | Destructive operation (e.g. `file delete`) |
| `credential_arg` | `int \| None` | `None` | Arg index that carries a secret |
| `taint_output_sink` | `str \| None` | `None` | Per-subcommand output sink diagnostic code |
| `xc_operation` | `str \| None` | `None` | XC translation operation |
| `forms` | `tuple[FormSpec, ...]` | `()` | Per-subcommand getter/setter forms |
| `defines_command_at` | `Option<u8>` | `None` | Subcommand-level twin of the command-level `defines_command_at` (index 0-based, *after* the subcommand word) — `interp create ?-safe? ?--? ?name?` binds `name` as the child interpreter's command |

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
| `COMMAND_PREFIX` | A callback command reference (`lsort -command cb`) whose first word is invoked at runtime with further arguments appended; recognises a literal bareword, a braced `{cmd extra}` multi-word prefix, and a `[list cmd extra]`-quoted prefix (gated on the `BUILDS_COMMAND_PREFIX` trait, below) -- distinct from `BODY` since the word is a reference, not code |
| `LAMBDA_LITERAL` | A `{argList body ?namespace?}` anonymous-lambda literal (`apply`'s argument shape) -- a *list*, not a script directly; element 0 is a parameter list, element 1 is the body to recurse into |

Two predicates on `ArgRole` itself answer the cross-cutting questions
consumers must not each re-derive. Both are exhaustive `match`es, so a new
role fails to compile until someone decides which side it falls on:

| Predicate | True for | Asked by |
|---|---|---|
| `carries_script()` | `BODY`, `EXPR`, `LAMBDA_LITERAL` | every walker that recurses into executable Tcl — semantic tokens, the iRules object-reference walker, the inert-text data-brace proof |
| `names_variable()` | `VAR_NAME`, `VAR_READ` | the analyser's reference recorder (a `VAR_READ` word is a use site of the named cell — `puts [set m]` and `info exists m` read `m` exactly as `$m` does), the dead-store suppressor's substitution scan (a **braced** word in such a role is a *literal* name, so a `$x` inside it belongs to that name and is not a read of `x`), and `tcl_compiler::var_refs::variable_name_role_words`, the one place that turns a segmented command into its variable-name words |

`LOOP_VAR_LIST` is deliberately outside `names_variable()`: that word is a
*list* of names, not one name, so a consumer must split it before it holds a
variable name at all.

### Repeated argument layouts -- unbounded regular tails

`arg_roles` is a fixed index table and an `arg_role_resolver` is an opaque
closure; neither is a good fit for the *regular, unbounded* argument tails a
whole family of Tcl commands takes:

```tcl
global a b c                       ;# a name at every word
variable n1 v1 n2 v2               ;# a name at every other word
foreach v1 $l1 v2 $l2 { ... }      ;# a spec at every other word, body excluded
namespace upvar ::ns o1 l1 o2 l2   ;# the local of each pair, after a fixed prefix
dict update d k1 v1 k2 v2 { ... }  ;# the same, with the body excluded
```

Every consumer that needed one of these re-derived the stride from the
command's *name* -- three separate copies in the semantic-token walk alone.
`repeated_args: &[RepeatedArgLayout]` (on both `CommandSpec` and `SubCommand`)
declares the layout as data instead:

| Field | Meaning |
|---|---|
| `role` | the `ArgRole` assigned at each covered position |
| `start` | first covered index (after the head, or after the subcommand word) |
| `stride` | distance between covered positions (`1` = every word, `2` = every other) |
| `exclude_trailing` | words at the *end* the layout does not cover (a trailing body) |
| `optional_leading_word` | the command takes one optional leading word whose presence shows only in the argument count (`upvar`'s `?level?`); the group is then anchored to the end of the covered range |

The layouts feed `CommandRegistry::arg_indices_for_role` alongside
`arg_role_resolver` and `arg_roles`, **additively** -- so a spec can pin its
leading words with `arg_roles` (`namespace upvar`'s leading namespace word)
and still declare the repeating pair tail. A consumer therefore just asks
"which arguments carry role X" and gets the whole answer.

**Limits.** The layout is purely positional: it cannot express a stride that
depends on a *word's value* (a switch that shifts the tail), which still needs
an `arg_role_resolver`. `upvar` deliberately does **not** use one -- its
`?level?` word is already modelled more precisely by `FrameEffectSpec`
(`FrameArgLayout::AliasPairs` + `FrameLevelWord::ArityParity`, which is how C
Tcl itself decides: `Tcl_UpvarObjCmd` tests `objc`, never the word's text).

### Format strings -- family plus location

Which words of a call carry a conversion / field string, and in which
mini-language, is two registry facts read together through
`CommandRegistry::format_string_args`:

- **Where** -- the `ArgRole::FormatString` / `ArgRole::ScanFormat` positions of
  the call. That covers a fixed index (`format`), a resolver-computed one
  (`scan`, `regsub` past its switches), a subcommand-relative one
  (`binary format`), and an **option value** (`clock format ... -format FMT`),
  because `arg_indices_for_role` already resolves all four.
- **Which language** -- `format_string_type` (`FormatType::Sprintf` / `Clock` /
  `Binary` / `Regsub`), overridden by the subcommand's own when one dispatches.

The `scan` flag on the returned `FormatStringArg` says which *direction* the
word is written in: `format` and `scan` share the `Sprintf` family but not its
conversion set, so a consumer must not infer one from the other.

The family check is load-bearing for correctness, not decoration. `clock`'s
field string, `binary`'s cursor spec, and `regsub`'s backreference template all
sit at `FormatString`/`ScanFormat` positions, and none is a printf %-string --
running the sprintf version gate over `clock format $t -format {%b}` would
report a bogus Tcl 8.6 requirement. `record_dsl_format_sites` (W138) therefore
gates on `FormatType::Sprintf` and leaves the other families alone rather than
guessing.

### HandleBindingSpec -- which argument becomes an object handle

Some calls make a *variable* hold an object handle, with a second word of
the same call saying what class the handle is. Two shapes recur:

```tcl
install axis using ::verticalAxis $win.a   ;# snit component install
set     axis      [::verticalAxis $win.a]  ;# snit bare-word construction
```

Both used to be recognised by matching the command word in the LSP's
handle scan. They are now registry data
(`rust/tcl-registry/src/handle_binding.rs`):

```rust
pub struct HandleBindingSpec {
    pub name_at: u8,                     // the variable that receives the handle
    pub class_from: HandleClassSource,   // where the class is written
    pub keyword: Option<HandleKeyword>,  // a literal word the layout requires
}

pub enum HandleClassSource {
    Word(u8),               // the word *is* the class name (`install … using TYPE`)
    ConstructionValue(u8),  // the word *contains* a construction (`set n [TYPE …]`)
}
```

`HandleBindingSpec::resolve(args)` returns a `BoundHandle { name,
class_word, class_source }` or `None`. It abstains rather than guesses: a
missing keyword (`install a b c`, a user's own `install`), a call too short
to carry both words, or a dynamic word all answer `None`.

Where the descriptor hangs depends on whether the command is global:

| Command | Home | Why |
|---|---|---|
| `set` | `CommandSpec::binds_handle` | a real global command; `CommandRegistry::handle_binding` resolves it through `get`, so `::set` answers identically |
| snit `install` | `DefinitionBodyGrammar::member_body_commands` | the word exists **only** inside a snit member body -- a global spec would make a user's own `proc install` look like a built-in everywhere. `CommandRegistry::member_body_handle_bindings()` enumerates them once per document |

The paired grammar flag `DefinitionBodyGrammar::bare_word_construction`
says whether a family's *type command* constructs from a bare instance
name (`$type $name`, snit(n)'s "The Type Command"). It is `true` for snit
and `false` for `TclOO` / `[incr Tcl]`, and it replaced a
`metaclass.starts_with("snit::")` spelling test in the scan.

**Limits.** The descriptor covers a *fixed* pair of indices plus one
optional literal keyword -- enough for both shapes above and for a
comparable installer in another class system, and deliberately not a
general option parser. snit's `installhull ?using TYPE …?` is **not**
modelled: it binds the implicit `hull` component rather than a named
variable, so it has no `name_at`, and adding it needs another variant
rather than another row. `install NAME $widget` (a run-time-typed
component) is likewise not modelled -- there is no static class word, so
the scan abstains.

### ArgPresentation -- how a formatter lays an argument out

`ArgRole` says what an argument **is**. `arg_presentation` (a
`&'static [(u8, ArgPresentation)]` on both `CommandSpec` and `SubCommand`)
says how a *formatter* should lay it out. The two are deliberately separate
facts, because two arguments can share a semantic role and still want
different presentation.

`for start test next body` is the case that forced the split. All three of
`start`, `next`, and `body` are Tcl scripts and carry `ArgRole::Body` --
every analysis consumer must keep walking them. But conventional Tcl keeps
`start` and `next` on the `for` header line and expands only `body`:

```tcl
for {set i 0} {$i < 3} {incr i} {
    puts $i
}
```

Erasing the semantic distinction would break the walkers; keeping a
`name == "for"` branch in the formatter is exactly the command-specific
knowledge the registry contract forbids. So `for` declares the layout
preference as data instead:

```rust
arg_presentation: &[
    (0, ArgPresentation::InlineScript),
    (2, ArgPresentation::InlineScript),
],
```

| Variant | Meaning |
|---|---|
| `BlockScript` | expanded onto its own indented lines, opening brace left on the command line (K&R). The **default** for every `Body` argument |
| `InlineScript` | kept on the command's own line, however long -- `for`'s `start` / `next` |

Only overrides are declared, so a spec with nothing unusual to say leaves
the field empty. Consumers ask
`CommandRegistry::arg_presentation(name, args, index)`, which resolves
through the same `get` path as every other query -- so `::for` answers
identically to `for` -- and returns `BlockScript` for a command the registry
does not know.

**Limits.** `ArgPresentation` describes layout preference only; it carries no
line-width, alignment, or comment policy (those are `FormatterConfig`), and
it cannot make a *non*-body argument expand. The enum is `#[non_exhaustive]`:
it is the extension point for further presentation facts, and a consumer must
keep working when a new variant arrives.

**Structural keywords are not a presentation fact.** `if`'s `then` /
`elseif` / `else` and `try`'s `on` / `trap` / `finally` already carry
`ArgRole::Keyword` at the positions the C-Tcl-shaped clause walk puts them,
so the formatter reads that role directly rather than scanning argument
*values* for those words. The difference is observable: in
`if {1} {a} else then` the trailing `then` sits in the else-branch **body**
slot -- tclsh 8.6 and 9.0.4 run `a` and treat `then` as that branch's script
-- so it is a body word, not a keyword, and only a positional answer gets
that right.

`Traits.BUILDS_COMMAND_PREFIX` (set on `list` only, not `concat`) marks a
command whose result, when its own first argument is a literal command name,
is a valid command reference the remaining arguments append to -- the
`[list cmd extra]` idiom for building a callback or deferred command around a
dynamic value (`-command [list doSomething $x]`,
`package ifneeded name ver [list apply {argList body} $dir]`). Consulted
generically wherever a `COMMAND_PREFIX`/`BODY`/`LAMBDA_LITERAL` argument
position needs to see through the quoting -- never by comparing a command
head to the literal string `"list"`.

### Unit-linkage traits

Three traits record that a command makes the file it appears in part of a
**bigger program** — the fact `tcl_compiler::unit_scope` needs to decide
whether one file's call sites can be trusted as every call site (issue #977):

| Trait | Set on | Means |
|---|---|---|
| `PROVIDES_PACKAGE` | `package provide`, `package ifneeded` | the file is a loadable package; its commands are public API |
| `EXPORTS_COMMAND` | `namespace export`, `namespace ensemble` | the file publishes command names for another unit to import or dispatch through |
| `LOADS_EXTERNAL_UNIT` | `source`, `load`, `package require`, `auto_load`, `auto_import` | another unit's script runs in this interpreter and can call back in |

`CommandRegistry::unit_linkage(name, args, dialect)` resolves an invocation
through `resolve_call` and returns the union of `spec.traits | sub.traits`
masked to `UNIT_LINKAGE_TRAITS`, so the answer is subcommand-precise:
`package provide` is a boundary, `package names` is not. Adding a new
boundary command is a spec edit — the compiler never names one.

The first two traits publish the file's commands to consumers no project
enumeration can bound, so they decline the interprocedural seed
unconditionally; `LOADS_EXTERNAL_UNIT` names a caller a workspace normally
contains, so it defers to host-supplied cross-file evidence. See
[compilation-unit-scope.md](compilation-unit-scope.md).

`namespace import` is deliberately excluded: it is as often an intra-file
convenience over a namespace the same file defines, and `unit_scope`'s
evidence scan already models the import as a real caller path.

### The `Tcl_ConcatObj` eval family

Five commands evaluate the **concatenation** of every trailing word from
their first `BODY`-role argument onwards, rather than that one word alone:

| Trait | Set on | Means |
|---|---|---|
| `SCRIPT_CONCATENATES_ARGS` | `eval`, `uplevel`, `namespace eval`, `namespace inscope`, `interp eval` | every word from the first `BODY` index to the end of the call is part of one script |
| `SCRIPT_APPENDS_LIST_ARGS` | `namespace inscope` | refines the above: the tail is appended as *list elements*, not space-joined |

`Tcl_ConcatObj` (`generic/tclUtil.c`) takes each word's **string
representation** — so a braced word contributes its *contents*, the outer
braces already removed by Tcl's word parsing — trims ASCII whitespace from
each end, drops words that trim to nothing, and joins the rest with a single
space. Commands therefore span word boundaries:

```text
concat {set x} {5}      -> set x 5
concat "  a  " "  b  "  -> a b
eval set l2 hello       == set l2 hello
eval {set l2} hello     == set l2 hello
```

The consumer contract is that no pass may treat the first `BODY`-role word as
the whole script when the trait is present and further words follow. Either
walk the tail as one script — sound only when every word is statically-known
script text, since substitution runs *before* concatenation; a braced word
qualifies even when its contents carry `$`/`[`, because the braces blocked
the outer substitution and the eval-family command itself resolves them when
the script runs — or consume the command without walking it. Analysing
`eval set l2 hello` as the one-word script `set` invents a wrong-#-args
error and loses the write to `l2` (issue #1051).

A consumer that records *spans* while walking (the analyser's
`dispatch_concatenated_script`) must not walk a freshly-joined string
against a linear anchor: stripped delimiters, dropped words, and collapsed
whitespace shift every offset after the first divergence, so recorded
references become rename hazards. `analyser::utils::concat_script_window`
is the shared answer — it rebuilds the equivalent script *in place* over
the words' source window (content bytes at their true offsets, structural
bytes blanked to spaces, whitespace runs being separator-equivalent), so
every recorded span maps to the exact source bytes it describes. Consumers
that harvest names only (the CFG barrier-write harvest) may still use a
plain text join.

`namespace inscope` is the one member that does not space-join:
`namespace inscope ns script ?arg ...?` is `namespace eval ns [concat script
[list arg ...]]`, so `namespace inscope :: {puts} {a b}` prints `a b` where
`namespace eval :: {puts} {a b}` errors `can not find channel named "a"`.
Reconstructing that needs list quoting the analyser does not model, so any
trailing word means the call is consumed without walking.

The **VM** does model it, because it has to run the command:
`tcl-vm/src/cmd_namespace.rs`'s `inscope_script` builds the tail as a
`Value::list` (whose string rep is the canonical `tcl_syntax::list::join_list`
quoting) and concatenates it onto the script through the shared
`tcl_cmd_core::list::concat`, so neither the quoting nor the `Tcl_ConcatObj`
trim rule is re-derived there. With no trailing word it evaluates the script
verbatim, matching C's `objc == 3` arm (issue #1056).

`catch` is deliberately outside the family: it takes a single bounded script
argument, so its remaining words are result / options variable names.

The `EXPR_CONCATENATES_ARGS` trait is the expression-side counterpart, for
`expr`'s whole-tail expression.

### `TclOO` method-context keywords

Three traits classify the words that appear at the head of a call inside a
`TclOO` method body. They sit on **different axes** and must not be
conflated:

| Trait | Set on | Means |
|---|---|---|
| `TCLOO_SELF_DISPATCH` | `my` | instance self-dispatch — the next word names a method on *this* object, reaching non-exported (and, from 9.0, private) methods |
| `TCLOO_NEXT_CHAIN` | `next`, `nextto` | superclass chain — no word names a method; the callee is the next implementation of the *currently executing* one |
| `TCLOO_INTROSPECTION` | `self` | introspection, never dispatch — its argument is a closed subcommand set |

`link` carries none of them, deliberately: it *creates* per-object bareword
commands (`link {alias method}`), so the barewords it installs are per-class
data rather than language keywords (issue #1026).

`CommandRegistry::method_dispatch_keyword(head) -> Option<MethodDispatchKind>`
is the single query every consumer uses instead of a `head == "my"` /
`matches!(head, "my" | "next" | "nextto")` literal (issue #1050). It:

- normalises a leading `::`, matching `get`;
- answers under the **registry instance's own** dialect profile, so a
  registry built by `registry_for_dialect("tcl8.5")` returns `None` for all
  four (every one is `TCL86_PLUS`), while a profile-less
  `CommandRegistry::build_default()` answers dialect-agnostically;
- returns `SelfDispatch` / `NextChain` / `Introspection`, so a consumer that
  wants "does the next word name a method" can ask for `SelfDispatch` alone.

`nextto`'s explicit resume-from class is distinguished from `next`
*structurally* — an `ArgRole::Name` at argument index 0 on the spec — not by
name, so a consumer capturing that target queries the role rather than
matching the spelling.

A future dialect variant of any of these keywords propagates through its
`CommandSpec`, never through a walker edit; the contract tests in
`registry_commands.rs` assert the consumer-visible keyword set equals the
trait-carrying specs, per dialect.

### `TCLOO_METHOD_CONTEXT` — where a bare spelling resolves

The three traits above say what a word *does* once it has resolved.
`TCLOO_METHOD_CONTEXT` says *where* it resolves at all, and is orthogonal
to them (issue #1026).

A `TclOO` method body runs with the **object's** namespace current and
`::oo::Helpers` on that namespace's `namespace path`, so the family's bare
spellings are reachable there and nowhere else. Pinned against tclsh 9.0.4:

```text
% link foo                              -> invalid command name "link"
% info commands ::link                  -> {}
% oo::class create C { method m {} {
      namespace current                 ;# ::oo::Obj22
      namespace path                    ;# ::oo::Helpers
      namespace which -command link     ;# ::oo::Helpers::link
      namespace which -command my       ;# ::oo::Obj22::my   <- not a helper
  } }
```

`my` / `myclass` live in the object's *own* namespace rather than
`::oo::Helpers`, which is why the trait — not a namespace name — is what
the registry records. The scoped set is `link`, `my`, `next`, `nextto`,
`self`, and `classvariable`; every one raises `invalid command name` at
the top level, inside an ordinary `proc`, and inside an `apply` lambda
written within a method body (`apply` runs its body in the global
namespace). tclsh 8.6.14 agrees for the four members it ships.

Consumers pair one registry query with one call-site fact, and neither
side carries a command name:

- `CommandRegistry::resolves_only_in_method_context(head) -> bool` — which
  commands are scoped, dialect-aware exactly like
  `method_dispatch_keyword`;
- `analyser::scope::innermost_scope_reaches_oo_helpers(root, offset)` —
  whether a byte offset sits in a frame that reaches `::oo::Helpers`.

### `TCLOO_REQUIRES_METHOD_FRAME` — "resolves here" is not "callable here"

One place makes the two facts diverge, and conflating them is a live
defect: a Tcl 9 class-level `initialise` / `initialize` body. It runs in
the *class object's* own namespace with `namespace path` = `::oo::Helpers
::oo`, so the family genuinely **resolves** there — but there is no method
context, so calling one raises. Pinned against tclsh 9.0.4, inside
`oo::class create ::P { initialize { … } }`:

```text
ns=::oo::Obj20  path=::oo::Helpers ::oo
link:          which='::oo::Helpers::link'           call -> link may only be called from inside a method
next:          which='::oo::Helpers::next'           call -> next may only be called from inside a method
nextto:        which='::oo::Helpers::nextto'         call -> nextto may only be called from inside a method
self:          which='::oo::Helpers::self'           call -> self may only be called from inside a method
classvariable: which='::oo::Helpers::classvariable'  call -> classvariable may only be called from inside a method
my:            which='::oo::Obj20::my'               call -> OK  (`my new` returns ::oo::Obj22)
```

So the split is:

| fact | scope query | registry query | consumer |
|---|---|---|---|
| resolves here | `innermost_scope_reaches_oo_helpers` | `resolves_only_in_method_context` | **W123** |
| callable here | `innermost_scope_is_oo_method_frame` | `requires_oo_method_frame` | **completion, hover** |

`W123` must stay silent in an init body — the command is not unknown —
while completion and hover must not offer a word the interpreter will
refuse. `my` is the one member that is *not* `TCLOO_REQUIRES_METHOD_FRAME`:
it is `::oo::ObjN::my`, the object's own dispatch command rather than an
`::oo::Helpers` member, and a class **is** an object, so `my new` in an
`initialize` body really does make an instance. That per-command
difference is why the second fact is a trait rather than a second scope
flag.

The scope side carries the pair as `Scope::oo_global_resolution` (reaches
`::oo::Helpers`) and `Scope::oo_method_frame` (a real method invocation);
the two predicates share one descent so they cannot disagree about which
scope is innermost. The LSP asks once through
`tcl_lsp_core::oo_dispatch::OoFrame::at(...).admits(registry, name)`.

A top-level `link` is therefore an unknown command with no hover and no
completion entry; inside a method body it resolves and is offered; inside
an `initialise` body it resolves (no W123) but only `my` is offered.

The **qualified** spellings (`oo::Helpers::link`, `…::next`, `…::nextto`,
`…::self`, `…::classvariable`) are registered as separate specs by
`commands/tcl/oo_helpers.rs`, derived from their bare twins so arity,
hover, and dialect **and package** gating cannot drift. They do **not**
carry `TCLOO_METHOD_CONTEXT`, `TCLOO_REQUIRES_METHOD_FRAME`, or the
dispatch traits: `info commands ::oo::Helpers::link` answers under tclsh
9.0.4, and calling it outside a method fails with the *runtime* error
`::oo::Helpers::link may only be called from inside a method`, not
`invalid command name`. The pattern matches `dict::qualified_specs`
(issue #923 idx 105).

`link` is derived **twice**, once per bare entry: Tcllib's `ooutil`
installs a real `::oo::Helpers::link` under 8.6/8.7, so the qualified
spelling needs the same 9.0-core-plus-`ooutil`-8.6 pair its bare twin has
or the fully qualified call reads as unknown on exactly the dialect where
a user must reach for it. The registry keys duplicates by name and picks
per dialect (`best_visible`).

> A name with several specs must be **resolved for the dialect** before
> anything is read off it. `CommandRegistry::get` returns the
> last-registered spec, which for `link` / `oo::Helpers::link` is the
> `ooutil` twin — enough to make a completion item on a Tcl 9 buffer
> describe a core command as `tcllib (ooutil)`, and to drop an ambient
> core keyword out of the generated editor keyword lists altogether.
> Consumers use `profile.resolve_command`; generators projecting a whole
> grammar union (`gen_zed_queries`, `gen_tmlanguage_keywords`) ask
> "does **any** ambient spec qualify" over `CommandRegistry::specs(name)`,
> because no single-spec lookup can answer a cross-version question.

`TCLOO_BINDS_METHOD_ALIAS` completes the `link` model: it declares that
each argument word binds a bareword alias for a method of the current
object (`NAME`, or `{NAME TARGET}`). The analyser's class-body walk finds
those calls through `CommandRegistry::binds_method_alias(head)` rather
than a `texts[0] == "link"` literal, populating `ClassDef::linked_members`
— which in turn makes the installed bareword resolve for W123, hover, and
go-to-definition inside that object's method bodies.

### Branch-selected bodies

`BRANCH_SELECTED_BODY` marks a command whose body arguments run only when
its own run-time selection picks them, and which performs no iteration —
`if` (at most one clause body plus an optional `else`) and `try` (handler
bodies reached only on the matching exception, with `finally` the one
exception that always runs).

The fact a consumer needs is that nothing established inside such a body
**dominates** the code after the command: a `package require` there is
conditional, and a variable written there is not reliably set afterwards.

The trait is command-level, so a command whose *clauses* differ — `try` — has
its per-clause answer decided by the consumer that already models the clause
grammar. `try`'s analyser hook raises `conditional_depth` for the main body
and each `on` / `trap` handler body, and not for `finally`, which always runs
(issue #1065); the per-clause table lives in
[package-loading.md](../contracts/package-loading.md#analyser-extraction).
A consumer reaching a body through the *generic* `ArgRole::Body` walk gets
the command-level answer for every body, which is correct for `if`.

Deliberately narrower than `CONTROL_FLOW`, which also covers `while` / `for`
/ `foreach` / `lmap` / `switch`. A loop body is *repeatable* as well as
skippable, so the two questions have different answers and different
consumers — the analyser's `control_flow_body_depth` (straight-line-ness,
driven by `CONTROL_FLOW`) and `conditional_depth` (domination, driven by this
trait) stay separate. Also distinct from `HAS_BOOLEAN_COND`, which is about
an argument being *read* as a boolean expression rather than about which
bodies run.

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

### Keyword abbreviations -- one resolver for every prefix spelling

Tcl's `Tcl_GetIndexFromObj` dispatch accepts **any unique prefix** of a
keyword-table entry, so `string le` is `string length` and `lsearch -noc` is
`lsearch -nocase`. `tcl_registry::abbrev` models this once; no consumer
carries its own matcher.

**Derived, not authored.** The minimal unique abbreviation is a pure
function of the table, so nothing is hand-written. Two facts *are* declared,
because they cannot be derived:

| Fact | Where | Meaning |
|---|---|---|
| `prefix_matching: PrefixMatching` | `CommandSpec`, `SubCommand` | `Enabled` (default, `Tcl_GetIndexFromObj`) or `Strict` (`TCL_INDEX_STRICT`) |
| `min_abbrev: Option<u8>` | `SubCommand`, `OptionSpec` | Documented minimum abbreviation length, when longer than uniqueness requires |

**One resolution API.** `CommandSpec::resolve_subcommand_word`,
`CommandSpec::resolve_option_word`, and `SubCommand::resolve_option_word`
build a `KeywordTable` filtered by dialect and lifecycle, then return a
three-valued `KeywordMatch`:

| Outcome | Meaning |
|---|---|
| `Unique(canonical)` | Resolves to exactly one keyword. Everything downstream — arity, arg roles, side-effect traits, safe-interp hiding, version gates — treats it exactly as the canonical spelling. |
| `Ambiguous(candidates)` | Prefixes more than one keyword. A guaranteed runtime error in real Tcl; the candidate set is what the user needs. |
| `Unknown` | Prefixes nothing. The pre-existing unknown-keyword path. |

An exact spelling always wins over a prefix (`string trim` is `trim`, not
ambiguous with `trimleft`). In a **strict** table an abbreviation is
`Unknown`, never `Ambiguous` — the user's error is an unknown keyword.

`prefix_override` on each entry point lets the analyser force `Strict` at a
call site it saw configured with `namespace ensemble … -prefixes 0`.

**Version ranges matter twice**: the table contents change between releases,
and a word is safely `Unique` for a *range* only when it resolves to the same
keyword in **every** version of it. `abbrev::resolve_over_versions` enforces
that — `string c` was `compare` in 8.5 but is ambiguous once 8.6.2 added
`cat`, so it is `Ambiguous` for an 8.5–8.6 target.

**Booleans are a built-in table.** `abbrev::boolean_table` holds
`true/false/yes/no/on/off/0/1` and reproduces `Tcl_GetBoolean` exactly,
including `o` being the one ambiguous boolean prefix.
`abbrev::resolve_boolean` returns the denoted value.

**Command names are never prefix-matched.** `str length` is a genuine unknown
command. This machinery is only for keyword tables.

`KeywordTable::minimal_unique_prefix` is the emitter's side of the same data:
the shortest legal spelling of a keyword, respecting `min_abbrev` and
returning `None` for a strict table.

### Lifecycle -- one contract for every versioned entity

`tcl_registry::lifecycle::Lifecycle` is the *only* way a registry entity
describes its availability over releases. It is carried by `CommandSpec`,
`SubCommand`, `OptionSpec`, `VersionedArgValue`, `EventProps` (iRules
events), and `ProfileSpec` (BIG-IP profile types) under the same field name,
`lifecycle`.

| Field | Meaning |
|---|---|
| `introduced` | First release where the entity exists. `None` = present in every release of its axis. |
| `deprecated` | First release where it still exists but should warn. `None` = not deprecated. |
| `retired` | First release where it no longer exists — **exclusive**. `None` = not retired. |

There is deliberately **no** generic maximum version. A known range is
described by `introduced` alone; an upper bound exists only as retirement
metadata, and `retired: 10.0.0` means the entity is already gone *in*
10.0.0.

`Lifecycle::state_at(target)` is the single decision point:

```
target >= retired                     -> LifecycleState::Retired
target <  introduced                  -> LifecycleState::NotIntroduced
target >= deprecated (still available) -> LifecycleState::Deprecated
otherwise                              -> LifecycleState::Available
```

An absent `target` is permissive (`Available`) — the registry never gates on
a version it could not resolve. `available_at`, `deprecated_at`, and
`retired_at` are thin wrappers, and `available_for_version` on each spec type
delegates to them, so completion, hover, diagnostics, the CLI, MCP, the query
engine, snapshots, and the Spec Studio schema all apply the same exclusive
retirement rule.

Consumers:

- **Diagnostics** — `W135`/`W136` (not introduced yet), `W144` (deprecated),
  `W139` (retired); `IRULE1002`/`IRULE1003` for iRules events.
- **Completion / hover** — retired items are omitted, deprecated ones are
  kept and labelled with their deprecating release.
- **Serialisation** — every JSON surface names the three fields
  `introducedVersion` / `deprecatedVersion` / `retiredVersion` (snake_case
  in the Spec Studio draft schema) with `null` meaning "never reached that
  state".

`Lifecycle::validate()` rejects impossible orderings (`deprecated <
introduced`, `retired < deprecated`, `retired < introduced`); the registry
sweep runs it over every entity so bad data cannot reach a consumer.
`with_baseline` fills an absent `introduced` from an axis baseline (the
BIG-IP surfaces declare everything present since 15.0) but never creates an
impossible ordering.

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

## Known limitations

### Lookup is by literal spelling, not by command identity

`CommandRegistry::get` / `get_for_dialect`
(`rust/tcl-registry/src/registry.rs`) resolve a segmented command's head by
an exact string match against `by_name` — a plain by-name lookup keyed on
the spelling baked into the static `CommandSpec`. Every piece of
registry-driven behaviour reached through that lookup (`arg_roles` /
`arg_role_resolver`, `taint_*`, `side_effect_hints`, `safe_on_uninit`,
`lowering`, `object_class`, `definition_body`, …) is therefore only found
when the call site's literal head text matches the spec's registered name.

Real Tcl does not work this way: `rename apply myapply` and `interp alias
{} myapply {} apply` both make `myapply` fully behaviourally identical to
`apply` at runtime — same argument handling, same effects, same result —
because Tcl resolves a command by its interpreter-level binding, not by
the spelling used to invoke it. `CommandRegistry::get` /
`arg_indices_for_role` themselves have no equivalent notion of binding;
they only ever see the token text passed in — literal head text in, literal
`by_name` match out, nothing else.

### The compiler's own lowering pipeline is not affected — a consumer can shield itself

Whether a *consumer* is blind to a rename/alias therefore depends on
whether it resolves the call's canonical name itself before handing the
head to the registry — and the compiler's IR-lowering pipeline already
does this for statically-visible bindings. `Lowering::lower_command`
(`rust/tcl-compiler/src/lowering/mod.rs`) detects `interp alias {} name {}
target ?args?` and static `rename oldName newName` as it walks a
compilation unit (via `tcl_registry::CommandTableEffect` +
[`detect_interp_alias`/`detect_rename`](../../../rust/tcl-compiler/src/alias.rs)) and
records each into a `CommandAliasMap` (`self.aliases`). `lower_default`
resolves the call's head through that map (`resolve_alias`) *before*
calling `arg_indices_for_role`, and threads the resolved canonical name
forward as `Statement`'s `canonical_command`, so codegen-hook selection,
side-effect classification, GVN purity, and var-escape all key off the
real target rather than the source spelling. Taint sink classification
(`rust/tcl-compiler/src/taint.rs`) reads that same resolved name back via
`canonical_command_or_source()` before dispatching — proven by regression
tests (`t100_fires_through_interp_alias_indirection`,
`t100_fires_through_rename_indirection`) showing `interp alias {} myEval
{} eval; myEval $tainted` and `rename eval myEval; myEval $tainted` both
still raise `T100` through the alias. So purity, side-effects, codegen
dispatch, var-escape, and taint sinks reached through this pipeline are
**not** part of this limitation for a statically-visible rename/alias —
they resolve through the same registry lookup, but only after the
canonical name has already been substituted in.

### Semantic tokens resolve identity themselves (issue #1185)

The semantic-token walker is no longer part of this limitation. It resolves
each head's **effective command identity** once, before any registry query,
through `rust/tcl-lsp-core/src/head_identity.rs`:

```rust
enum HeadIdentity<'a> {
    Command(&'a str),  // the registry name this spelling really invokes
    Rebound,           // the binding was provably taken over -- no grammar applies
}
```

`command_head_identities` scans the document's **top-level** statements once
and records an offset-keyed fact per head spelling. Which commands mutate the
command table is registry data (`CommandTableEffect`), and the argument shapes
come from the compiler's own `alias.rs` detectors -- the same ones the
IR-lowering pipeline uses -- so nothing here spells a command name. Four
sources feed it:

| Statement | Fact |
|---|---|
| `namespace import ::tcltest::*` | `test` -> `::tcltest::test` (issue #776) |
| `interp alias {} myfmt {} format` | `myfmt` and `::myfmt` -> `format` |
| `rename format origfmt` | `origfmt` -> `format`, **and** `format` -> `Rebound` |
| `proc format {args} {...}` | `format` -> `Rebound` |

Both the bare and the explicitly `::`-qualified spelling of a bound name are
recorded, and the *latest* fact at or before the call's byte offset wins, so a
binding never retroactively re-tags an earlier call:

```tcl
format {%08x} 42       ;# still the built-in -- specifier sub-tokens
rename format origfmt
origfmt {%08x} 42      ;# now the built-in
format  {%08x} 42      ;# Rebound -- a plain string argument
```

`HeadIdentity::spec_name()` answers `""` for `Rebound`, which
`CommandRegistry::get` never resolves -- so every registry query the walker
already makes (`arg_indices_for_role`, `format_string_args`,
`handle_binding`, ...) answers "unknown command" without a variant check at
each call site.

**Explicit limits.** The table is sound by abstention, and states nothing for:

- a **dynamic** binding -- `rename $old new`, `interp alias {} $n {} eval`,
  `interp alias {} n {} $t` (rejected by `is_dynamic_word`);
- an alias with **pre-bound arguments** -- `interp alias {} pad {} format %08x`
  shifts every index, so the target's layout cannot be reused; the name is
  marked `Rebound` rather than aliased;
- **another interpreter** -- a non-empty `srcPath` binds a name in a *child*
  interpreter and states nothing here; a non-empty `targetPath` marks the name
  `Rebound`. Hidden commands in a safe interpreter are invisible for the same
  reason: this is a description of *this* document's command table;
- a **conditional or nested** binding -- only top-level statements are scanned,
  so a `rename` inside an `if` / proc / `eval`, and a `proc` inside
  `namespace eval ::n` (which defines `::n::format`, not `::format`), state
  nothing;
- `unknown` fallback, traces, and computed heads -- nothing is inferred.

The other source-text consumers below are still blind to rename/alias; the
`HeadIdentityMap` is the shared pre-pass the "plausible future fix direction"
note called for, and extending it to them is a matter of threading it through,
not of inventing a mechanism.

### The limitation is real for the other source-text, re-segmentation-based consumers

The remaining blind spot is the consumers that never go through
`Lowering`'s alias table at all: the ones that recognise a shape (a
lambda literal, a body, a callback prefix) by re-parsing a segmented
command's own raw head text directly against the registry, outside the
IR-lowering pass and with no alias map available to them. This is
precisely the set of consumers the #954/#999 fix taught
`ArgRole::LambdaLiteral` awareness — semantic tokens, folding, formatting,
minification, declaration scanning, the best-effort text-based
interprocedural call-graph scanner (`tcl-compiler/src/interprocedural.rs`
— distinct from the IR-based taint/SSA interprocedural analysis, which
*is* on the alias-aware pipeline above), param-trait inference, and the
iRules object-reference walker. The concrete, verified case is `apply`'s
`ArgRole::LambdaLiteral` handling under `rename`/`interp alias`: see the
"Failure modes" section of
[the `apply`-lambda-body KCS note](../../kcs/kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)
for the reproduction and file-path anchors. The same gap applies to any
other registry-keyed field (arg roles, `defines_command_at`, …) queried the
same direct way by one of these consumers — `apply` is simply the instance
that has been reproduced and written up (issue #1002); it is not evidence
that taint or purity share it, since those two are demonstrably covered by
the mechanism described above.

This is a different mechanism from issue #973, which is about the
analyser's `known()` predicate (`scope.rs`) not gating an existence check
(W123) on deletion — a single analyser-side predicate partially growing
rename/alias-awareness for one diagnostic. The gap described here is a
missing integration (the source-text consumers have no access to an alias
table at all, static or otherwise), not a partial one.

**Fix direction** (done for semantic tokens in issue #1185, still open for
the rest): give the source-text-based consumers the same kind of alias
resolution the compiler's lowering pass already has, rather than inventing a
new mechanism. `head_identity.rs` is that shared document-wide pre-pass, and
the remaining consumers need it threaded through rather than reinvented. The
`CommandAliasMap` pattern in
`rust/tcl-compiler/src/alias.rs` is IR-lowering-specific (it's built while
walking a `CompilationUnit`, and reads `self.aliases` accumulated so far in
that walk); the source-text consumers operate on individually re-parsed
segmented commands, often outside any compilation-unit walk, so reusing it
directly is not a drop-in change — each consumer would need either its own
document-wide alias scan pre-pass, or a shared one computed once and passed
down to every consumer that currently calls `CommandRegistry::get` /
`arg_indices_for_role` directly on raw head text. That is still a real,
multi-consumer change (it touches every one of the source-text consumers
listed above, not just `apply`'s), just a narrower one than rewriting
`CommandRegistry::get` itself.

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
