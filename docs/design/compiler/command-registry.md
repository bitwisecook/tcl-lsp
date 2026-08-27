# Command registry infrastructure

How command metadata reaches the compiler: what a `CommandSpec` declares,
how definitions are registered per dialect, and which passes consume the
result. Read this when adding a command definition, or when arity, taint,
or purity information is not reaching a downstream pass.

Every Tcl command is defined in its own module under
`rust/tcl-registry/src/commands/<dialect>/<name>.rs`, which exposes a
single `pub fn spec() -> CommandSpec` returning a struct literal.  Each
dialect's `mod.rs` collects those into a `Vec<CommandSpec>` from one
`<dialect>_command_specs()` function, and `CommandRegistry` merges the
vectors into a unified lookup table.  Core specs (Tcl, stdlib, tcllib)
are always present; dialect-specific packs (Tk, iRules, iApps, EDA,
Expect) are loaded lazily on first access for that dialect.  Registry
metadata drives IR lowering, SCCP, GVN, taint, side-effects,
diagnostics, and code completion.

## Context

Every Tcl command is a `CommandSpec` value returned by a `spec()` function in
its own module under `rust/tcl-registry/src/commands/<pack>/`. Each pack
module exposes a `<pack>_command_specs() -> Vec<CommandSpec>` collector, and
`CommandRegistry` merges those into a by-name lookup table. Core packs (Tcl,
stdlib, tcllib, argparse, ticklecharts, itcl, and Tk) are built in by
`CommandRegistry::build_default`; the remaining dialect packs (iRules, iApps,
tmsh, Expect, BPF) load on demand through `load_dialect`. The EDA shells are
**not** Rust modules at all — `sdc_base` and the five vendor packs ship as
bundled `.tclspec` loadables under `specs/` and reach a registry only through
the `tcl-spectcl` loader (see [`../spec-packs.md`](../spec-packs.md)).
Registry metadata drives IR lowering, SCCP, GVN, taint, side-effects,
diagnostics, and code completion.

Source: `rust/tcl-registry/src/spec.rs` (`CommandSpec`, `SubCommand`),
`rust/tcl-registry/src/registry.rs` (`CommandRegistry`),
`rust/tcl-registry/src/cache.rs` (`registry_for_dialect`),
plus the per-fact modules beside them (`arg_role.rs`, `traits.rs`,
`taint.rs`, `hooks.rs`, `lifecycle.rs`, `side_effects.rs`).

## Content

### Architecture

```
commands/<pack>/<command>.rs :: spec() -> CommandSpec
    |
    +-> traits: Traits                       (one u128 bitset over Trait)
    |
    +-> arity: Arity                         (overall argument-count bounds)
    |
    +-> forms: &'static [FormSpec]
    |     +- kind, synopsis, arity, options, pure, mutator, side_effects
    |
    +-> subcommands: &'static [SubCommand]
    |     +- arity, traits, return_type, options, taint_transform,
    |        codegen_hook, lowering_hook, analyser_hook, ...
    |
    +-> arg_roles: &'static [(u8, ArgRole)]  (+ arg_role_resolver, repeated_args)
    |
    +-> taint_source / taint_transform / taint_*_sink* / setter_constraints
    |
    +-> dialects, lifecycle, event_requires, deprecated_replacement, ...
```

Two structural points are worth stating up front, because they change how a
spec is read as well as written:

- **Behaviour is one bitflag set, not a field per fact.** Every behavioural
  flag lives in `traits: Traits`, a `u128` bitset over the `Trait` enum
  (`rust/tcl-registry/src/traits.rs`), set as a union
  (`Traits::PURE | Traits::CSE_CANDIDATE`). A bit that is not named is unset,
  and `CommandRegistry` exposes membership queries over the set rather than
  one accessor per flag.
- **Arity is a field.** `CommandSpec::arity` carries the overall constraint
  directly.

### Defining a command

A command is a module holding a `spec()` function that returns a
`CommandSpec`, with `..CommandSpec::DEFAULT` supplying every field the
command does not declare:

```rust
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "puts",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::TAINT_SINK,
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(puts_arg_roles),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        options: const {
            &[OptionSpec {
                name: "-nonewline",
                value: OptionValue::flag(),
                detail: "Suppress the newline puts normally appends after string.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        ..CommandSpec::DEFAULT
    }
}
```

Registration is a declaration, not a decorator: add `mod puts_;` to the
pack's `mod.rs` and `puts_::spec(),` to the list its `<pack>_command_specs()`
collector returns. (The trailing underscore avoids clashing with a Rust
keyword or a std name.) The registry keys duplicates by name and picks per
dialect, so one command may have several specs — see the note on
`best_visible` under the TclOO helpers below.

### CommandSpec field reference

#### Identity and availability

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `name` | `&'static str` | *(required)* | Command name (e.g. `"lappend"`, `"dict"`) |
| `dialects` | `Option<DialectSet>` | `None` | Which dialects have this command.  `None` = all dialects.  `DialectSet` is a `bitflags` set (`rust/tcl-dialect/src/dialect_set.rs`) with composite constants such as `ALL_TCL` and `TCL85_PLUS`, combined with `union` / `\|` |
| `required_package` | `Option<&'static str>` | `None` | Only show in completions when this package has been `package require`d |
| `tcllib_package` | `Option<&'static str>` | `None` | Tcllib package that provides this command (per-document activation) |
| `warn_missing_import` | `bool` | `true` | Whether W120 fires when used without `package require`.  `false` for Tk commands (auto-loaded by `wish`) |
| `is_namespace_exported` | `bool` | `false` | Whether the source namespace exports the bare name, so `namespace import` can bring it in |
| `lifecycle` | `Lifecycle` | `UNSPECIFIED` | Introducing / deprecating / **retiring** releases on the owning package's version axis.  See [Lifecycle](#lifecycle----one-contract-for-every-versioned-entity) |

#### Documentation

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `hover` | `Option<HoverSnippet>` | `None` | Man-page summary, synopsis, snippet, and examples for hover/signature help |
| `forms` | `&'static [FormSpec]` | `&[]` | Invocation forms (getter vs setter variants).  See FormSpec section |
| `arity` | `Arity` | `Arity::any()` | Overall arity constraint, counted after the command name.  Drives W101 (wrong number of arguments) |

#### Subcommands

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `subcommands` | `&'static [SubCommand]` | `&[]` | Ensemble subcommand table, keyed by each entry's own `name`.  See SubCommand section |
| `allow_unknown_subcommands` | `bool` | `false` | Suppress W001 (unknown subcommand) for unrecognised subcommands (e.g. user-defined `oo::class` methods) |
| `prefix_matching` | `PrefixMatching` | `Enabled` | Whether this spec's keyword tables honour unique-prefix abbreviation (`Tcl_GetIndexFromObj`) or only exact spellings (`TCL_INDEX_STRICT`) |
| `implementation_namespace` | `Option<&'static str>` | `None` | Namespace the ensemble's subcommands are also individually callable under |
| `default_form_first_word` | `Option<DefaultFormFirstWord>` | `None` | Value shape a non-subcommand first word may take to select the command's *default* form (`after 200 ...` — an integer first word is a delay, not an unknown subcommand). Matched via the canonical `tcl-syntax` number parser, so every Tcl integer spelling works |

