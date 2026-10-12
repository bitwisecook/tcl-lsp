# Contract: resolved semantic queries

Consumers share stateful owners for command bindings, variable cells, execution
regions, and dialect policy. Lexical spelling, registry availability, and a live
implementation are separate inputs. A registry hit supplies candidate semantics;
it does not prove that the interpreter will dispatch that implementation.

This contract describes the implemented APIs and their uncertainty boundaries.
The owner inventory and its drift gate are in
[shared-utility-contracts-rust.md](shared-utility-contracts-rust.md).
Implementer recipes, proof-purpose boundaries, consumer review tables, and
discriminating test guidance are in
[name-resolution-implementer-guide.md](../compiler/name-resolution-implementer-guide.md).

## Choose the semantic axis before querying

| Question | Owner and entry point | Required inputs |
| --- | --- | --- |
| Which qualified names are searched? | `tcl_syntax::naming::command_resolution_candidates`, `resolve_command_with` | Actual command namespace, namespace path, written name, command existence |
| Which implementation can dispatch at this source invocation? | `tcl_compiler::command_binding::SourceCommandBindings::invocation_at_source` | Whole source instance, selected lexer grammar and registry, exact dispatch offset, explicit entry/loader contracts |
| What identity should an editor project at an arbitrary position? | `tcl_compiler::realm::CommandBindingRealm::binding_at`, `head_words` | Document realm, source position; this projection is assistance evidence |
| Which command can dispatch in executable control flow? | `analyse_command_binding`; executable `analyse_dispatch_stability` | Retained IR, joined entry and mutation facts, registry dependencies |
| Which storage cell does this access touch? | `variable_bindings::build_point_resolve_contexts_with_entry`; `var_resolve::resolve_place` | Actual activation entry, CFG site, selected registry, reference syntax |
| Which cell did a substitution read before later words changed aliases? | `CommandTokens::variable_access_for_site`; `PointResolveContexts::source_reads_at` | Exact retained reference site and the source owner's reached-read context |
| Which SSA contents version belongs to that read? | `SsaSourceView::read_reference`, `read_word` | Retained source reference, cell identity, reaching represented contents-store provenance |
| May two accesses interfere? | `place::overlap`; `place_bridge::read_places`, `def_places` | Point-specific contexts, cell ownership/generation, trace observation, array/dict selectors |
| What do a command's words mean? | `CommandRegistry::resolve_structured_invocation`, `command_binding_transitions` | Source-aware `InvocationWords`, selected dialect surface, proved implementation when used to transform source |
| Which facts and argument indices may an IR statement consumer use together? | `registry_invocation::resolved_statement_invocation` | Retained statement tokens and source binding, registry and selected semantic context |
| Which command words agree across advisory candidates? | `RegistryInvocationAssistance::unanimous_command_words` | Nonempty closed lookup; every command identity, effective word, origin and frozen prefix agrees; no role or execution authority |
| Which scripts does a wrapper execute? | `BodyExecutionSpec::select`; `EvaluatedBodyRegion` | Authored registry grammar, literal operands, proved implementation/provider/hook prerequisites |
| How are frames, package versions, or namespace imports interpreted? | `DialectProfile` policy accessors; `frame_effect`; release-parameterised version helpers | Selected profile, runtime family, independent numeral/word grammar |
| Can an incremental result reuse this environment? | `DialectProfileKey`; `CommandRegistry::snapshot` | Complete profile value, registry layers and overlay order, grammar, entry and provider dependencies |
| What does the independent interpreter do? | `tcl_test_support` discovery/execution; `tcl_syntax::execution_conformance` | Requested exact C releases or pinned Jim revision, actual feature probes, shared vectors |

Do not implement a new command-name candidate loop, declaration scan, package
version comparison, frame-level digit test, or alias-following loop inside a
consumer. Add the missing contract to its owner and pass the necessary context.

The compiler convenience entry points select an explicit authoring target:
the supplied profile, the registry's attached profile, or the compiler's
documented Tcl 9.0 native default. When the actual entry dialect is known,
`LexerConfig::with_grammar` selects its lexical axes before segmentation,
preserving the supplied coordinates, strict-quoting mode and leading-BOM mode.
Unknown execution dialects retain the supplied lexical configuration.
`LexerConfig::grammar_over` describes lexical axes without changing numeral or
expression policy or identifying a native engine; it cannot supply execution
provenance. Nested consumers use `CommandTokens::native_lexer_config` so they
read the same original word grammar as the source owner.
An explicit Jim profile therefore remains Jim even when its C Tcl release is
absent. `SourceCommandBindings::*_with_options` and
`Lowerer::set_source_analysis_options` use the caller's entry contract directly;
missing runtime policies stay missing. Embedding hosts and interpreters with
retained runtime state use these explicit entry points rather than the fresh
authoring convenience contract.

## Retain navigation identity separately

`SourceInvocationBinding::command_reference` selects a candidate value in the
actual post-argument lookup snapshot. The privately constructed carrier retains
the called slot, its direct command token or imported origin, and an optional
current source definition allocation. It grants navigation only. Alias target
execution, native compiler selection, object class and normal completion require
their independent queries. Unrelated namespace tails are never lookup candidates.

`SourceCommandReference::definition` is the current implementation allocation;
its source instance, offset and incarnation survive into signature and workspace
invocation records. A surviving token's original declaration cannot replace it
after redefinition. A Class binding kind includes instances; only a genuine
class-definition receipt donates a class declaration. `proc_for_definition`
projects an exact original Authored allocation to retained declaration tokens.
Loaded/materialised origins cannot donate the same document offsets. Navigation
consensus and writable-span consensus are independent; a shared literal consumed
in different implementation worlds cannot claim one editable target.

`CommandBindingRealm::original_declaration_assistance` projects the unanimous
original declaration grammar for source navigation. It returns declaration names
with their written argument positions, plus body and lambda argument positions.
The query uses the retained declaration frame, pre-operand command table, alias
prefixes and frozen argument values. Prefix-only and expanded operands cannot
donate a written token. Callers stamp the unchanged `CommandTokens` through the
same realm before querying; they must not recover roles from the written head
or index effective arguments directly into original source tokens.

`state_transition::local_alias_name` delegates `global` and namespace `variable`
to their separate selected naming recipes. C consumes qualification runs as
one separator. Jim's rooted `global` creates no local alias, while its unrooted
operand retains its full name; namespace `variable` uses Jim's namespace tail.
Missing native naming policies and non-Unicode projections remain opaque.
Declaration navigation supplies no successful link, physical variable cell,
executed body or completed store.

## Command bindings and source recipes

Build `SourceCommandBindings` once for a source instance with
`analyse_in_namespace_with_options`. `SourceAnalysisOptions` carries
`unknown_entry`, `invocation_dialect`, `native_compilation`, and driver-supplied
`trusted_package_loaders`, plus `declared_commands` for document assistance.
`SourceAnalysisEntry` retains those entry facts with
source IR so later consumers receive the same contract.
`ModuleCommandBindings::analyse` consumes the lowering-owned
`RetainedSourceModuleBindings` projection when its original source image, lexer
axes, registry semantic key, namespace, actual entry frame and compilation scope
still match. A changed input withdraws reuse and invokes the same original-source
owner with the module's effective grammar and entry contract. Consumers must use
this query rather than reading the retained projection directly or reconstructing
a frame from a namespace spelling. `TrustedPackageLoader` records the selected provider,
exact version when established, and installed command/export surface. A package
advertisement or a written `package require` is not provider provenance.
`SourceInvocationBinding::variable_context` retains the same point-specific
activation, alias, trace, generation and allocation knowledge used by the
variable owner. `variable_frame` and `namespace_cell_presence` remain explicit;
namespace existence alone does not establish that a relative cell exists.
Document declarations describe assistance shape; they do not prove a runtime
command is installed. Retain that distinction in result types. A declaration's
body/argument roles can guide editing under its unchanged command slot, while
renaming, defining, or importing a different implementation invalidates that
slot's assistance association. Those roles cannot prove effects, purity, frame
mutation, or a transformation's dispatch prerequisites.
Use `resolved_declared_assistance(surface, tokens)` for that separate projection.
Its `DeclaredInvocationAssistance` carries the authored declaration/provenance
and written argument role positions; it contains no runtime invocation facts.
The declaration's applicability comes from the shared point binding, and opaque
or expanded words decline a fixed role projection. `DocumentCommandSurface`
owns declaration access through `declared_command`.

For semantic lowering, use the exact dispatch offset with
`invocation_at_source(head, offset)`. This query returns unknown for a missing
site. `invocation_at` and `projection_at_source` are range projections;
`invocation_unpositioned` joins reachable document states. Those broader queries
must not license a source rewrite merely because one candidate remains visible.
Nested bodies need the owner's recorded namespace and offset; parsing a body in
isolation without its activation entry loses mutation history.

`SourceInvocationBinding` retains `targets`, `may_be_absent`, and `unknown`.
`proved_target()` succeeds only for a single target with neither absence nor an
unknown residual. A target retains its terminal command, alias-prepended
arguments, and `registry_backed` flag. These fields prevent a document procedure
named like a builtin from inheriting builtin semantics.

`proved_target()` describes live lookup after argument evaluation. Executable
consumers use `proved_execution_target()`: it first accounts for a native
compiled operation selected before that evaluation. The two targets may differ.
`compiled_candidates`, `may_use_live_dispatch`, and
`compiled_execution_unknown()` retain possible early selection alongside ordinary
lookup. A possible compiled candidate does not license an unconditional native
operation. Do not replace these fields with a single “builtin” boolean.

Compiler admission has its own immutable projection.
`native_compilation_admission_selection()` retains the recipe selected when the
chunk compiled; later callback uncertainty does not change that decision.
`admitted_inline_invocation()` and `admitted_named_invocation()` preserve its
actual registration, source instance and compiler prerequisites. They do not
prove a reached handler or normal continuation. `AdmittedNativeCompilerInvocation`
exposes only compiler hooks, original operand layouts and compiler contexts;
its general invocation facts remain private. Guarded bytecode lowering uses
`try_lower_admitted_hook_with_binding` and preserves the operation's exact
guard/replay plan. Semantic consumers and executable erasure continue to use
strict execution or the separately authored normal-transfer query.

`proved_handler_target()` is a narrower, separate identity-convergence query:
the same native token must remain selected at compilation and both argv
boundaries, with observer closure established. It cannot establish an opcode
or a body protocol when native compilation remains unknown. Normal variable
transfer uses the purpose-typed `NormalTransferInvocation` adapter and the
registry's `successful_handler_effects` contract. Strict executable consumers
continue to require `proved_execution_target()`.

For a generic call whose compiler selection is unresolved, the normal store
query keeps that uncertainty while resolving the cell and operand:

```rust,ignore
let binding = tokens.source_binding.as_ref()?;
let normal = normal_transfer_invocation(registry, semantic_context, tokens)?;
let state = &binding.variable_context; // after argv; before this handler's store
if let Some(value) = normal.stored_value_literal(state, registry) {
    // A frozen value for a bounded, observer-free destination.
    return Some(value);
}
let (_spelling, original_word) = normal.stored_value_operand(state, registry)?;
let reference = SsaSourceView::at_statement(ssa, block, index).read_word(original_word)?;
let version = reference.version?;
// Read the lattice by (reference.symbol, version), retaining its uncertainty.
```

A computed destination in `set $n [set n NEW]` is the frozen first argument,
not the later contents of `n`. An alias reselected by a later argument resolves
the store through the post-argv context. A read on the value operand keeps its
own earlier source reference. Installing a write observer during argv prevents
passthrough value reasoning even when the new destination is uniquely known.
These queries do not remove the invocation or its possible compiler rejection.

Taint sink diagnostics can inspect the known candidates from
`registry_invocation_assistance`; that does not establish an executed store,
result type, source colour or sanitiser. The retained warning adapter first
queries exact runtime/native facts, then composes each possible sink's argument
spellings with `EffectiveCommandWords`. Nested sanitiser, list-builder and
transform mitigations resolve the actual command-substitution source sites
through `nested_command_words`, `inherit_nested_bindings` and
`resolved_tokens_invocation`. Identical wrapper spellings at different sites
must each establish their implementation. Alias prefix values and braced words
remain literal values; a dollar sign in decoded argv is not a source read.

`SsaSourceView::source_symbols` is the retained operand-name inventory, not the
whole environment visible at a boundary. A loop environment can therefore be
empty at a `goto` that performs no reads. Use `reaching_binding(name, registry)`
or batched `reaching_bindings(registry)` for environment inputs: they resolve
candidate names again through the exact point context, physical cell lifetime,
presence, applicable read observers, and represented reaching store origin.
They do not manufacture lexical read sites. Missing contexts decline; a
`SsaReadReference` with `version: None` retains only the address dependency.

### Native compilation context

`SourceAnalysisOptions::native_compilation` is an independent
`NativeCompilationContext`. `mode` distinguishes file/direct evaluation,
bytecode-object evaluation, and unknown entry. `frame` distinguishes availability
of a procedure compiler's local-variable table from a script compiler; it is not
the script's runtime variable activation. `loop_depth` is lexical compilation
context, not a count of runtime stack frames. `catch_depth` retains enclosing
inline catch exception ranges; missing evidence remains unknown. Separately
compiled bodies reset the depth, while a proved inline catch increments it.
C Tcl 8.4's simple return hook requires a procedure compiler outside such ranges;
C Tcl 8.5–9.1 also compile simple return inside catches and script frames.

Stock hook availability is authored on the individual registry command specs,
against [the pinned C source registration inventory](../../../rust/tcl-registry/src/native_compilation_hooks.tsv).
`NoHook` proves an absent compile hook; `HookFrom` records release-dependent
availability, and `Unresolved` preserves unknown grammar for actual hooks.
A missing descriptor never licenses generic dispatch. Core ensembles have their
own registrations outside `builtInCmds`; an absent row does not mean no hook.
The C table does not establish availability on an independently evolved fork.

`file` retains its monolithic, uncompiled handler on C Tcl 8.4 and 8.5.
C Tcl 8.6–9.1 use the original `::tcl::file` worker map. Each worker descriptor
records its own native compiler arity and registration; `attributes`, `copy`
and `rename` have no worker compiler. The newer `home`, `tempdir` and
`tildeexpand` workers require their C Tcl 9 surface. A named invocation requires
both the actual ensemble map and the independent worker registration. Its
captured private name does not freeze the handler subsequently looked up there.

The convenience authoring driver uses `Direct` / `ScriptCode`. A bytecode
backend supplies `BytecodeObject`; a procedure compilation supplies
`ProcedureCode`. Explicit driver options remain explicit, including unknown
mode. A lexer grammar, command catalogue, or C Tcl release alone cannot establish
how a script was entered.

`NativeCompilationSpec::select` consumes original source-word shapes and the
selected dialect/context. Never pass frozen substitutions as source literals:
`$command` known to contain `set` still has a dynamic head, and an alias to a
native token does not inherit its compiler hook. Imported native tokens can
forward that capability through the shared command lookup owner. The registry
also owns each native command's body compilation boundary. An inline loop body,
a separately evaluated `catch` body, and a procedure body do not necessarily
share compilation context.

`NativeCompilationSnapshot` retains the actual command knowledge at the chunk
boundary. C Tcl 8.4's selected operations use `ChunkEntry`; later C cores use
`BeforeArguments`. Neither validates after an operand substitution has already
replaced the selected command. Unknown hook grammar or missing entry proof stays
unknown. Runtime entry and artifact admission require their own retained
compilation contract; a source witness cannot substitute for that contract.

`native_compilation_failure_at` is the source owner's must query for a rejected
chunk; its failure inventory also contains possible failures. IR and CFG retain
the must failure at the compilation boundary. Analysis admits no preceding
body effects. `codegen::native_failure` transfers exact presentation and failed
native lookup dependencies to the runtime ABI, including dependencies whose
invocation never reaches runtime dispatch. `NativeCompilationError` owns ordered
compiler command contexts and authored child-body notes; runtime supplies the
actual procedure name before formal binding. Its intrinsic error-storage writes
use the shared variable owner and active write observers.