#### Compiler traits

These are **bits of the `traits` field**, not fields of their own. A spec sets
them as a union: `traits: Traits::HAS_LOOP_BODY | Traits::NEVER_INLINE_BODY`.
`rust/tcl-registry/src/traits.rs` is the authoritative list; the table below
covers the compiler-facing ones.

| Trait bit | Purpose |
|-----------|---------|
| `CREATES_DYNAMIC_BARRIER` | Lowered to `IRBarrier` -- blocks optimisations across this call |
| `HAS_LOOP_BODY` | Command has a loop body (affects dead-code analysis) |
| `NEVER_INLINE_BODY` | Body arguments must not be inlined by the optimiser |
| `LOOP_LIST_HEADER` | CFG header carries list-expression args evaluated once before the loop body (foreach, lmap) |
| `CONTROL_FLOW` | Command is a control-flow statement (break, continue, return) |
| `NEEDS_START_CMD` | Bytecode control flow: needs a `startCmd` instruction |
| `CREATES_SCOPE_ALIAS` | Creates a scope alias (upvar-like binding) |
| `ALIASES_GLOBAL` | Refines `CREATES_SCOPE_ALIAS`: binds the interpreter global namespace |
| `STRUCTURALLY_CHECKED_ARITY` | Registry `arity` is a descriptive floor only; a `clause_shape_check` hook owns real arity + shape validation, so the generic E002/E003 floor/ceiling check steps aside (`if`) |

Traits compose across levels: a `SubCommand` carries its own `traits`, and
consumers read the union of the command's and the resolved subcommand's bits.

#### Purity and optimisation

`pure` and `cse_candidate` are likewise `traits` bits (`Traits::PURE`,
`Traits::CSE_CANDIDATE`), not fields; the remaining optimisation facts are
typed descriptor fields.

| Field / trait | Type | Default | Purpose |
|-------|------|---------|---------|
| `Traits::PURE` | trait bit | unset | No side effects -- safe for SCCP to propagate through |
| `Traits::CSE_CANDIDATE` | trait bit | unset | Result can be cached by GVN (common subexpression elimination) |
| `result_stability` | `Option<ResultStability>` | `None` | Separates argument-only, versioned-world, volatile, and unknown results. Purity alone never proves replay returns the same value. |
| `world_effects` | `Option<WorldEffectDescriptor>` | `None` | Typed reads, writes, callbacks, and clobbers of mutable Tcl-world domains. |
| `state_transitions` | `Option<StateTransitionDescriptor>` | `None` | Known binding, namespace, interpreter, trace, and variable-cell identity changes; dynamic operands widen their typed domain. |
| `dispatch_dependencies` | `Option<DispatchDependencyDescriptor>` | `None` | Mutable domains whose contents must be proved stable before resolved registry semantics are treated as live dispatch. |
| `representation_effect` | `Option<RepresentationEffect>` | `None` | Tcl dual-representation and copy-on-write behaviour, independent of the inferred value type. |

These fields resolve at command, subcommand, and invocation-form specificity.
`None` is conservative. A pure, CSE-candidate command with a referentially
transparent result is still only a static GVN candidate: call reuse also needs
closed transitions, no relevant effects, and a site proof covering every
dispatch dependency. A world-state version does not prove that a command or
execution trace is absent.

#### Argument semantics

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `arg_roles` | `&'static [(u8, ArgRole)]` | `&[]` | Static arg roles: `Body`, `Expr`, `VarWrite`, `VarRead`, `Pattern`, etc. |
| `arg_role_resolver` | `Option<ArgRoleResolver>` | `None` | Dynamic arg-role resolution for variable-layout commands (if, try, switch) |
| `arg_presentation` | `&'static [(u8, ArgPresentation)]` | `&[]` | Formatter layout override per argument index -- see [ArgPresentation](#argpresentation----how-a-formatter-lays-an-argument-out) |
| `repeated_args` | `&'static [RepeatedArgLayout]` | `&[]` | Roles that recur at a fixed stride over the argument tail (`global a b c`, `foreach v l ... body`) |
| `command_prefixes` | `&'static [(u8, AppendedArity)]` | `&[]` | Static `ArgRole::CommandPrefix` positions with the arity appended to the callback |
| `command_prefix_resolver` | `Option<CommandPrefixResolver>` | `None` | Dynamic command-prefix positions (`trace add …`, `interp alias`) |
| `script_timing_resolver` | `Option<ScriptTimingResolver>` | `None` | Invocation-sensitive `SameInvocation` / `Deferred` / `ReferenceOnly` timing for positions already classified as executable |
| `callback_taint_inputs` | `&'static [(u8, &'static [CallbackTaintInput])]` | `&[]` | User-controlled substitutions injected into deferred positional callbacks; generic taint replay never infers framework metadata |
| `clause_shape_check` | `Option<ClauseShapeChecker>` | `None` | Validates a clause-chain shape a plain `min..=max` arity can't express (if's `elseif`/`else` chain -- see `tcl_registry::clause_shape`); the compiler dispatches on the hook's presence, not the command name |
| `frame_effect` | `Option<FrameEffectSpec>` | `None` | How the command crosses stack frames: the level word, the frame-selected variable arguments, and caller-frame scripts |
| `option_constraints` | `&'static [OptionConstraint]` | `&[]` | Relationships between otherwise valid leading options, including dialect gates. Drives generic W147 without naming the command. |
| `literal_argument_validator` | `Option<LiteralArgumentValidator>` | `None` | Registry callback for literal argument relationships or collection members whose legal domain depends on surrounding words. It returns Valid, Invalid with an optional replacement Tcl value, or a typed Abstain. |
| `arg_types` | `&'static [(u8, ArgTypeHint)]` | `&[]` | Per-argument type expectations (e.g. `Int`, `List`).  Drives shimmer detection |
| `return_type` | `Option<TclType>` | `None` | Return type of the command |
| `completion` | `Option<CompletionDescriptor>` | `None` | Tcl *completion-code* semantics: which return codes (`CompletionCodeDomain::Exact` / `Any`) the call can complete with, and its result / return-options payload obligations.  A resolved subcommand or invocation form may supply a more specific descriptor; `None` stays conservative |
| `body_kind` | `BodyKind` | `Plain` | Whether body arguments run in the caller's frame or a separate definition context |