`FunctionAsm::validate_native_compilation_entry` keeps an unresolved definite
failure in the host admission channel through `NativeCompilationPreflight`.
It cannot become ordinary late dispatch or a guessed Tcl error. Runtime hosts
resolve that obligation through a genuine compiler provider. Defining a
procedure does not admit its body: validation remains at actual body entry.

### Runtime compilation and embedding

An embedding host must pass `NativeCompilationEntry` through
`CompileService::compile_script_with_entry` or `compile_procedure_with_entry`.
The image retains the source profile, logical invocation policy and physical
compiler engine independently. Compiler-hook selection uses the physical engine;
normal handler/value/frame facts use the logical invocation policy. Missing
policy or engine evidence stays unknown rather than borrowing the other axis or
editor assistance. Original words retain their source lexer and value rules.
The image also identifies the interpreter, namespace search paths,
command allocations, imports, aliases, observers, and actual frame. A fresh
stock catalogue cannot substitute for this image after interpreter mutation.
Transparent compilation-service wrappers must forward the image along with
the script; a wrapper that changes the source or compilation contract must
establish its own compatible entry. The default trait implementation declines
optimized compilation when that evidence cannot be consumed.

Public registration of a custom implementation under `set` does not make it
the stock native implementation. Stock provenance is assigned by the private
stock-registration path. Display names and matching function signatures do
not establish that provenance.

Artifact admission checks every retained dependency. Once an activation has
entered a C Tcl 8.4 chunk, its selected `ChunkEntry` operations retain that
selection through later command-table mutation. `BeforeArguments` operations
validate at their invocation boundary. Neither guard may be moved after argv
evaluation. Selected native operations also retain their execution-trace
disposition: a trace installed by an argument substitution affects later live
dispatch, but cannot retroactively trace an already selected native opcode.

### Preserving source proof through lowering

Use `CommandTokens::restore_source_proofs` when restoring semantic evidence on
an already lowered statement. This restores dispatch, nested-dispatch, and
reached-read ownership while preserving the lowered argv and synthetic marker.
Replacing the whole token snapshot can turn an iteration-only marker into a
second evaluation of the original arguments.

Source input and reached reads are separate queries. A loop header may retain
its original input tokens without evaluating them again. The CFG represents
the initial argv evaluation at the preloop boundary; cached input reads keep
the contents version from that boundary. Later body writes must not change
which version supplied the cached list. Missing or conflicting represented
versions remain unknown.

Every inline body command needs its own source carrier and compiler context.
An outer `catch` proof does not prove an inner `return` or `set`. An exact
authored literal body can retain its source coordinates; a concatenated or
computed script belongs to its own source instance. Reusing matching text or
a neighbouring invocation's proof is invalid.

### Successful continuation facts

`ResolvedInvocation::facts()` and `argument_roles()` describe the invocation
without assuming success. `facts_after_success()` and
`argument_roles_after_success()` add only guarantees established on the normal
continuation. For optional-level frame commands,
`FrameEffectSpec::successful_layout` returns a `FrameSuccessProjection` that keeps `may_argument_error`
independent from the successful argument layout. Actual expanded argv cardinality
must be known before pair arity can select that layout.

Use the successful projection when transferring normal source state. Keep the
ordinary projection for argument evaluation, exceptional continuation, and
partial commits. Never use a successful layout to erase an error route or to
rewrite the original operands before the call succeeds.

A transformation recipe is:

1. Construct the source owner under the actual selected environment and entry
   contract. Supply trusted loaders only when the driver establishes them.
2. Query the exact invocation. Require `proved_execution_target()` and the relevant
   implementation/provider dependencies, rather than a matching written name.
3. Compose `effective_command_words` and use its `argument_spellings`,
   `argument_literal`, and `written_argument` projections. Document-procedure
   identities can form call-graph edges; `resolve_command_tokens` separately
   requires registry-backed provenance for registry semantics. Keep
   substituted and expanded words nonliteral; their spelling is not their value.
4. Read registry roles, transitions, effects, completion, and body descriptors.
   Select specialisation only when the common proof covers every dependency.
5. If facts remain unknown, retain generic dispatch with its already-evaluated
   argv. Preserve substitutions and side effects exactly once.

For an IR statement, `resolved_statement_invocation` is the mandatory shared
adapter for registry-backed semantics. Its `ResolvedStatementInvocation` joins
`facts`, effective presentation `arguments`, the `effective` word-origin
projection, and `evaluated_arguments` frozen during argument evaluation from the
same retained proof. Query it once and keep those fields together when
interpreting argument roles or effects. Synthetic analysis boundaries and
explicitly uncertain dispatch return `None`; consumers must retain their typed
effects or generic uncertainty, rather than retry a written-head registry lookup.
For a nested `LiftedCall`, pass its retained `tokens` to
`resolved_tokens_invocation`, which is the same adapter without constructing a
synthetic statement. The source carrier retains exact nested dispatch offsets;
an absent nested proof under a positioned parent becomes explicit uncertainty.
`CommandTokens::inherit_nested_bindings` is the shared transfer for a recovered
nested carrier. It uses the exact source head offset, rather than matching text,
and preserves the parent's invocation grammar through the retained point result.
Re-segmenting text at offset zero must not restore implementation provenance.
`resolve_word_exprs_with_dialect` takes retained `InvocationDialect` explicitly
when catalogue and runtime policy differ. Literal word decoding uses that
escape/list/brace grammar before registry roles and frame selectors are queried;
source spellings containing escapes must not be stamped as evaluated values.
The lower-level `effective_command_words` remains necessary for call-graph
identity of document procedures, which does not authorise builtin semantics.

Within an existing statement visitor, keep the proof and arguments together:

```rust
let Some(invocation) = resolved_statement_invocation(registry, semantic_context, statement) else {
    record_unknown_invocation(statement); // retain generic effects and argv
    return;
};
for &(relative, role) in &invocation.facts.arg_roles {
    let argument = invocation.facts.argument_offset + usize::from(relative);
    let static_value = invocation.argument_literal(argument);
    let written_argument = invocation.effective.written_argument(argument);
    // None for a prefix operand: it has a value but no editable source word.
    record_role(role, static_value, written_argument);
}
record_effects(&invocation.facts.effects);
```

The visitor's `record_*` functions are consumer operations. The shared query
supplies runtime facts, selected decoding, and the source-index mapping; callers
do not rebuild these three independently. An unknown evaluated word's `static_value` stays `None`, even when its spelling
resembles a literal. A variable word can instead have a proved frozen value;
that value does not change its original source word or editable origin.

For `set v OLD; list $v [set v NEW]`, the first argument's frozen value is `OLD`,
its presentation spelling remains `$v`, and its origin remains `Written(1)`.
The post-argv variable context can already contain `NEW`; querying that context
to reconstruct the first argument is wrong. Consumers use
`ResolvedStatementInvocation::argument_literal(0)` or `evaluated_arguments`,
which retain the source owner's left-to-right evaluation result. An alias that
prepends `PREFIX` moves this operand to effective argument 1:
`argument_literal(0)` returns `PREFIX`, `written_argument(0)` returns `None`, and
`written_argument(1)` maps the original `$v` to written argument 0. Prefix values
are already evaluated values and must never undergo source substitution again.

Frozen values do not establish the identities of earlier reads. For example,
after `set x OLD; set y NEW; upvar 0 x a`, evaluating
`list $a [upvar 0 y a]` reads `x` before the second argument retargets `a` to
`y`. In `list "$a[upvar 0 y a]$a"`, two references within the same word read
different cells. The invocation's post-argument `variable_context` cannot
represent either sequence by itself.

The source owner records each reached reference as `SourceVariableAccess`,
after array-index evaluation and before that reference's read observers.
`CommandTokens::variable_access_at(source_site, original_spelling)` is the exact
consumer query; its result retains the read's `ResolveContext`. Repeated visits
to the same reference join their contexts. Missing, skipped, synthetic, or
unpositioned references stay unresolved. Do not substitute the command's later
context, a neighbouring reference, or another matching spelling for a missing
record. Use the variable owner's place projection with the returned context,
then the SSA owner's version projection at that evaluation point. Retaining a
cell identity does not by itself establish the cell's contents or SSA version.

`SsaSourceView::read_reference`, `read_word`, and `read_expression_variable`
return `SsaReadReference` with a
physical `symbol` and a separate optional `version`. For
`"$x[set x NEW]$x"`, both references can identify the same cell while the
second reference has no represented CFG definition for its nested store.
In that case its version is `None`; using the earlier version would invent
a contents proof. For a spelling that denotes different cells within one
statement, the statement-wide spelling lookup must abstain. Consumers that
need a value query the reference, rather than guessing from a spelling map.
Consumers first retain each part's source site and use `read_reference`,
`read_word`, or `read_expression_variable`; earlier and later reads remain
independent even when their source spelling agrees. Native consumers must not
discard those sites to obtain a name-only query. Legacy name-only value
consumers use `SsaSourceView::read_spelling` only as a consensus projection: at least one reached read must exist, and all captured reads of that
spelling must agree on the canonical symbol and contents version. A conflict,
missing native grammar or missing represented version declines the value proof.
`is_positioned` distinguishes this strict path from compatibility queries over
a whole function; absence at an actual point never falls back to statement uses.

For expression nodes, pass `read_expression_variable` the retained original
`expr_base`; a reconstructed expression or absent base cannot license a value.
The variable owner retains `ContentsOrigin::{Incoming, WrittenAt, Alternatives,
Unknown}` separately from cell identity. SSA accepts a version only when that
origin agrees with a represented definition or phi. `PointResolveContexts::source_tokens_at`
preserves the original typed carrier for these queries.
`read_contents_at`, `read_word_contents`, and
`read_expression_variable_contents` project bounded reaching writes at the same
exact reference when one scalar version cannot represent all alternatives.
`SsaReadContents::unknown_residual` explicitly retains unrepresented effects;
a consumer joins the represented producers for its own analysis purpose and
must not interpret an unknown residual as clean or constant.

SCCP's positioned path keeps an earlier contents version immutable when a
later operation writes or destroys a dynamic name. That later read needs its
own represented reaching store; `version: None` declines the value. Dynamic
name effects do not imply a variable trace. Store folding consults the actual
write-observed places from `def_places_with_continuation`, and a future trace
or an unreachable unknown command cannot invalidate an earlier branch proof.
Carrierless compatibility SSA retains its conservative exposure summary.

Existence diagnostics and rewrites share
`existence_constant_branches_with_ssa`. It resolves the nested invocation at the
retained expression base, composes alias arguments, then queries
`ContentsPresence` in that invocation's selected variable context. Unknown
presence or absent dispatch evidence declines; no whole-function spelling scan
repairs a missing positioned proof. Allocated cells alone do not prove existence,
and defined contents alone do not prove that a root is an array.

Runtime expression consumers preserve the actual source `Value` and call
`prepare_expression_value`. `cmd_expr`, the stack expression opcode and resumable
control conditions share this door. It materialises the original source bytes
under the selected lazy double policy before replacing the source object's
internal representation, including on a parse error. Shared aliases therefore
observe the representation change. AST reuse requires full
`DialectProfileKey` and `InvocationDialect` equality; integer and floating
source objects do not bypass Tcl expression source parsing. Expression operands
are converted only when their lazy evaluation actually reaches them.

Persistent callable storage lives in `RawBindingArena`, keyed by typed command
allocation, incarnation and implementation generation. `RawBindingSlotId`
retains the physical cell independently of the ordinary name table. Static
bindings install before formal activation; deleting a name detaches its slot
while a capture can retain the old cell. Captured Jim alias wrappers select an
absolute logical level and name again on each access, including a later frame
activation at the same level. Hashing, joins and relocation retain this arena;
consumers must not replace it with a variable-name map or frame lifetime guess.

Nested token recovery transfers this carrier through
`inherit_nested_bindings`. Cache relocation rebases reference sites and
`restore_script_bindings` obtains fresh contexts from the actual source
instance. Changing source words invalidates their references; copying the old
carrier after a rewrite cannot establish new evaluation history.

Typed statements that consume their invocation words retain the same carrier
on `CommandBindingSite::source_tokens`. This keeps expression and branch reads
available without adding large recursive statement variants. The point owner
exposes the retained words and reads alongside its before/after contexts.
Synthetic CFG bindings use `SyntheticMarker::IterationBindings`; they establish
iteration-variable stores and do not assert that a readable command label is
a live invocation. Source-changing transforms must invalidate or reconstruct
their retained source carriers and contents provenance together.

Read inventories are temporal projections. Use
`SourceCommandBindings::variable_accesses_for_invocation_args` for reads
directly owned by an invocation's original word evaluation, or
`variable_accesses_during_argument_evaluation` for its reached substitution
subtree. `variable_accesses_during_expression_invocation` additionally retains
the selected native expression and substitutions it enters. Production token
carriers use this last projection so typed expression consumers share the
same proof. Consumers still select references from their original `WordExpr`
or expression node; the inventory itself is not an unordered list of effects.
The structured word producer preserves the original spelling of each reference
as well as its source site. Compatibility argv canonicalisation cannot rewrite
that spelling: a receipt for `$x` does not authenticate a structured `${x}` at
the same offset. Exact read selection requires a unique matching source site
and spelling; duplicates, changed words and absent originals remain unresolved.

An executed script body belongs to its own phase. For example, in
`puts [catch {puts $x}]`, the inner read belongs to the outer `puts` argument
evaluation subtree, but does not belong to `catch`'s argument evaluation.
Lexical containment cannot establish that relationship.
`variable_accesses_in_span` is a source inventory query, not an execution-phase
proof. After relocating cached carriers, requery their temporal owners from
the new source instance; shifting a span cannot transfer allocation identity.

Analysis termination and executable continuation have different obligations.
An analysis may classify an exact proved native `error` as having no normal
continuation. Executable bytecode can subsequently replay that source command
after command-table mutation; its replacement can return normally. Preserve
the executable continuation and its result rather than appending a synthetic
Tcl `return` to encode an analysis fact. Completion descriptors and native
procedure/capture boundaries own the actual return code and unwind level.

Declaration assistance uses a separate visitor branch:

```rust
if let Some(assistance) = resolved_declared_assistance(&surface, tokens) {
    for (written_argument, role) in assistance.roles {
        offer_editor_role(written_argument, role, assistance.declaration);
    }
}
// This result supplies no effects, stores, purity or dispatch proof.
```

Possible runtime candidates and catalogue metadata also have separate result
types. `registry_invocation_assistance` returns candidate shapes with
`unknown_residual` and `may_be_absent`; retain both flags when displaying possible
roles. A candidate's `possible_traits` or `nominal_return_type` is not a Must
property of the invocation. `catalogue_invocation_assistance` applies only to an
unchanged logical slot under the document surface context; it can describe a
provider that is not loaded. Neither API licenses effects, writes, purity,
result-type guarantees or optimisation. A document definition or slot mutation
must suppress unrelated nominal metadata rather than retrying the written name.

Body-cache reuse also consumes the source owner. Admission checks typed mutation,
deferred-body and entry dependencies before reusing an isolated body; legacy
text/shape helpers cannot prove equivalence. After cloning and rebasing the IR,
`SourceCommandBindings::restore_script_bindings` restores original exact source
sites, activation/frame context and nested dispatch proofs. Its `false` result
discards the cached body and lowers in the actual entry. `for_each_script_mut`
and `Statement::tokens_mut` provide the shared traversal/carrier mutation; a
consumer cannot repair cached provenance by changing offsets alone. Retain the
entry, selected grammar and provider/dependency facts in cache identity.

`realm::document_realm_bindings_with_config` delegates to this owner. Editor
adapters consume `binding_at`, `head_words`, or unpositioned projections according
to their source-position needs. Do not feed those assistance projections back
into executable identity or bypass explicit entry assumptions.

## Variable cells and activation recipes