#### Variable assignment

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `assigns_variable_at` | `Option<u8>` | `None` | Arg index of the variable this command writes to (e.g. 0 for `set varName value`) |
| `var_write_typing` | `VarWriteTyping` | `ReturnValue` | How the type-inference pass types the variable(s) this command *writes*, distinct from `return_type` (which types the value it *returns*).  See below |
| `safe_on_uninit` | `Option<DialectSet>` | `None` | Whether the command safely creates an uninitialised variable. `None` = not safe (W210 fires); `Some(set)` = safe only when the active dialect belongs to `set` (an empty set means every concrete dialect). The lowerer resolves the matched command/subcommand form, projects this fact into IR, and W210 consumes the resulting statement flag. A profile-less registry abstains (`false`) rather than treating its union of dialects as proof. |
| `inferred_storage_type` | `Option<StorageType>` | `None` | Inferred type for the target variable: `Dict`, `List`, or `Array` |
| `Traits::DEFINES_PROCEDURE` | trait bit | unset | Command defines a procedure (proc, method, etc.) |
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
| `excluded_events` | `&'static [&'static str]` | `&[]` | Events where this command is explicitly forbidden |
| `event_requires` | `Option<EventRequires>` | `None` | Transport, profile, and connection-side requirements.  Drives IRULE1001 |
| `event_requirement_forms` | `&'static [EventRequirementForm]` | `&[]` | Argument-prefix-specific event contracts that override `event_requires`. Drives IRULE1001. |
| `data_collection` | `Option<DataCollectionOperation>` | `None` | Protocol, collect/release/payload action, payload availability, and release policy. Drives IRULE1005–1008 and collect quick fixes. |
| `side_switch_target` | `Option<SideSwitchTarget>` | `None` | Client, server, or peer body context for a nesting-script command. |
| `event_handler_priority` | `Option<EventHandlerPriority>` | `None` | Runtime default and whether omission is reportable. BIG-IP `when` defaults to 500. |

#### Security and taint analysis

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `unsafe_command` | `bool` | `false` | Command is dangerous in iRules (IRULE2003) |
| `Traits::TAINT_SINK` | trait bit | unset | Command is a taint sink (T100) |
| `Traits::TAINT_SOURCE` | trait bit | unset | Command's result is attacker-influenced |
| `taint_source` | `Option<TaintColour>` | `None` | Colour bits the return value carries when the command is a source |
| `taint_output_sink` | `Option<&'static str>` | `None` | Output sink diagnostic code (e.g. `"IRULE3001"` for XSS) |
| `taint_output_sink_subcommands` | `&'static [&'static str]` | `&[]` | Subcommands that are output sinks. Empty = every invocation |
| `taint_log_sink` | `Option<&'static str>` | `None` | Log injection sink diagnostic code |
| `taint_network_sink_args` | `Option<&'static [u8]>` | `None` | Arg indices that are network sinks |
| `taint_code_sink_args` | `Option<&'static [u8]>` | `None` | Arg indices where a tainted value reaches eval-style evaluation |
| `taint_interp_eval_subcommands` | `&'static [&'static str]` | `&[]` | Subcommands that eval untrusted input |
| `taint_transform` | `Option<TaintColour>` | `None` | Colour bits added to tainted output |
| `taint_double_encode_colour` | `Option<TaintColour>` | `None` | Colour for double-encoding detection |
| `taint_sink_safe_colour` | `Option<TaintColour>` | `None` | Colour that suppresses T100 for this sink |
| `taint_sink_gate` | `Option<fn(&[&str]) -> bool>` | `None` | Predicate over the call's own flags deciding whether the sink applies |
| `credential_options` | `&'static [&'static str]` | `&[]` | Option flags that carry secrets (e.g. `-password`) |
| `sensitive_headers` | `&'static [&'static str]` | `&[]` | Header names whose values are secrets |
| `Traits::PASSWORD_OPTION` | trait bit | unset | Command has a password option |
| `setter_constraints` | `&'static [SetterConstraint]` | `&[]` | Required argument prefixes on setter forms (IRULE3101) |

#### Side effects

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `side_effects` | `&'static [SideEffect]` | `&[]` | Static effect declarations overriding heuristic classification.  Each `SideEffect` declares target (`Variable`, `ChannelIo`, etc.), reads/writes, connection side, and an optional dialect gate |

The coarser, target-neutral companions to this field — `world_effects`,
`state_transitions`, and `dispatch_dependencies` — are listed under
[Purity and optimisation](#purity-and-optimisation) above, since the
optimiser is what reads them.

#### Deprecation and diagnostics

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `deprecated_replacement` | `Option<&'static str>` | `None` | Replacement command name for deprecation warnings |
| `deprecated_replacement_drop_in` | `bool` | `false` | Whether the replacement accepts the deprecated argument list unchanged, so the quick fix can rewrite calls automatically |
| `Lifecycle::deprecation_fix` | `Option<DeprecationFixHook>` | `None` | Registry-owned edit plan or contextual callback behind the deprecation code action.  Carried on `lifecycle`, not as a top-level field |

#### Execution and compilation hooks

Each is a **typed ID** the consumer dispatches on, not a function pointer into
a per-command handler: the compiler holds the implementation and the registry
only names which one applies. `rust/tcl-registry/src/hooks.rs` declares them.

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `lowering_hook` | `Option<LoweringHookId>` | `None` | IR lowering specialisation |
| `codegen_hook` | `Option<CodegenHookId>` | `None` | Bytecode specialisation for the `TclVM` emitter |
| `inline_codegen_hook` | `Option<InlineCodegenHookId>` | `None` | Inline (value-position `[cmd …]` / catch-body) bytecode specialisation hook, dispatched by `tcl_compiler::codegen::{cmd_subst,control_flow}` |
| `analyser_hook` | `Option<AnalyserHookId>` | `None` | Per-command handler family in the analyser's central dispatch |
| `semantic_operation` | `Option<SemanticOperationId>` | `None` | Target-neutral operation identity selected before backend dispatch |
| `bpf_op` | `Option<&'static BpfOpSpec>` | `None` | Typed BPF-Tcl lowering descriptor |
| `const_fold` | `Option<ConstFoldFn>` | `None` | Compile-time folder returning the command's constant result |
| `const_fold_versioned` | `Option<VersionedConstFoldFn>` | `None` | Tcl-version-aware folder; takes priority over the plain folder |
| `context_gate` | `Option<ContextGate>` | `None` | Validity gate keyed on lexical or dispatch context rather than argument shape |

#### Cross-cutting

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `Traits::DIAGRAM_ACTION` | trait bit | unset | Include in diagram extraction |
| `xc_translatable` | `Option<bool>` | `None` | XC translatability.  `None` = follow default rules |
| `xc_operation` | `Option<&'static str>` | `None` | The XC operation the command maps to when translatable |
| `format_string_type` | `Option<FormatType>` | `None` | Format string metadata (e.g. `format`, `scan`) |
| `pattern_type` | `Option<PatternType>` | `None` | Pattern metadata (e.g. glob, regex) |
| `byte_array_effect` | `ByteArrayEffect` | `None` | How the command transforms a byte-array operand (S110) |
| `defines_symbol` | `Option<SymbolDef>` | `None` | Command binds a navigable definition *name* the outline lists (`tcltest::test` → test case, `tcltest::testConstraint` → constraint, `tcltest::customMatch` → match mode).  `SymbolDef` carries the name argument index, an optional description-argument index, an optional `requires_arg` (record only when that argument is present — so a `testConstraint NAME value` setter defines but the `testConstraint NAME` getter does not), and the outline category (`DefinedSymbolKind`: `Test` / `Constraint` / `Matcher`).  Every symbol consumer (document + workspace symbols) reads it generically — no command-name check.  Distinct from `traits.DEFINES_PROCEDURE` / `definition_body`, which carry the richer proc / class records |

### SubCommand field reference

SubCommand shares many fields with CommandSpec but at the subcommand level.
Only fields unique to SubCommand or with different semantics are listed;
shared fields (`arg_roles`, `return_type`, `var_write_typing`, `arg_types`,
`traits`, `side_effects`, `taint_transform`, `safe_on_uninit`, etc.) have the
same meaning as on CommandSpec.  A subcommand's `var_write_typing` overrides
its parent's when the call resolves to that subcommand (`binary scan`
destructures where the bare `binary` does not).

`pure` and `mutator` are the one place these two levels differ in shape: on
`SubCommand` they are real `bool` fields, while the command level expresses
purity through `Traits::PURE`.

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `name` | `&'static str` | *(required)* | Subcommand name (e.g. `"set"`, `"length"`) |
| `arity` | `Arity` | *(required)* | Arg count after the subcommand word |
| `detail` | `&'static str` | `""` | Short description for completion items |
| `synopsis` | `&'static str` | `""` | Usage synopsis for completion/hover |
| `pure` | `bool` | `false` | Side-effect free |
| `mutator` | `bool` | `false` | Mutates state |
| `dialects` | `Option<DialectSet>` | `None` | Override parent's dialect set.  `None` = inherit |
| `lifecycle` | `Lifecycle` | `UNSPECIFIED` | Introducing / deprecating / **retiring** releases of this subcommand on the owning package's version axis. Retirement is exclusive (`retired: 10.0.0` ⇒ gone *in* 10.0.0). On iRules commands this is compared with the existing `tclLsp.bigipVersion` / `--bigip-version` keyed BIG-IP floor. See `tcl_registry::lifecycle` |
| `versioned_arg_values` | `&[VersionedArgValue]` | `&[]` | Owning-package release ranges for individual literal values declared in `arg_values`, indexed after the subcommand word (for example, the `mcp` mode of `persist add`) |
| `destructive` | `bool` | `false` | Destructive operation (e.g. `file delete`) |
| `returns_path` | `bool` | `false` | Result is a filesystem path (`file join`, `file dirname`) |
| `is_unescape` | `bool` | `false` | Performs unescaping or decoding — undoes sanitisation in taint terms |
| `credential_arg` | `Option<u8>` | `None` | Arg index that carries a secret |
| `taint_output_sink` | `Option<&'static str>` | `None` | Per-subcommand output sink diagnostic code |
| `xc_operation` | `Option<&'static str>` | `None` | XC translation operation |
| `subcommand_forms` | `&'static [SubCommandForm]` | `&[]` | Per-form arity, roles, options, and hooks matched after the subcommand word |
| `sub_subcommands` | `&'static [SubSubCommand]` | `&[]` | Operations selected by the word after this subcommand (`info object <op>`) |
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
| `method_prefix_matching` | `PrefixMatching` | `Strict` | Whether this class's instance-method table accepts source-proven unique-prefix abbreviations; Tk widget classes opt in |

The class's `new` / `create` constructor returns an object handle of
`class_name`.  Two consumers act on this:

- **Object-handle tracking** (`tcl_compiler::object_types`) harvests
  `set VAR [Class new|create …]` provenance so a variable is known to hold an
  instance of `class_name`; it follows scalar and array-element handles across
  the top level, procedures, and method bodies.  This is *provenance*, not the
  object→class dispatch *lattice* described in
  [`../name-resolution.md`](../name-resolution.md) §5.6, which measured as a
  negative on real `TclOO` corpora (factory-return receivers dominate the ⊤
  bucket); an un-provenanced (proc-parameter) receiver is deliberately left to
  the generic shape-based option highlighting rather than resolved unsoundly.
- **Semantic tokens** resolve a `$var method …` / `[Class new] method …`
  dispatch against the class's `instance_methods` and colour the method plus
  its declared options exactly like a built-in's — the object-handle half of
  issue #748.  A method whose options are not modelled still resolves as a
  method call; its `-option value` pairs fall through to the generic option
  highlighting.

### Invocation forms -- two levels, two jobs

A command that reads one way and writes another is described at two
different depths, and the two are not interchangeable.

**`FormSpec`** (`hover.rs`, on `CommandSpec::forms` and `SubCommand::forms`)
is *documentation*: it names a form so completion and hover can show the
right synopsis line for it.

| Field | Purpose |
|-------|---------|
| `kind` | `FormKind::Default`, `Getter`, or `Setter` |
| `synopsis` | The usage line for this form |
| `dialects` | Dialect gate for the form. `None` = inherit |

**`CommandForm`** (`forms.rs`, on `CommandSpec::command_forms`, with
`SubCommandForm` its subcommand-level twin) is *behaviour*: a named form with
its own arity, roles, options, hooks, and effect descriptors, for a command
whose forms genuinely differ in what they do rather than only in how they are
written.

| Field | Purpose |
|-------|---------|
| `name` | The form's identifier |
| `arity`, `arg_roles` | Per-form argument count and roles |
| `literal_argument_prefix` | Optional known-literal words at the start of the form's arguments. Exact spelling wins; when enabled, abbreviations must uniquely identify a sibling selector word. Prefix-overlapping selectors are legal and the longest statically matched, arity-admitting form wins. A dynamic/expanded word while a longer selector remains viable abstains so parent semantics remain effective |
| `options`, `option_constraints` | Per-form switches and their relationships |
| `semantic_operation`, `lowering_hook`, `codegen_hook` | Per-form dispatch |
| `traits`, `mutator`, `side_effects` | Replacement-capable behavioural/effect refinements; `None` inherits, `Some` replaces the coarser row |
| `result_stability`, `world_effects`, `state_transitions`, `dispatch_dependencies`, `representation_effect` | Per-form optimiser facts |
| `literal_argument_validator`, `completion` | Per-form validation and completion contract |
| `dialects` | Dialect gate for the form |