`ResolveContext::for_function` selects a procedure activation;
`ResolveContext::for_namespace` selects namespace storage. Event and method
adapters must pass their actual activation through
`build_point_resolve_contexts_with_entry`. A qualified procedure name does not,
by itself, establish a runtime event frame or an instance namespace.

An actual invocation enters a caller-linked context with
`enter_called_frame`; `selected_frame_context` interprets a proved numeric
selector against that retained caller chain. A lexical procedure inventory with
no call site cannot invent its caller depth. `restore_execution_frame` projects
child effects back into the real parent after execution.

Alias creation selects its destination grammar separately from ordinary reads.
Use `VariableAliasDestination::is_active_in_frame_with_policy` with the actual
`VariableAliasFrame`, retaining an unknown result. For a `ProcedureLocal`
destination, C Tcl `global` is inactive in namespace/global activations; Jim
activates it in namespace evaluation while retaining the root no-op. A
`CurrentNamespaceOrLocal` destination has its own contract. Once active,
`resolve_alias_destination_slot` creates/selects the destination without a
same-named global read fallback. `resolve_target_access` retains whether the
name operand was braced, so literal dollar signs and element indices do not
acquire fabricated name-formation reads; dynamic names still retain those reads.

The result provides `before_statement`, `after_statement`, and
`before_terminator`. Query the context at the access's CFG site. The solver joins
branches and exception edges; it does not interpret CFG construction order as
execution order. `after_statement` describes successful continuation. A joined
whole-function projection from `place_bridge::build_resolve_context` is a
compatibility approximation, not a substitute for point queries.

`resolve_place` carries canonical storage identity alongside the access shape.
`CellIdentity` identifies owner and generation; arrays and dict paths retain
their selectors. `cell_key` can project this identity for consumers that require
a key. Read/def footprints and `overlap` determine interference. A name-string
comparison cannot replace them: globals, namespace variables, `upvar`, element
links, unset/recreation, and retargeting change identity independently of spelling.

For an access, use `resolve_literal_access` for an evaluated name operand and
`resolve_substitution_access` for an original `$name` or `${name}` source spelling.
The latter retains whether an array index came from a braced reference and uses
`ResolveContext.invocation_dialect.lexer_grammar` directly. A display spelling
cannot reconstruct that provenance. Both APIs call `project_access` with the
actual `TraceOperation`; read, write and unset callbacks have distinct effects.
`resolve_access` remains the generic spelling adapter where the caller already
knows the access shape. Keep the point context when projecting trace observation.

SSA consumers use `SsaFunction::cell_name`/`cell_names` for canonical binding-value
keys, together with the SSA version. `var_name`/`var_names` are display spellings
for diagnostics. Different captured cells can both display as `x` with version 1; keying a
proof map by that pair merges unrelated contents. Scalar binding-value keys
track the current slot through unset/recreation with distinct SSA versions;
physical cell identity remains generation-sensitive in the place/memory owner.
`canonical_binding_value_name` owns that distinction, while captured element
links retain the lifetime that can dangle. Resolve a written name at the actual access
with `var_symbol_at` or `var_symbol_at_terminator`; a uniform `var_symbol` may
abstain when point identities differ. `DefUseResult::chain_for` supports a source
spelling only when its compatibility index establishes a unique storage chain;
ambiguity declines. Optimisation and dependency maps retain canonical value keys.

`SsaSourceView::at_statement` and `at_terminator` adapt source operands to those
SSA symbols at the exact operation. Use `read_reference`, `read_word`, or
`read_expression_variable` with the retained source extent. A returned cell with
`version: None` is a dependency, but supplies no constant: nested evaluation may
have written contents without a represented CFG store. For example:

```rust
let lookup = SsaSourceView::at_statement(ssa, block, statement_index);
let read = lookup.read_reference(original_site, original_spelling)?;
let version = read.version?;
bind_operand(original_spelling, (read.symbol, version));
```

`bind_operand` consumes SSA value keys; the spelling belongs only to this exact
reference's expression environment. Two references in one word can resolve to
different cells or versions. A name-only environment must first use
`read_spelling` to prove consensus among all reached reads with that spelling;
conflicting or unrepresented reads decline. `symbol` and `source_symbols` are
binding projections, not contents proofs. `unpositioned` is a compatibility query
over unique bindings; a missing positioned mapping stays absent. Never build an
expression environment from display `var_name` strings. Explorer graph edges export `cell`,
`fromCell` and `toCell` as canonical value identities alongside readable names.
Consumers use those identity fields to associate dependencies, even when several
nodes display the same name.

Binding does not imply initialisation. `global` can link an absent cell;
`upvar` can retarget an alias. Traces observe accesses and can re-enter Tcl;
`observed`, unknown bindings, generations, and widening must survive the bridge.
A dynamic name also reads the values used to form that name. Use
`places_read_to_form` and the bridge rather than dropping those reads.

Build the point owner once, then distinguish the access from its successful
continuation:

```rust
let points = build_point_resolve_contexts_with_entry(cfg, actual_entry, registry);
let tokens = points.source_tokens_at(block, statement_index)?;
let access = SourceVariableAccess::find_at_source(
    &tokens.variable_accesses, lexical_site, "$array($key)",
)?;
let read = resolve_substitution_access(
    &access.original_spelling, &access.variable_context, registry, TraceOperation::Read,
);
let after = points.after_statement(block, statement_index);
let next_write = resolve_literal_access("array(next)", after, false, registry, TraceOperation::Write);
record_interference(overlap(&read, &next_write));
```

`actual_entry` carries the selected activation and invocation grammar. Abrupt
edges use the CFG solver's corresponding successor context; they cannot reuse
`after_statement` as though the statement succeeded. A lexical read uses its
retained reference context, including preceding index substitutions; a command's
`before_statement` or post-argv context cannot replace that context. A consumer that needs the
reads forming `$key` must also retain the bridge's name-formation footprint.

Current consumers include scalar SSA, `cell_state_ssa`, dead-store analysis,
connection-scope analysis, and `FunctionUnit` context queries. Their local
algorithms differ; their answers about cell identity must come from this owner.

## Completion and procedure boundaries

Use `CommandRegistry::invocation_completion_route` with the proved invocation's
retained `InvocationArguments` and surface context to obtain an
`InvocationCompletionRoute`. This preserves a pending return's eventual code
and remaining level; a catch-visible numeric code alone loses those facts.
`normal_possible()` gates the next statement, while `abrupt_possible()` retains
other continuations. An unknown route includes both and must not be reduced to
a successful call. Arguments and their substitutions execute before this route
is applied.

Only a procedure return boundary calls
`through_procedure_boundary_in(retained_dialect)`. Namespace evaluation and
ordinary script evaluation keep the route intact even when they allocate a
frame. The selected policy also distinguishes C rejection of raw break/continue
escaping a procedure from Jim propagation. A configured return that releases a
code follows its own pending unwind semantics. Source interpretation, CFG
routing and executable region adapters must consume this same route owner;
scanning a written `return` word cannot establish which statements are dead.

## Evaluated scripts and exact-once execution

`BodyExecutionSpec` lives in the registry and parses argument roles and wrapper
selection. `CapturedLifecycleSpec` selects setup/body/cleanup operands and
versioned hook policy. `EvaluatedBodyRegion` retains ordered phases, normal and
abrupt completion routes, selection, source provenance, and explicit
`ExecutionRegionDependency` prerequisites. `valid()` checks structural validity;
it does not prove those prerequisites. The lowering ingress must prove them
before attaching a region to command tokens.

The canonical executable graph evaluates invocation words and builds argv
before entering phases. `RegionChoice` represents possible skipping.
`CompleteEvaluatedRegion` completes the residual wrapper after represented
phases; its retained `GenericInvoke` is metadata, not another full invocation.
Legacy CFG projections distinguish argument evaluation from residual wrapper
work. `WrapperEffectProjection::resolve()` supplies residual effects without
reapplying the complete original invocation footprint. An expansion cannot count wrapper argument substitutions again or treat a
captured `return`, `break`, or `continue` as an enclosing procedure exit.

`ExecutableAnalysisAvailability::evaluated_regions()` exposes these boundaries
separately from `invocations()`. Residual completion/effect inputs remain
conservative unless a dedicated residual contract proves more. Do not reuse the
full original invocation's effects as if its bodies execute again at the join.