How a matched form, subcommand, and command combine into one answer is set
out in [resolution order across the three
levels](#resolution-order-across-the-three-levels) below.

### Arity

```rust
pub struct Arity {
    pub min: u16,
    pub max: u16,             // Arity::UNLIMITED (u16::MAX) = unbounded
    pub step: u16,            // 0 = no parity constraint; S = min, min+S, min+2S, …
    pub also_exact: Option<u16>,  // one extra exact count, exempt from `step`
}
```

`Arity::new(min, max)` covers the common case; `Arity::stepped` takes all
three bounds for a command whose argument tail comes in groups (`array set`'s
`name value` pairs), and `with_also_exact` adds the single exception a
stepped command sometimes allows.

The arity checker emits `W101` (wrong number of arguments) when an
invocation falls outside bounds.  Each `SubCommand` has its own arity,
counted after the subcommand word.

### ArgRole -- argument semantics

| Role | Meaning |
|------|---------|
| `Body` | Tcl script body -- recursively lowered into IR |
| `Expr` | Expression -- parsed into the expression AST |
| `VarWrite` | Variable written by the command (SSA def) |
| `VarRead` | Variable read without modification |
| `LoopVarList` | Loop variable *list* (`foreach` / `lmap`) -- several names in one word |
| `ParamList` | Procedure parameter list |
| `Name` | Symbolic name (proc, namespace, class) |
| `Pattern` | Pattern or regex argument |
| `Option` | Option flag word |
| `Value` | Generic value -- the default |
| `Subcommand` | The subcommand word |
| `OptionTerminator` | The `--` terminator |
| `FormatString` / `ScanFormat` | Conversion template, written in the language `format_string_type` names |
| `Channel` | Channel identifier |
| `Index` | List/string index expression |
| `Keyword` | Fixed keyword word (`in`, `from`, `to`, `if`'s `then`/`else`) |
| `CommandName` / `CommandNameProbe` | Names a command that must exist / need not exist yet |
| `NamespaceName` | Names a namespace (`namespace children ::ns`) |
| `Boolean` / `NumericOrBoolean` | Consumed as a boolean, or as a number or boolean |
| `Result` | Becomes the command's own result (`return $w`) |
| `CommandPrefix` | A callback command reference (`lsort -command cb`) whose first word is invoked at runtime with further arguments appended; recognises a literal bareword, a braced `{cmd extra}` multi-word prefix, and a `[list cmd extra]`-quoted prefix (gated on the `BUILDS_COMMAND_PREFIX` trait, below) -- distinct from `Body` since the word is a reference, not code |
| `LambdaLiteral` | A `{argList body ?namespace?}` anonymous-lambda literal (`apply`'s argument shape) -- a *list*, not a script directly; element 0 is a parameter list, element 1 is the body to recurse into |

Command-prefix callback arity is a registry contract, not an analyser guess.
`AppendedArity` can be `Exactly(n)`, a finite non-contiguous `OneOf` set,
`AtLeast(n)`, or `Unknown`. Value-dependent resolvers receive structured
source-word facts and may expose a Tcl value only when it is literal. For
example, execution-trace operations select exactly 2, exactly 4, or `{2, 4}`
appended arguments from their literal operation list; substituted, expanded,
malformed, or invalid lists abstain. The workspace callback checker then
requires a fixed, defaulted, or trailing-`args` procedure to accept every
finite alternative.

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

Both are recognised from registry data
(`rust/tcl-registry/src/handle_binding.rs`), not by matching the command word
in the LSP's handle scan:

```rust
pub struct HandleBindingSpec {
    pub name_from: HandleName,           // which variable receives the handle
    pub class_from: HandleClassSource,   // where the class is written
    pub keyword: Option<HandleKeyword>,  // a literal word the layout requires
}

pub enum HandleName {
    Word(u8),                 // the word at this index names the variable
    Implicit(&'static str),   // a fixed variable the class system provides
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
| snit `installhull` | `DefinitionBodyGrammar::member_body_commands` (widget grammars only) | same reason, and only a `snit::widget` / `snit::widgetadaptor` has a hull, so it is absent from the plain `snit::type` grammar |

The paired grammar flag `DefinitionBodyGrammar::bare_word_construction`
says whether a family's *type command* constructs from a bare instance
name (`$type $name`, snit(n)'s "The Type Command"). It is `true` for snit
and `false` for `TclOO` / `[incr Tcl]`, and it replaced a
`metaclass.starts_with("snit::")` spelling test in the scan.

snit's `installhull using TYPE ?args…?` is the shape that forced
`HandleName` to be an enum rather than an index. It binds the widget's
**implicit** `hull` component, whose name appears nowhere in the call, so
`HandleName::Implicit("hull")` supplies it from the descriptor. snit(n)
documents a second form, `installhull $win`, which names an
already-created widget and carries no static type word; the required
`using` keyword makes `resolve` abstain on it.

**Limits.** The descriptor covers a *fixed* pair of positions plus one
optional literal keyword -- enough for the three shapes above and for a
comparable installer in another class system, and deliberately not a
general option parser. `install NAME $widget` (a run-time-typed
component) is not modelled -- there is no static class word, so the scan
abstains.

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

**`self`'s one dispatchable value (issue #1322).** `TCLOO_INTROSPECTION`'s
"never dispatch" is true for eight of `self`'s nine closed subcommand
words, but not the ninth: a bare `self` call (no argument at all) and the
explicit `self object` both return the current object's own command
name, and a bracketed substitution of either (`[self] m` / `[self
object] m`) reaches the same target `my m` does — TclOO's own spelling
for the same-object dispatch idiom. This is a narrower, additional fact
about specific words of `self`'s closed set, not a fourth
`MethodDispatchKind` axis — unlike `my`, the value only dispatches once
*substituted* as a command head, and unlike plain introspection, this one
specific word's result is the receiver itself.

`CommandSpec::self_receiver_words: &'static [&'static str]` names the
words (`self`'s `object`, and nothing else in the registry today) for
which this holds; a bare call also counts whenever the command's own
`Arity` permits omitting the argument. `CommandRegistry::is_self_receiver_call(cmd, arg)`
is the query — parse the substitution's head and first argument (or
`None`) with `value_shapes::parse_command_substitution`, then ask the
registry, rather than matching `"self"` in a consumer.

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

1. **`arg_role_resolver`** (dynamic callback,
   `fn(args: &[&str]) -> Vec<(u8, ArgRole)>`) — inspects the actual argument
   list at analysis time. Used for variable-arity commands where roles depend
   on argument count or content. Examples: `set` returns `VarWrite` for arg 0
   when two arguments are present but `VarRead` when only one is present; `if`
   maps body and expression positions by scanning for `elseif`/`else`
   keywords.
2. **`arg_roles`** (static table) — a fixed `&'static [(u8, ArgRole)]` on the
   spec. Sufficient when every invocation has the same layout.
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
  The parent's `arg_role_resolver`, or the subcommand's own `arg_roles` and
  `repeated_args`, assign roles to the remaining arguments.
- The **analyser** dispatches on `analyser_hook`, whose `AnalyserHookId`
  values include the compound forms directly (`DictFor`, `DictUpdate`,
  `DictWith`, `NamespaceUpvar`, `InterpAlias`, …), so scope handling for a
  compound command is selected by ID rather than by name.
- The **lowering** and **codegen** layers dispatch on `lowering_hook` /
  `codegen_hook` in the same way.

When verifying whether a compound command is handled, check which hook IDs
its spec carries before looking in a consumer: an unhandled compound form is
usually a missing subcommand entry or an unset hook ID, not a missing branch.

### OptionSpec and option terminators

`OptionSpec { name, value, detail, dialects, aliases, lifecycle, min_abbrev }`
declares `-flag` switches; `value` is an `OptionValue` saying whether the
flag consumes a following word.  An `OptionSpec` whose `name` is `"--"`,
on a `CommandSpec`, `SubCommand`, or `CommandForm`, declares `--` support;
W304 ("use `--` before dynamic pattern") is derived automatically via
`CommandRegistry::resolve_option_terminator`.

#### The audit-registry option-surface gate

The `OptionSpec` tables are the source of truth for which options a command
has, so anything else that enumerates options is a second copy that can
drift.  The dialect audit (`rust/xtask/src/audit_option_dialects.rs`) is one
such copy: its `PROBES` table names ~100 command/option pairs and measures
them against real tclsh 8.4-9.0.  It drifted once already -- the audit probed
`fconfigure -profile` (TIP 656, Tcl 9.0) while the registry declared no such
option, and the omission was found by hand rather than by a gate (issue
#1396).

`cargo xtask audit-option-dialects --check` (wired into `make xtask-check`
as `xtask-option-registry-drift`) closes that hole: every option the audit
probes must be declared by the registry, or the gate fails naming the site.
It sources the option surface from the registry rather than restating it,
runs no tclsh, and needs no built Tcl trees.  The equivalent assertion runs
under `cargo test -p xtask` as `probe_options_exist_in_registry`.

What counts as declared is the command's whole option surface -- its own
`options`, every `CommandForm`'s options, and every `SubCommand`'s options,
including declared aliases, with no dialect or package-version filter.  The
probe table's subcommand column records where the *probe script* exercises
the option, not where the registry must declare it: an ensemble may hang one
shared table off the command (`string -nocase`) or off each member (`clock
scan -format`), and `encoding -profile` is probed with no subcommand column
at all yet is declared on `encoding convertfrom`.  Insisting on a particular
declaration site would flag registry-modelling choices instead of the one
drift class this guards -- an option surface the audit knows about and the
registry has never heard of.  Version gating is deliberately not filtered
either: a 9.0-only option is still declared, and whether its `dialects` gate
is *correct* is what the tclsh audit itself measures.

A genuinely-missing option goes in `KNOWN_UNSPECIFIED` with the issue
tracking the registry work -- migration debt is tracked, not grandfathered.
The list is currently empty.  A waiver whose option has since been declared,
or that names no probe, fails the gate too, so an entry cannot outlive the
gap it documents.

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
| `deprecation_fix` | Registry-owned typed edit hook used while the lifecycle state is deprecated. `None` = diagnostic only. |

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

A deprecation edit hook receives the matched word, all surrounding invocation
words with literal/dynamic facts, the active dialect, and the effective
version. It returns an owned word or invocation edit plan, including its
description and safety, or abstains. Declarative fixed replacements decline on
dynamic words; a contextual shape rewrite uses a custom registry callback and
must prove which original words it can preserve. The generic analyser supplies
source spans and materialises the code action. It never matches a command name
or invents a replacement.

`Lifecycle::validate()` rejects impossible orderings (`deprecated <
introduced`, `retired < deprecated`, `retired < introduced`); the registry
sweep runs it over every entity so bad data cannot reach a consumer.
`with_baseline` fills an absent `introduced` from an axis baseline (the
BIG-IP surfaces declare everything present since 15.0) but never creates an
impossible ordering.

### ArgValue -- completions

An `ArgValue` provides completion text and hover documentation for a literal
value at a specific argument position. They hang off `arg_values`, a
`&'static [(u8, &'static [ArgValue])]` keyed by argument index; listing an
index under `closed_value_args` additionally makes the declared set
exhaustive, so a value outside it is reported (W127).

### HoverSnippet -- documentation

`HoverSnippet` carries `summary`, `synopsis`, `snippet`, `source`,
`examples`, and `return_value`, and appears on `CommandSpec::hover`,
`SubCommand::hover`, and an `ArgValue`'s own hover.

### ArgTypeHint -- type expectations

`ArgTypeHint` declares what Tcl internal representation a command expects at
an argument position, and whether reaching it would shimmer the value.  Used
by type inference and shimmer detection.

### safe_on_uninit -- variable initialisation safety

Some commands safely create an uninitialised variable rather than erroring:
`lappend` creates an empty list, `append` an empty string, `incr` treats
the variable as `0` (8.5+ only), and `dict set`/`dict append`/`dict lappend`/
`dict incr` create an empty dict.

This fact is set on `CommandSpec` (for top-level commands) or `SubCommand`
(for ensemble subcommands like `dict set`).  The value is an
`Option<DialectSet>`:

| Value | Meaning |
|-------|---------|
| `None` | Not safe -- W210 fires if variable is read before set |
| `Some(DialectSet::ALL_TCL)` | Safe in every Tcl dialect (`append`, `lappend`, `dict set`) |
| `Some(DialectSet::TCL85_PLUS)` | Safe only from 8.5 onwards (`incr`, which errors in 8.4 and iRules) |
| `Some(…)` any other set | Safe only in exactly those dialects |

The version-gated sets are `DialectSet` constants
(`rust/tcl-dialect/src/dialect_set.rs`), so a derived dialect inherits the
right answer from the bits it contains rather than from a name comparison.

**Current state (verified 2026-08-15):** the registry resolves the most
specific declared value (matched subcommand, otherwise command) into
`InvocationSemantics`. Lowering evaluates that `DialectSet` against the
active profile and writes the result to `Statement::Call` (including
structured `dict` writers) or the specialised `Statement::Incr`.
`use_site_safe_initialises` consumes the resulting IR flag when deciding
whether W210 applies to the command's own read-before-write. A profile-less
registry is deliberately conservative: its availability mask is a union,
not a runtime guarantee, so lowering writes `false`. `ArgRole::VarWrite`
still records the eventual definition; it is distinct from this
read-before-write safety fact.

### Dialect loading

`CommandRegistry::build_default` builds the always-present surface: the
`tcl`, `stdlib`, `tcllib`, `argparse`, `ticklecharts`, and `itcl` packs, plus
`tk` (folded in because a script may `package require Tk` at run time, so Tk
commands must be recognised under every Tcl dialect; the `TK` bit is marked
loaded so a later `load_dialect` call is a no-op rather than a double
insert).

The remaining packs load on demand:

1. **`load_dialect(DialectSet)`** matches the dialect bit to its pack
   collector — `BPF`, `IRULES`, `IAPPS`, `TMSH` (the `tmsh::` subset of the
   iApps pack), `TK`, and `EXPECT`. It is idempotent: `loaded_dialects`
   records what is already in, and an unrecognised bit loads nothing.
2. **`tcl_spectcl::bundled::registry_for_dialect(name)`** handles the EDA
   shells, which are modelled as a base Tcl version plus
   `required_package`-gated libraries rather than a dialect bit — and whose
   libraries are **bundled `.tclspec` loadables**, not compiled-in Rust
   (`docs/design/spec-packs.md`). It installs the shared `sdc_base` library
   plus the vendor's own pack, filtered to the packages the profile ships
   ambient. Any consumer that may be handed an EDA dialect name goes through
   this door rather than the one below; the CLI, the MCP server, and the LSP
   server all do.
3. **`registry_for_dialect(name)`** (`cache.rs`) is the compiled-in half: it
   resolves the name to a `DialectProfile` and returns a cached, fully-loaded
   `&'static CommandRegistry` for it. Every dialect but the five EDA shells is
   complete from here.

Because each profile's registry is built once and cached, there is no
invalidation protocol to observe — a consumer holds an immutable registry
for the dialect it asked about.

### How registry feeds the compiler

| Stage | Registry fields used |
|-------|---------------------|
| IR lowering | `arg_roles` (`Body`, `Expr`, `VarWrite`), `safe_on_uninit`, `lowering_hook` |
| CFG | `Traits::CREATES_DYNAMIC_BARRIER` -> `IRBarrier` |
| SSA/SCCP | resolved value, effect, transition, alias, and completion facts |
| GVN | `Traits::PURE`, `Traits::CSE_CANDIDATE`, `result_stability`, effects, closed transitions, and a site proof covering `dispatch_dependencies` |
| Codegen | `codegen_hook` / `inline_codegen_hook` -> specialised bytecode |
| Taint | `taint_source`, the `taint_*_sink*` fields, `taint_transform` |
| Side effects | `side_effects`, plus `pure` / `mutator` on forms and subcommands |
| Diagnostics | arity and clause shape; literal validators; option constraints; lifecycle/version gates; representation effects; event and safety contracts |
| Code actions | registry-owned lifecycle and literal-validation edit plans; the generic analyser contributes only source spans and LSP conversion |
| Completions | `arg_values`, `versioned_arg_values`, `options` |

### Resolution order across the three levels

`tcl_registry::resolved_invocation` resolves a call once, against the
command, the matched subcommand, and the matched `CommandForm`, and hands
consumers a single `InvocationSemantics`. Two different rules apply, and
mixing them up is a real source of bugs:

- **Most-specific-wins**, for the optional descriptor facts: the form's value
  if set, else the subcommand's, else the command's. This covers
  `completion`, `result_stability`, `representation_effect`, and
  `lowering_hook`, among others. Arity likewise takes the form's, else the
  subcommand's, else the command's. A form's optional `traits`, `mutator`, and
  `side_effects` are replacement refinements too: `Some(&[])` is a deliberate
  proof of no legacy side effects, not an absent declaration.
- **Union until a form refines it**, for command/subcommand traits: the
  command's `traits` **or** the subcommand's, with `pure: true` folded in as
  `Traits::PURE`. When a matched form supplies `traits: Some(...)`, that value
  replaces the inherited union. This is what lets a zero-argument widget query
  remove mutation-only callback traits from its neutral parent method row.

`CommandRegistry::resolve_instance_invocation` uses the same projection for
`$object method ...`, but does not inherit the class factory command's
constructor-only traits and effects. It resolves the instance method and its
`SubCommandForm` from the registry, retaining the receiver and concrete
argument shape in `ResolvedInvocation`; consumers do not branch on class or
method names.

## Known limitations

### Lookup is by literal spelling, not by command identity

`CommandRegistry::get` / `get_for_dialect`
(`rust/tcl-registry/src/registry.rs`) resolve a segmented command's head by
an exact string match against `by_name` — a plain by-name lookup keyed on
the spelling baked into the static `CommandSpec`. Every piece of
registry-driven behaviour reached through that lookup (`arg_roles` /
`arg_role_resolver`, `taint_*`, `side_effects`, `safe_on_uninit`,
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

### The source-text consumers resolve identity themselves (issues #1185, #1275)

The source-text consumers are no longer part of this limitation. Each resolves
its head's **effective command identity** once, before any registry query,
through the document's realm command-binding state
(`rust/tcl-compiler/src/realm.rs` — the P1a home of what
`head_identity.rs` used to carry, ledger C4):

```rust
enum RealmBinding<'a> {
    Command(&'a str),  // the registry name this spelling really invokes
    Rebound,           // the binding was provably taken over -- no grammar applies
}
```

`document_realm_bindings` scans the document's **top-level** statements once
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

A binding **source** is read through the map before the registry, so chains
compose: `interp alias {} a {} format; rename a b` leaves `b` naming `format`
(tclsh 8.6.16 and 9.0.4, byte-identical: `b %08x 42` answers `0000002a` while
`info commands a` answers empty), and `proc format {…} {…}; rename format
myfmt` correctly leaves `myfmt` `Rebound` rather than inheriting the built-in's
grammar. An alias always *takes over* the name it binds, because C Tcl lets one
shadow an existing command outright.

Both the bare and the explicitly `::`-qualified spelling of a bound name are
recorded, and the *latest* fact at or before the call's byte offset wins, so a
binding never retroactively re-tags an earlier call:

```tcl
format {%08x} 42       ;# still the built-in -- specifier sub-tokens
rename format origfmt
origfmt {%08x} 42      ;# now the built-in
format  {%08x} 42      ;# Rebound -- a plain string argument
```

`RealmBinding::spec_name()` answers `""` for `Rebound`, which
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

#### Positioned and unpositioned readers

`resolve(head, at)` answers for a head at a known byte offset. Several
consumers re-lex a body out of its own *decoded* text — the formatter
reformats `arg.text`, the minifier re-minifies a body slice, the call-graph
and param-trait scans segment a body string at offset 0 — so no
document-absolute offset exists at the point of the query. Those read
`resolve_unpositioned(head)`, which folds *every* fact about the spelling and
abstains (`Rebound`) unless they agree. A document that binds one name twice
cannot be read without a position, and guessing one of the two bindings is
exactly the fall-back-to-spelling this module exists to remove.

`HeadWords { written, resolved }` carries both forms where the distinction
matters. A **global command** lookup reads `resolved`; a **lexical** test reads
`written`, because a class-body member sub-keyword (`method`, `constructor`) or
a `$var` head is not a command binding at all — a top-level `rename method …`
says nothing about the word inside an `oo::define`.

#### Which consumers resolve, and how

| Consumer | Reads | Notes |
|---|---|---|
| semantic tokens | positioned | issue #1185 |
| inlay hints (format specifiers) | positioned | issue #1185 |
| folding | positioned | body / lambda / clause-list roles |
| the declaration scan | positioned | scope-alias grammar + body recursion |
| the iRules object-ref walker | positioned | reference args, roles, `set` constant propagation |
| the minifier's rename-barrier scan | positioned | the invocation carries its own range |
| the minifier's keyword abbreviation | positioned | `base` makes a nested word's offset absolute |
| the minifier's render path | unpositioned | re-minifies each body from its own slice |
| formatting | unpositioned | re-lexes decoded `arg.text`; range formatting lexes a slice |
| the call-graph scan (`interprocedural.rs`) | unpositioned | walks lowered statements and body text at offset 0 |
| param-trait inference | unpositioned | scans a proc body from its own text |

Range formatting builds the map from the **whole document**, not the selected
slice, so a `rename` above the selection still governs it. The analyser builds
it alongside its registry at the top of every entry point, so a per-proc
param-trait scan reads the *document's* bindings while scanning a *body*.

This is a different mechanism from issue #973, which is about the
analyser's `known()` predicate (`scope.rs`) not gating an existence check
(W123) on deletion — a single analyser-side predicate partially growing
rename/alias-awareness for one diagnostic.

**Still by spelling.** `CommandRegistry::get` itself is unchanged: it is a
by-name lookup, and every consumer above resolves *before* calling it. A
consumer added later that queries the registry on raw head text re-opens the
gap for itself; the `apply`-lambda reproduction in the
is what that looks like from the outside.

## Decision rule

- To add a new command: add a module under
  `rust/tcl-registry/src/commands/<pack>/` whose `spec()` returns a
  `CommandSpec` ending in `..CommandSpec::DEFAULT`, then declare it (`mod
  foo_;`) and list `foo_::spec(),` in the pack's `<pack>_command_specs()`
  collector.  For a new dialect pack, add its collector to
  `CommandRegistry::load_dialect`.  An EDA/vendor *library* is not a Rust
  module: add or edit its `.tclspec` under `specs/` instead — those packs are
  the source of truth for their commands and there is no generator to re-run
  (see [`../spec-packs.md`](../spec-packs.md)).
- To add taint tracking: set `taint_source` / `taint_transform` / the
  `taint_*_sink*` fields on the spec, and the `TAINT_SOURCE` / `TAINT_SINK`
  trait bits that go with them.
- To add special lowering: set `lowering_hook` on the `CommandSpec` or
  `SubCommand` to the matching `LoweringHookId`.
- If arity validation fails to fire, check that `arity` is set and subcommand
  arities are correct — and that `Traits::STRUCTURALLY_CHECKED_ARITY` is not
  set, since it stands the generic check down in favour of
  `clause_shape_check`.
- Purity flows from command -> subcommand -> form; the most specific level wins.
- To mark a command as safe on uninitialised variables: set `safe_on_uninit`
  on the `CommandSpec` or `SubCommand`.  Use `Some(DialectSet::ALL_TCL)` for
  every dialect, or a version-gated constant such as
  `Some(DialectSet::TCL85_PLUS)`.  Never hardcode dialect names in the
  compiler or analyser.
- Prefer a new `CommandSpec` field or a typed hook ID over teaching a
  consumer a command name; the registry is the source of truth, and a
  consumer that matches on a name is the thing this design exists to avoid.

## The reference manual

[`docs/references/command-spec/`](../../references/command-spec/README.md)
is the library-author-facing manual over this contract: every field in
Tcl terms (generated from the Spec Studio schema, so it cannot drift),
plus the impact tables mapping fields to the diagnostics, optimisations,
and editor features they drive.

## Authoring a spec without Rust

The primary non-Rust authoring format is **SpecTcl**: a `.tclspec` file,
written in a Tcl dialect built for exactly this, that declares the same
facts this page describes as `speclib` / `command` / `option` / `arg`
statements instead of a `CommandSpec` literal. It is its own compiled-in
command pack (`rust/tcl-registry/src/commands/spectcl/`), so a `.tclspec`
file gets full editor support — highlighting, completion, and diagnostics
for a misspelled trait or role — with no extra tooling. The frozen syntax
is [`spec-dsl-examples/README.md`](../spec-dsl-examples/README.md); the
architecture, discovery tiers, and crash-containment guarantee are
[`spec-packs.md`](../spec-packs.md).

Two pieces of that design are implemented and two are still landing.
Implemented: the `.tclspec` dialect and its self-spec pack (editor
support), the parser (`tcl-spectcl::load_pack`), and its validation report,
exposed today as the `spectcl_check` MCP tool — it loads a pack for real
and reports which fields each declaration set, every dropped or
misspelled word, every declared hook, and any collision with a shipped
name. Landing: the three-tier discovery, pack merge, and compiled-pack
cache exist as a library (`tcl-spectcl::discovery` / `pack` / `cache`) but
are not yet wired into the running language server or exposed as an editor
setting, so a pack dropped in a workspace or config directory does not yet
change what the editor shows; a `tcl spec check` CLI equivalent and the
Spec Studio's DSL tab are likewise designed, not shipped.

The [Command Spec Studio](../contracts/command-spec-studio.md) is the other
non-Rust route today: a browser front-end over this registry that browses
the live command surface, edits every field described above, and renders
the result back out as a drop-in `.rs` module or a stub. Each field carries
a plain-language explanation written for Tcl developers, and a Reference
tab searches the whole vocabulary — every field, trait, argument role, and
taint colour.

See the KCS how-tos: [creating a command spec without knowing
Rust](../../kcs/kcs-howto-create-command-specs-without-rust.md) and [writing
a SpecTcl pack](../../kcs/kcs-howto-write-a-tclspec-pack.md).

## Related docs

- [Command infrastructure in walkthroughs](../../../docs/design/example-script-walkthroughs.md#command-infrastructure)
- [kcs-lowering-dispatch.md](../../../docs/design/compiler/lowering-dispatch.md)
- [kcs-taint-analysis.md](../../../docs/design/compiler/taint-analysis.md)
- [kcs-side-effects-system.md](../../../docs/design/compiler/side-effects-system.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [kcs-dialects-events.md](../../../docs/design/compiler/dialects-events.md)