Mixed-region planning, native lowering, and the bounded WASM generic-invoke
planner decline a function containing evaluated-region boundaries before
emitting any operations. Selection, completion capture, residual wrapper work
and already-built argv form one protocol; replacing just the completion boundary
with an original-source fallback executes the scripts twice.

`GenericInvoke::registry_specialisation_arguments_exact()` is a separate
argument-correspondence gate. Native and guarded intrinsic selection use it
before reading registry argument positions: an alias's inserted prefix belongs
to effective semantics while runtime generic dispatch still consumes the
original argv. A backend that has not composed the prefix cannot specialise
those operands. The bounded WASM generic-invoke plan retains generic dispatch.

## Dialect policies and unsupported capabilities

Select `DialectProfile` through the environment ingress, then thread the same
profile through lexing, registry queries, and resolution. `profile.cache_key()`
freezes the entire selected snapshot, including its execution point and build,
number and lexer policies, package availability, and library pins. Use this key
for a profile cache; a name such as `jim` can denote both 0.80 and 0.84.
The exhaustive snapshot projection makes adding a profile field require an
explicit cache identity decision at compile time. A name
round-trip must not replace an explicit point with the environment's default.
`context_for_profile` and its static counterparts preserve the selected point
and stamp the command view with its exact snapshot; `profile.intern()` supplies
a shared static snapshot without changing catalogue identity.
`profile.cache_key().profile()` restores that exact snapshot, without a name
lookup.

`CommandRegistry::snapshot()` retains an immutable registry for incremental
queries. Repeated reads share its owned view; changing specs, layer order,
ambient pins, profile, or document grammar invalidates the current snapshot.
Existing readers keep their original surface. Equality compares the complete
structural key, including immutable spec identities; a cached hash is only an
index. Specs and grammars remain process-lifetime immutable registrations.
The LSP database's body and lattice memos retain these actual registry/profile
snapshots and the normalised lexer configuration. Taint and optimiser memos
read the same retained registry, and optimiser reconstruction keeps the
module's original source entry, including selected loader provenance. Do not
replace these inputs with `db.registry(profile.name)` or infer a grammar from
a display label.

Bytecode compilation preserves the original supplied profile handle on the
artifact. VM pointer validation remains strict, even when two separately
allocated profiles describe identical snapshots. Sharing equivalent registry
values does not authorise substituting the artifact's profile identity.

Runtime-family policies are separate from C release grammar:

- `variable_lookup_policy()` distinguishes Tcl and Jim storage rules.
- `package_protocol()` distinguishes Tcl discovery/version negotiation from
  Jim's package protocol.
- `namespace_import_binding()` distinguishes C command-token imports from Jim
  source-name imports.
- `variable_link_binding()` distinguishes C links retaining a cell from Jim
  links retaining a selected frame/name. Unset/recreation, missing arrays and
  alias cycles use that policy in both live execution and point binding analysis;
  consumers cannot select it from the command catalogue's apparent Tcl version.
- `InvocationDialect::catch_positional_arity()` selects the C positional envelope from native completion grammar, separately from host command catalogue arity; Jim completion switches remain with its selector.
- `upvar_level_presence()` and `uplevel_level_presence()` govern whether a word
  is a level selector. Numeral parsing still uses `NumberSyntax` separately.
- `TclVersion` version/requirement helpers with `_for` suffix retain the selected
  release's package grammar. Unknown family/profile is not latest C Tcl.

`InvocationDialect::arithmetic()` selects the integer tower independently of
`NumberSyntax`: Tcl 8.4 wraps wide integers, Tcl 8.5 and later promote to
bignums, and current Jim wraps wide integers with different shift-count and
literal-overflow rules. `tcl_syntax::expr::wide` owns the fixed-width numeric
operations and conversions; `number_tower` owns arbitrary precision. Runtime
adapters retain their value and error surfaces while constant folding maps
unprovable operations to a declined fold. Native undefined operations are
explicit errors in the shared contract, never values inferred from an
accidental machine instruction or a crashed oracle process.

Pass `FoldPolicy::with_invocation_dialect` the actual invocation snapshot.
The catalogue profile alone cannot establish the engine's numeral grammar,
integer tower, or fixed math-function roster. Preserve selected string operands
where the engine returns them unchanged: Jim's `expr {"001"}` returns `001`,
while the bare numeric literal `expr {001}` returns `1`. A numeric-only folding
result cannot stand in for the former without changing its spelling.

Policy accessors can return `None` when the environment has no corresponding
runtime. Keep that absence visible. Catalogue availability, language syntax,
ambient packages, and provided package implementations are independent facts.

F5 declaration admission is also separate from procedure runtime dispatch.
Load-time literal-head validation and registry-owned placement rules cannot be
replaced by runtime alias resolution. Runtime procedures still need their
activation/binding entry; event bodies need actual event roots. Query the
`tcl_irules` execution-context adapters rather than inventing a command-name or
namespace prefix exception.

## Consumer purposes and uncertainty

| Consumer family | Shared surface | Required regression when extended |
| --- | --- | --- |
| Analyser and lowering | Source command owner, registry descriptors, point cell contexts | Conditional rebinding, dynamic words, missing exact sites, error/abrupt paths |
| Optimiser, interprocedural analysis, taint | Terminal identity and alias prefix; point cell footprints | Same spelling/different implementation, deferred mutation, unknown residual, trace re-entry |
| LSP hover, signature help, references, semantic tokens, formatting/minify, folding, selection, caller-frame views | Document realm projections and authored body/argument descriptors | Positioned identity, renamed/imported/aliased commands, substituted words, selected dialect |
| Executable common analysis, native and WASM selection | `SemanticAnalysisBundle`, dependency proof, prebuilt argv, canonical region graph | Missing dependency declines before emission; substitutions execute once; no wrapper replay |
| iRules declarations/events | Registry placement and `tcl_irules` contexts | Load-time shape versus runtime aliases; reachable event/procedure activation |
| VM and runtime live dispatch | Naming/identity primitives and actual live tables | Shared lookup vectors and runtime mutation/trace/alias lifecycle |

Each consumer requires its own purpose-specific projection. A command-owner
result does not by itself identify an editable source span, establish a runtime
entry, close operand effects, or validate a cached artifact. Missing carriers
retain typed uncertainty. Public query documentation specifies input provenance,
normal versus abrupt continuation, and whether a result can authorise a
transformation.

A useful regression starts with a positive case and changes one independent
axis: file versus compiled-object entry, alias versus import, before versus
during argv mutation, allocated versus defined cell, normal versus exceptional
continuation, worker identity, or native dialect. Assert both the shared owner
result and the observable consumer behavior. Native comparisons must record the
entry protocol, exact release/revision, completion code, result, and state changes.
An owner-only test does not establish the observable consumer behaviour.

Do not resolve a failed test by removing its semantic assertion, widening a
budget, assigning a missing dialect to C Tcl, or inserting a written-name
fallback. First inspect the retained proof at the owner and each adapter. Add a
missing capability to the lowest owner that can express it, then run the
positive case and its mutation/unknown-entry controls together.

Expression-only and reconstructed-text scans require exact source carriers
before they can consume runtime identity. Missing carriers remain unknown; a
written-name lookup cannot replace the missing proof.

Effective presentation arguments retain `Option<String>` slots. `None` means
an actual captured value has no materialised bytes; it is never an empty Tcl
literal. Use `argument_word` or `with_argument_words` for semantic registry
queries. `callback_taint_inputs_words` can select a known deferred positional
callback even when an earlier captured value is unknown. Unknown option layout
and argv expansion abstain. Legacy string-only sanitizer/transform queries
cannot license mitigation for missing argument presentations; callgraph identity
continues to use the proved target independently of argument bytes.

Runtime expression preparation uses `ExprParseContext` with the retained source
lexer, operator base and host grammar, and independently selected native syntax.
`CheckedExprParse::Unsupported` is an execution-provider obligation, not a Tcl
syntax-error result. A proved syntax rejection whose native presenter abstains
has the same neutral boundary requirement. VM embedders use `try_eval_source`,
`try_eval_expr`, `try_invoke_command`, `try_run_module` or `try_run_function` to
receive `NativeExecutionError`; artifact admission errors are wrapped in its
`CompilationAdmission` variant. Existing convenience APIs invariant-refuse at
their outer host boundary when faithful execution is unavailable.

Public VM object-vector invocation also owns the selected native error-log
frontier. On a C Tcl error it reaches the original argument objects' string
updaters before constructing the command excerpt. Tcl 8.4 uses NUL-terminated
DString elements, Tcl 8.5 uses a counted List even when the error is already
logged, and Tcl 8.6–9.1 skip that construction for an already-logged error.
Jim object-vector invocation does not borrow this C logging protocol. Internal
command relays and bytecode dispatch use their own entry protocol; consumers
must not add this observation to every dispatch or render copied arguments.

The reached-expression refusal retains exact resolved source bytes, full source
and native profile keys, and actual interpreter/frame/namespace identity. It is
diagnostic context, not an executable continuation. The refusal is retained on
the engine, so child interpreter switching cannot lose it. The drive loop prioritises it before catch/try
settlement, result/options stores, error-global publication and leave/unset
callbacks. Effects before the unsupported expression remain applied; neither
guest handlers nor later instructions execute. Fresh VM tests prove those
boundaries and a shared expression object's cache miss when a same-name profile
changes only its operator base. `CompileError::NativeCompilationAdmission` preserves the same host channel at
all dynamic script/procedure compilation entries. `CompileError::Message` stays
an ordinary source diagnostic; identical display bytes do not establish an
admission obligation. Compiler adapters must propagate the typed variant rather
than converting it to a string that a guest catch could swallow.
Operational service capability failures use `CompileError::Unsupported` and
`NativeExecutionError::CompileServiceRefusal`. Its immutable payload retains the
requested source, compilation namespace and script/procedure scope, separately
from the actual interpreter/frame/namespace and full native/source profiles.
Host compilation uses `try_compile_function` or `try_define_procedure` and
`VmCompilationError::{Tcl,Host}`; the engine adapter preserves the host diagnostic
rather than publishing the internal empty unwind carrier as a Tcl error.
Reusable handles use `try_invoke_function` to retain the same boundary when
profile, namespace or live binding changes require recompilation.

Reached host commands use `Vm::refuse_host_command` and the same retained
neutral execution channel. `EngineError::ExecutionRefusal` remains separate from
Tcl errors in the engine adapter; guest `catch` cannot consume it, and accepted
operations before the refusal remain applied. Registration and reusable handles
use the typed `try_*` entry points, so an internal unwind never escapes as an
empty Tcl result.

Jim expression preparation calls `prepare_fixed_function_expression` before
conventional AST parsing, including warm source-object reads. It resolves the
actual installed fixed-function table, validates all lexical function names
before substitutions and executes the returned native term-stack tree. A
nominal roster or a cached conventional AST cannot replace this preparation.

Math folding consumes `ExpressionMathBindings::resolved_call` at each reached
AST call coordinate. This retains the actual native implementation and separately
requires the captured operand object's callback effects to be closed.
`resolved_call_for_value_analysis`, `proved_invocation` and `proves_intrinsic`
retain implementation identity only; they cannot license erasure.
Boolean adapters which erase the unchanged native call use
`proves_intrinsic_for_erasure`. `native_fold_dependency` enforces the same effect
gate when committing an artifact dependency. Unknown input classes, intervening
observers and conflicting observations decline erasure independently of known
function identity, arity or mathematical value. The source owner's actual
`SourceImplicitMathInvocation::object_callback_effects_closed` inventory is the
only closure projection; registry availability supplies none. Prefix
objects require independently proved reached coercion effects for folding; frozen bytes alone do not establish that changing a shared list or double
representation is unobservable. Failed or lazy skipped calls consume no reached
call dependency. Expression-entry preparation is a separate obligation: Jim
and C 8.4 can reject an unknown function even in a lazy skipped branch.
Successful SCCP folds retain their exact invocation in
`SccpResult::required_math_invocations`; a literal replacement must carry those
requirements into backend admission even after removing the original call AST.
`ExpressionMathBindings::with_preparations` attaches the actual source inventory.
`preparation_for_context` selects a whole-expression witness under the complete
`FoldPolicy::preparation_context`; a conventional subtree cannot consume the
whole prepared tree. A successful whole-expression fold uses the witness tree
and records `SccpResult::required_expression_preparations`, even when lazy
execution reaches no function calls. Successful static-loop summaries merge
both ledgers. CFG and backend admission retain both requirements after literal
replacement. Conflicting fixed interpreter/table prerequisites decline the
local fold atomically across preparation and reached-call ledgers.
Analysis-only `FoldEvaluation` also records reached coercion obligations and
native result dependencies. A known numerical value with those obligations
cannot license removing its producer or returning different object bytes;
`SccpResult::values` is the separate execution-safe compatibility projection.
The numeric dispatcher separately selects `NativeMathProtocol`; Jim integer
power, overflow, conversions and NaN results cannot inherit C numeric semantics.

These guarantees do not establish a resumable external-provider continuation;
a host must not replay prior effects.

`execution_conformance::vectors`
covers command binding, package lookup, and autoload with independent C and Jim
expectations; `rewrite_cases` supplies standalone scripts where whole-file C
compilation or file error semantics matter. `filesystem_cases` shares package
loader files with C `pkgIndex.tcl` and Jim plain-file discovery expectations;
C `package names` can retain loader advertisements after a failed provision,
so presence in that list does not establish a provided package;
`run_script_fixture` creates isolated file roots for both oracle and rewrite
consumers. Do not turn an unsupported feature
into a skipped semantic assertion: the Jim feature probes and vector runner
verify the expected absence by actually invoking it.

`scripts/dev/run-resolution-oracles.sh` requires every C 8.4–9.1 interpreter and the
pinned current Jim. `required_tclshs(TclVersion::ALL)` and `require_jimsh()` fail
explicitly on missing, malformed, or stale requested interpreters. Optional local
interpreter selections remain visibly optional; the complete CI workflow is mandatory. The
provisioning manifest owns the Jim revision and revision-bearing patchlevel;
capabilities are measured by operations, not inferred from a version number.
Jim's C-style `interp create` and `interp alias` capabilities are absent;
`interp-handle-create` and `interp-handle-alias` separately probe its returned
child-command API. The child fixtures exercise namespace/package isolation and
parent-command rebinding through retained cross-interpreter alias prefixes.
They also measure the alias target's namespace and variable frame while the
parent activation is suspended; all six references see that active parent frame.

Rewrites compare original and optimised file execution for exit status, stdout,
and primary error text. Those checks do not prove identical stack traces or
side-channel state not observed by the fixture. Add observable counters, cells,
command identities, loader effects, and error options when they are part of the
new contract. A positive output case alone cannot prove that callback absence,
exact-once evaluation, or implementation identity was established soundly.

`RegistrySnapshot::semantic_key` supplies immutable structural identity without
retaining the reader's memo caches. Name-resolution states and allocation keys
retain this `RegistrySemanticKey`; consumers that execute registry queries keep
the full snapshot reader separately. Both compare complete structural fields,
and matching fingerprints never establish equality.

A function-lattice cache may reuse a body after its authored prefix shifts only
through `prepare_native_body_template`. Supply the actual whole source, exact
body bytes, declaration normalisation offset, and executable-body offset in
`BodyProofScope`. The source owner admits a closed native body and retains its
external variable, observer, profile, and registry dependencies; a broad unknown
or document call keeps the complete source snapshot. Full structural equality
remains mandatory, with a fingerprint used only to select a hash bucket.

On a cache hit, apply the template's inverse `VariableProofRelocation` with
`FunctionUnit::relocated_variable_proofs`, then restore actual original carriers
with `restored_source_proofs(BodySourceProofs::from_body(actual_body))`. Either
failure discards the candidate and builds against the complete original key.
Lexical span rebasing and the ordinary actual-source semantic-analysis ingress
follow restoration. A physically relocated artifact deliberately marks its old
executable/world facts unavailable until that ingress rebuilds them. Symbol and
version numbers remain stable; physical cell, activation, store, trace and exact
read provenance must all agree with the current source before publication.

Native numeric values retain their selected engine policy until their first string representation is generated. `InvocationDialect::double_string_policy` and `number::format_double_selected` own formatting; a VM numeric producer uses `Value::native_double` or `with_native_double_format`. C8.x doubles observe the thread-shared precision at first materialisation, including a later child-interpreter write. Cached strings remain stable. Numeric consumers check `double_representation` before requesting a string, so arithmetic does not round an existing double through its display representation. C9 and Jim select immutable formatting policies.

`SpecialVariableHook::DoublePrecision.name_in(dialect)` identifies the native precision observer. Attach it to the resolved global storage cell, rather than to every same-spelled local. The observer runs after user traces on the whole-variable group and before element traces. Tcl8.x element aliases suppress the whole-variable group. Reads may canonicalise or recreate the stored precision value; a raw slot write is not a proof of the value a later read produces. Extension tests must cover first versus repeated string materialisation, child sharing, invalid and safe writes, unset/read recreation, array group ordering, and local-name isolation.

`Function::source_tokens_at` borrows only original evaluated-word carriers.
`source_input_tokens_at` additionally exposes typed cached iteration inputs.
The latter does not license a native invocation or reevaluation. SSA resolves
those inputs against the original reached reference and captured cell context;
it retains the pre-loop version even if the body changes the source variable.
Missing original-reference evidence leaves the value unresolved.

Binary field consumers delegate to `binary::format_values` through `FormatValueOps` and to `binary::scan_values` through `ScanValue`. The shared grammar selects byte, integer, double and counted-list conversion; an adapter must not stringify all arguments first. Double reads use retained numeric representations and double scan results use the selected lazy value constructor. Fixed tests compare both packed/unpacked values and the original operand's later string, so an eager conversion cannot pass merely by returning a plausible rounded result.

Jim static declarations capture a raw mutable variable wrapper. If that wrapper is a name link, its target follows the selected logical frame level on each access. A later activation at that level can therefore become its target. An ordinary captured scalar or array retains its physical cell instead. These are separate binding policies: do not replace Jim's logical-level link with a stable retired-frame table or copy its target's current value. The permanent native fixture distinguishes an initial missing read, a later unrelated activation, and a write followed by a separate read activation.


The purpose-aware SCCP owner uses one phi/store/read/reachability solver for
execution erasure and semantic analysis. `semantic_value_facts(cfg, ssa,
ValueFactInputs)` returns an immutable `SemanticValueFacts` view. Its `contents`
query reports actual known stored contents; its `expression` query reports the
numeric interpretation and producer obligations at an exact
`ExpressionEvaluationPoint`; `reaches` reports semantic reachability. These
queries do not grant permission to replace or remove execution. A consumer must
keep the original producer when its reached conversions or result-object
obligations remain unproved, even if later numeric uses are known. The existing
`SccpResult.values` remains the execution-safe compatibility projection.

`loop_analysis(point)` retains the same bounded simulator's conditional-on-normal
contents, ordered expression conversions and original increment statements.
`StaticLoopAnalysis` is a separate result from `StaticLoopSummary`; it cannot be
converted into a licence to erase the loop. Prepared expressions and reached
math dependencies remain attached to the semantic result. A following numeric
diagnostic may use these contents while the original loop and its possible
errors remain in execution.

Use the actual function's parameter seeds, retained registry, invocation policy,
trace inventory and substitution contracts in `ValueFactInputs`. Do not build a
second solver from display names or copy semantic constants into the legacy
map. Nested expression RHSs are classified before a source-frozen argument
literal, so frozen value bytes cannot bypass conversion or expression-entry
obligations. Jim selected-object expression results preserve existing raw bytes
such as `003` separately from numeric interpretation `3`; unknown raw bytes do
not become a canonical numeric result string. Both purposes execute the same
worklist and exact per-reference SSA queries, rather than maintaining parallel
mutable value maps.

Computed expression arguments can have a materialised source instance. For
`set a 3; set b 4; set r [expr "$a + $b"]`, the retained preparation owns
`3 + 4` at a derived parser base. `ExpressionMathBindings::at_invocation`
selects it by the original invocation's exact source identity and offset and
declines conflicting alternatives. The semantic value consumer evaluates that
prepared tree under its full native and lexical context. It does not borrow
offsets from the quoted word or treat the computed argument as authored source;
the original argv evaluation remains outside the execution-erasure projection.


`FunctionUnit::semantic_values` retains the exact construction input snapshot and
computes the semantic-purpose fixed point lazily. Both solver purposes use
`FoldPolicy::for_retained_entry`, retaining the actual invocation dialect and
lexer overlay independently of the editing catalogue. The invocation carries
its explicit character model, including embedding policies that differ from
the C base release. `SemanticValueProjection`
compares complete owned inputs; its detached `OnceLock` population does not
change semantic equality. Relocation, restored source proofs and lexical
rebasing invalidate the lazy view before another query. Diagnostic consumers
use the internal `DiagnosticValueFacts` purpose view; its known branches and
contents keep original producers and are not optimiser erasure permission.

Grouped stores have a separate proof boundary in `optimiser::store_packing`.
`Script::statement_result_use` ignores synthetic phase boundaries and identifies
a final command's enclosing normal result. O119 resolves prospective handlers
through the original immutable source lookup and selects engine/version policy
from the retained invocation dialect. Final three `set` commands return `3` on
all six native interpreters, while packed foreach/lassign return empty; C8.4 also
has different array-write failure text. Result discard alone does not close
ordered output address, observer, native failure or object-sharing equivalence.
The grouped proposal and paired removals are advisory because they lack a
proved descriptor-backed output schedule. Automatic source application excludes
them.

Runtime original C body artifacts consume the shared namespace binding recipe
with the retained `NativeCompilerWords`. Ordered visits allocate each compiled
local and prepare its original name/value operands at the native compiler's
position. A declined attempt keeps its local declarations and literal
allocations, withdraws its executable child publication, and prepares the
original children again for generic invocation. Parser-expanded literal members
retain their original source projection and use pooled headers directly.

An inline namespace binding evaluates the original name object, resolves the
pooled root namespace operand for `global`, and links the exact compiled slot
through the original namespace-only variable lookup owner. The alias exists
before the assigned value is evaluated. Indexed stores retain the normal
variable callback and trace protocol. Compiler preparation alone supplies no
runtime alias or successful store. Native fixture tests compare instruction
counts, complete local layouts, literal order and distinct private list headers
for accepted prefixes followed by a declined tail.

Source compiler analysis selects namespace and switch preparation through the
same authenticated complete original-word adapter. `switch_compilation_at`
requires unanimous retained source, compiler policy, lookup table and native
registration guards. Its original arm spans describe compilation geometry; they
do not assert which arm executes. Preflight visits the original subject before
arm bodies and compiles only arms marked `compile_body` by the shared native
switch recipe. This includes list-arm and parser-expanded geometry, duplicate
jump-table suppression and bodies required by continuations. Generic switch
selection retains ordinary original substitutions. A changed or truncated
word vector, missing source issuer or missing physical compiler point withdraws
source-dependent admission.

## Original coroutine compiler and execution owners

`compile_native_coroutine` accepts the retained original parser vector, the
independently selected C release and its original compilation frame. Preserve
its ordered steps across compiler preflight, emission and Runtime preparation;
changing word geometry or missing compiler evidence withdraws the recipe.
`NativeCoroutineCompilationUnavailable::Generic` is a native decline, while
`Geometry` means original operands are unavailable. Neither grants handler,
header, namespace-token or coroutine-suspension authority.

A relay's namespace object and argv List remain their actual original objects.
Resolve that namespace after restoring the resumer's variable frame, without
turning a reported name into a new lookup operand. Creation uses an admitted
direct invocation carrier retaining its original namespace/argv List, rather
than a regenerated script. The Runtime serialises owning object handoffs across
its coroutine threads; byte probe/inject interfaces remain independent and
cannot certify original headers. Native wasm suspension stays a typed
unavailable execution capability.
