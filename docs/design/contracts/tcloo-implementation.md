# TclOO implementation

## Summary

The TclOO subsystem provides class hierarchy analysis for the LSP and runtime
execution in both the bytecode VM and standalone runtime. It covers
`oo::class create`, `oo::define`,
`oo::objdefine`, constructors, destructors, methods, mixins, filters, private
variables/methods (TIP 500), properties (TIP 558), and configurable support.

## Architecture

### Registry-driven definer grammar (`tcl-registry`)

Per `AGENTS.md`, member **recognition** and **argument layout** come entirely
from the definer's `definition_body` grammar (`tcl-registry/src/definer.rs`):
member sub-keywords (`method`, `constructor`, `variable`, …) with their
body / param / var layout (`MemberKind::Flat`), nested-member wrappers
(`self`, itcl's access modifiers — `MemberKind::Wrapper`), and flag-keyed
forms (`property` — `MemberKind::FlagKeyed`).  TclOO, snit, and [incr Tcl]
are pure registry data; the shared walkers hold no member-keyword lists.

What each member *declares* is registry data as well: `MemberSpec::effect`,
a family-neutral `MemberEffect` — `Callable` (receiver, `CallableRole`, and
the name / parameter-list / body slots) for `method`, `classmethod`,
`constructor`, `destructor`, snit's `typemethod` / `onconfigure` / `oncget`,
itcl's `proc`, and snit's `proc` (role `Procedure`: a procedure in the
type's namespace, reached by name and never dispatched); `Forward` for
`forward`; `StateDeclaration` for
`variable`, `typevariable`, `component`, `common` and snit's `option`;
`Relation` for `superclass`, `mixin`, `filter` and itcl's `inherit`;
`Visibility` for `export` / `unexport`; `Retraction` for `deletemethod` /
`renamemethod`; `InitScript` for `initialise` and snit's `typeconstructor`;
and `Configuration` for the wrappers, `definitionnamespace`, `property`,
`delegate` and `expose`.  A wrapper's effect on what it wraps is
`MemberSpec::wrapper_shift` — `self` moves the member to the class object,
`private` (and itcl's `public` / `protected` / `private`) declares its
visibility.  `DefinitionBodyGrammar::member_row` reads one member statement
into a `MemberRow` — the effect, the receiver after every wrapper, the
literal name, the `MemberArity` of a literal parameter list, the visibility
(option word, then wrapper, then the family's name rule), the body operand
and a slot's operation — so a consumer routes a member by what it declares,
never by its keyword.

The list-valued definition words — `filter`, `superclass`, `mixin`,
`variable` — are **slots** (`oo::Slot` instances in real Tcl), not
assignments, and their behaviour is registry data too
(`MemberSpec::slot`, a `SlotSpec`): each slot carries its
C-pinned default operation (`-append` for `filter`/`variable`, `-set` for
`superclass`/`mixin` — identical in 8.6.16 and 9.0.4) plus a dedup rule
(`variable` dedups, `filter` keeps duplicates), and the explicit
`-set` / `-append` / `-appendifnew` / `-prepend` / `-remove` / `-clear`
operations fold through the single `SlotSpec::apply` fold.  Every consumer
(the instance `filters`, the class-object `class_filters`, `superclasses`,
`mixins`, and declared `variables`) takes the identical fold, so `filter a;
filter b` keeps both live and `superclass -append B` extends rather than
being dropped as a flag.  Known limit: an unrecognised leading `-op` word
aborts the whole definition in real Tcl; the fold conservatively leaves the
slot unchanged rather than modelling the abort.

### Manufactured-class visibility

A registered metaclass command's `CommandSpec::manufacturer_methods` describes
calls on that command. `CommandRegistry::manufacturer_method_for_descriptor`
reads the authentic selected descriptor, preferring these own-command rows. Its
`definition_body.manufacturers` describes the default
manufacturer surface of the class it creates. `TCLOO_ABSTRACT_GRAMMAR` keeps
`create`, `new` and `createWithNamespace` private on an abstract class, while
`oo::abstract` itself retains its selected factory methods. A concrete subclass
created by `oo::class` uses the ordinary class grammar.

Original constructor-shape queries read this selected grammar and then apply
original class-object export and unexport operations. An explicit
`oo::objdefine Abstract export new` can therefore expose `new`. A retained
factory attempt remains distinct from visible constructor syntax. Logical
advice and Native source names retain their separate purposes; neither issues
a Native constructor entry, runtime class identity or successful result.

SpecTcl and Studio share the shipped `tcloo-abstract` grammar name, preserving
these defaults when a selected command descriptor is imported or rendered.
Custom definers must describe their actual manufactured-class visibility in
the same grammar rather than relying on a metaclass reporting name.

The abstract manufacturer visibility proof
records public C9.0 and C9.1 method visibility, concrete subclass construction
and explicit export. C8.4, C8.5, C8.6 and Jim controls do not reach an abstract
factory worker; those availability observations supply no abstract semantics.

### Member arguments that name commands (`record_member_command_references`)

A member argument that *names a command* is a first-class command reference,
recorded during the class-body walk so navigation reaches it exactly as a
direct call does.  Which arguments name commands is registry data, never a
member keyword the walker knows by name:

- `MemberSpec::all_args_ref == Some(MemberRefKind::Class)` — every argument is
  a class (`superclass A B`, `mixin ?-append? M …`, `[incr Tcl]`'s
  `inherit Base`).  A class is a command in `TclOO`, so each resolves in the
  referencing class's namespace (the one-hop call-site rule) and is recorded as
  a `command_invocation`.
- an `ArgRole::CommandName` position — one argument names a command
  (`forward NAME TARGET …`: the delegated command).

The recorder (`analyser/oo.rs::record_member_command_references`) unwraps a
`MemberKind::Wrapper` prefix (`self mixin …`) and skips flags (`-append`) and
dynamic names (`superclass $base`).  Because these references land in the same
`command_invocations` collection as ordinary calls, find-references, rename,
go-to-definition, and call-hierarchy resolve them across files through the
workspace index — and rename and references can never disagree about a
`superclass` / `mixin` / `inherit` site.

### Original member and configuration metadata

`ClassDef::original_members` retains counted original method operands,
canonical declaration occurrences, receiver side, visibility and forward
inputs. `original_relations` retains superclass and mixin operands with their
own caller lookup geometry. It joins providers before the shared
`SlotSpec::apply_values` fold, so removal and deduplication compare provider
identity. Instance and class-object mixins are separate lists. A no-argument
slot query contributes no update; explicit empty `-set` and `-clear` remain
updates. Unknown operand coverage withdraws the affected list until an exact
replacement renews it.

`AnalysisResult::original_class_configurations()` retains each configuration's
own target receipt and member/relation delta. The target issuer authenticates
the full original command, the Registry-selected configuration ordinal and
operand effects at its retained point. Local deltas join an original class
declaration by its full implementation allocation site. A moved class can keep
that allocation; aliases and ordinary instances cannot borrow it. External
targets retain an independent lookup obligation and receive no class identity
from a reporting map. An unowned target withdraws effective original own views
while preserving their birth declarations.

These are source metadata inventories. They do not prove that a definition
worker executed, that an object was manufactured, or that a method is selected
at a later invocation. Positioned receiver state, worker admission, temporal
configuration selection and native compiler/body permissions remain separate
obligations. The String maps remain reporting and explicit compatibility data.

### Callback prefixes that name a method (`references.rs`)

A method reached through a callback prefix — `after 0 [list [self] tick]`,
`fileevent $s readable [namespace code [list my read]]`, `socket -server
[namespace code [list my connect]] $port` — is a method reference, and the four
navigation providers resolve it through one owner
(`tcl_lsp_core::references::callback_targets_from_command`). Recognition is
registry data throughout: the *slot* is an `ArgRole::Body` or
`ArgRole::CommandPrefix` position, the *builder* carries
`BUILDS_COMMAND_PREFIX` (`list`), and the *wrapper* carries
`WRAPS_COMMAND_PREFIX` (`namespace code`). No consumer names a command.

Three conditions bound it, and each abstains rather than guessing:

- **A sole substitution.** A compound outer word changes the prefix after the
  substitution runs — `[list [self] tick]Suffix` invokes `tickSuffix` — so only
  a whole-word `[…]` is an exact representation of the built command.
- **Not `{*}`-expanded, at any level.** `{*}` splices the value into the
  argument list, so the registry's role indices no longer describe where
  anything landed: `lsort -command {*}[list [self] compare] $items` runs the
  *object command* as the comparator and passes `compare` as an ordinary
  argument. Reading the word as if it were the callback slot invents a
  reference — and rename would rewrite it. One expansion invalidates every
  *later* index too, not just its own word, so the check is "every word up to
  this one is written out", and it is applied at the consumer's callback slot,
  inside the built prefix, and again at each `WRAPS_COMMAND_PREFIX` hop:
  `after idle [namespace code {*}[list my tick]]` gives `namespace code` two
  arguments and errors rather than dispatching anything. The compiler's prefix
  scan and the navigation scan both apply it.
- **One unambiguous same-scope constant**, for a prefix stored in a variable
  first (`set cb [list [self] tick]; bind .w <Button-1> $cb`). Every write to
  the name is a candidate: more than one write, a dynamic write, a scope alias
  in the frame, or a qualified or array name and the variable is omitted, so
  reassignment, branch joins and parameters abstain without a target.

A class system whose definer the analyser does not model records no `ClassDef`
at all, so none of this is reachable in such a file however well-formed its
callbacks are — `::clay::define` is the corpus example (#1956).

### LSP analysis layer (`rust/tcl-compiler/src/analyser/`)

The analyser (`oo.rs`, `class_hierarchy.rs`) recognises `oo::class create` /
`oo::define` / `oo::objdefine` during static analysis, building `ClassDef`
entries in the semantic model.  These feed the `rust/tcl-lsp-core` providers:

- **Hover** (`hover.rs`) — class hierarchy, method signatures, inherited
  methods.
- **Go-to-definition** (`definition.rs`) — method bodies and class
  definitions, including the cross-file method paths.
- **Completion** (`completion.rs`) — methods in `my` and `self` contexts.
- **Type hierarchy** (`type_hierarchy.rs`) — supertypes and subtypes, from
  the owner-aware class-hierarchy index.
- **Folding + semantic tokens** — the shared `oo_body.rs` walker, dispatching
  on `MemberKind`, never a keyword.

Every member statement — in a `TclOO` body, a snit type's or an itcl
class's — is read through its `MemberRow`, and one `match` on the row's
effect (`member_landing` in `oo.rs`) decides where it lands; the `TclOO`
fold (`apply_oo_subcommand_in`) and the snit and itcl walkers share it, and
a wrapper has no arm of its own because its shift is already in the row's
receiver and visibility.  The providers then read the recorded member, never
the keyword: a constructor's or destructor's declaring keyword is its
synthetic `<keyword>` name (`MethodDef::is_declared_by_keyword`) and what it
is its `kind`.

| Row | Lands in (`ClassDef`) | Notes |
|---|---|---|
| `Callable`, role `Method`, on the instances | `methods`, kind `method` | `method`; `private method` with visibility `private` |
| `Callable`, role `Method`, on the type object | `class_methods`, kind `classmethod` | `classmethod`, snit's `typemethod`, itcl's `proc`, `self method`; `is_self_method` when a receiver-moving wrapper put it there |
| `Callable`, role `Constructor` / `Destructor`, on the instances | `constructors` / `destructor`, named `<keyword>` | a type-object spelling (`self constructor`) is no member |
| `Callable`, role `Accessor` / `Mutator` | the methods of its side, named `<keyword -option>` | snit's `oncget` / `onconfigure` |
| `Callable`, role `Procedure` | an ordinary proc | snit's `proc` |
| `InitScript` at definition | `MemberCurrentNamespace::DefinedEntity`: the class methods, named `<keyword>`; `RuntimeReceiver`: the class's init script, walked in a class-keyed scope | snit's `typeconstructor`; `TclOO`'s `initialise` |
| `Forward`, on the instances | `methods`, kind `forward`, with its target and prepended words | a class-object forward is not recorded |
| `StateDeclaration`, on the instances | `variables`, through the slot fold | `variable`, `private variable`; `self variable` is not an instance variable |
| `Relation` | `superclasses` / `mixins` (the instances only) and each side's filter slot | the slot fold; itcl's `inherit` adds its list |
| `Retraction` / `Visibility` | the named members of the row's side | `deletemethod`, `renamemethod`, `export`, `unexport` |
| `Configuration`, flag-keyed | `properties` | `property`, until its 9.0 accessors are `Callable` rows of their own |
| `Configuration` | nothing | the wrappers themselves, `definitionnamespace`, snit's `delegate` / `expose` |

The lowering reads the same row: `MethodKind::from_effect(role, receiver)`
names the IR frame a callable opens (a method on the instances, a
class-method frame on the type object or for a procedure, a constructor or
destructor on the instances; nothing for an option handler or a type-object
constructor), and a definition-time script opens a class-method frame only
under `DefinedEntity`.

### MRO algorithm (`rust/tcl-syntax/src/mro.rs`)

Method resolution order uses a linearisation matching C Tcl's algorithm.  It
lives in `tcl-syntax` so the analyser and the bytecode VM share one
implementation without the VM depending on the compiler.

### Export state vs. implementation (`ClassHierarchy::spine_map`)

External dispatch reads **two independent facts** off the linearisation, and
`TclOO` sources them from different classes:

- the **export flag**, from the *most specific* class on the receiver's
  superclass spine that *mentions* the member name;
- the **implementation**, from the first class on the full MRO that declares a
  body.

They come apart because `export` / `unexport` accept a name their class does
not define: C's `TclOODefineExportObjCmd` creates a body-less method-table
entry whose only content is the flag.  `info class methods` does not list such
an entry, so the state cannot be read off the member tables alone — the
`ClassDef::exports` / `unexports` sets (and their class-object twins) are the
record.  Oracle, byte-identical on tclsh 8.6.16 and 9.0.4:

```tcl
oo::class create Base   { method tick {} { return base } }
oo::class create Child  { superclass Base } ; oo::define Child { unexport tick }
[Child new] tick     ;# -> unknown method "tick"   (Base's public body, suppressed here)

oo::class create Base3  { method tock {} { return b3 } ; unexport tock }
oo::class create Child3 { superclass Base3 ; export tock }
[Child3 new] tock    ;# -> b3                      (Base3's unexported body, revived here)
[Base3 new] tock     ;# -> unknown method "tock"
```

`private` is not such a flag: it is a separate, class-local slot, and a
subclass's `export` cannot lift it (9.0.4).  Internal (`my`) dispatch ignores
the export flag entirely and reaches `Base`'s body in the first case above.

**Every `mixin` branch decides for itself.** C's
`AddSimpleClassChainToCallContext` enters each mixin with a *fresh copy* of
the dispatch flags, so a branch's `export` / `unexport` governs that branch
alone and never travels to another.  The dispatch is therefore a sequence of
mixin-free spines — each reachable mixin branch, then the receiver's own
`superclass` spine — and a provider answers to the flag of the **first branch
that reaches it**:

```tcl
oo::class create A      { method m {} { return A-m } }
oo::class create MBase  { method m {} { return MBase-m } }
oo::class create MChild { superclass MBase } ; oo::define MChild { unexport m }
oo::class create D { superclass A ; mixin MChild }
[D new] m           ;# -> A-m       (the mixin branch is empty; the spine answers)

oo::class create MPub { superclass MBase }
oo::class create D2 { superclass A ; mixin MPub }
[D2 new] m          ;# -> MBase-m   (a live mixin branch outranks the spine)

oo::class create NBase  { method n {} { return NBase-n } ; unexport n }
oo::class create NChild { superclass NBase ; export n }
oo::class create A2 { method n {} { return A2-n } }
oo::class create D3 { superclass A2 ; mixin NChild }
[D3 new] n          ;# -> NBase-n   (the branch's own export revives its base)
```

Branch roots are collected transitively, so a `mixin` written on a superclass
or on another mixin is walked the same way.  `ClassHierarchy::spine_map` is the
mixin-free linearisation each branch is read along — the same resolved
`superclass` edges as `mro_map`, with the mixin edges withheld, from the same
[`tcl_syntax::mro`](../../../rust/tcl-syntax/src/mro.rs) owner, so the two
orders cannot disagree about which qualified name a bare `superclass Device`
meant — and `ClassHierarchy::mixin_map` carries the owner-resolved mixin edges
that name the roots.  `WorkspaceIndex`'s `ClassEdges` is the cross-file twin,
and both dispatch folds
(`tcl_lsp_core::oo_dispatch::method_dispatch_provider` in-document,
`WorkspaceIndex::dispatch_chain` across files) read the flag the same way.

The two dispatch kinds can consequently land on **different classes** for one
receiver, and consumers must ask separately: with the `MChild` shape above,
`my m` inside `D` reaches `MBase::m` while `[list [self] m]` reaches `A::m`.

Known limit: with multiple `superclass`es the spine is walked in declaration
order and the first mention wins.  C keeps each branch's flags independent, so
a later branch disagreeing with the first is approximated rather than
modelled — the same single-linearisation approximation `mro_map` already
makes.

### VM runtime layer (`rust/tcl-vm/src/cmd_oo.rs`)

The VM manages the object/class registry at runtime:

- **Object lifecycle** — creation, destruction, per-object instance
  namespaces (`oo::Obj<N>`).
- **Method dispatch** — walks the shared MRO, applies filter chains, handles
  `next`/`nextto`.
- **Variable binding** — `my variable`, `my varname`, private variable
  mangling using creation IDs (TIP 500).
- **Introspection** — `info object` / `info class` subcommands
  (`rust/tcl-vm/src/cmd_info.rs`).

Class name resolution during `oo::define` body evaluation resolves relative
names in the namespace where `oo::define` was invoked (`cmd_oo.rs::
resolve_class`, mirroring C's `GetClassInOuterContext` — the one-hop
call-site rule), not the `::oo::define` evaluation namespace.

### Runtime object identity

`tcl_core_types::OoId` is the shared, authoritative interpreter-local identity
of an object or class. Both runtimes carry that identity through class,
superclass, mixin, method-provider, and active-call relationships; a
fully-qualified command name
is only a Tcl-facing display projection. The command-table variant carries the
same identity so rename, import resolution, and dispatch never recover an
object by spelling.

This separation matters while deleting an active namespace. Tcl unpublishes
the namespace immediately but retains its exact namespace and command
generation until the last active frame returns. A newly created namespace may
therefore publish an object at the same fully-qualified spelling while a
relative call in the retained frame still reaches the old object. OO cleanup
runs only after each exact namespace token passes its own activation check.
Commands and default object namespaces created by a delete-trace callback join
the dying command-home token and its fixed-point teardown; they do not recreate
the visible namespace tree.

Command retirement is likewise identity-based. One runtime-owned seam removes
every public, retained-generation, or hidden command variant carrying an
`OoId`, including the private `my` and `myclass` dispatchers, without touching a
same-named replacement. Unknown-method scope and registry-installed root
classification also carry `OoId`; the root's original registry spelling is
retained only as its dialect-availability key. `OoId` has no universal missing
or default value because allocation policy belongs to each runtime.

Command mutation keeps the exact `OoId` and command generation together across
visible, retained-generation, and hidden tables, including rename, hide,
expose, replacement, and deletion. Trace sidecars carry the same generation.
Callback-bearing lifecycle phases re-resolve that generation before unlinking,
and discard only its sidecars, so a moved old object is still destroyed while a
newer replacement at the same spelling and its traces survive. The standalone
implementation lives in `runtime/rust/src/cmd_oo.rs`; its namespace-generation
lifecycle is owned by `runtime/rust/src/interp.rs` and
`runtime/rust/src/namespace.rs`.

## Test conformance

The behavioural suites live in `rust/tcl-vm/tests/cmd_oo_e2e.rs` and
`runtime/rust/src/cmd_oo.rs` / `cmd_namespace.rs` (tclsh-pinned end-to-end
vectors), plus the analyser's OO suites
(`rust/tcl-compiler` `analyser`/`oo` tests, `mro_lattice_adversarial.rs`).
Reference results captured from real tclsh 8.4–9.0 are queryable via the
`test-results` skill (`tests/test_reference/<version>/`, written by
`scripts/capture/test_results.sh` on demand — not checked in).

## Key files

| File | Role |
|------|------|
| `rust/tcl-vm/src/cmd_oo.rs` | OO runtime (object/class registry, dispatch, define body parsing) |
| `rust/tcl-vm/src/cmd_info.rs` | `info object` / `info class` introspection |
| `runtime/rust/src/cmd_oo.rs` | Standalone OO registry, lifecycle, dispatch, and introspection |
| `runtime/rust/src/interp.rs` | Standalone command identity and namespace-finalisation integration |
| `rust/tcl-syntax/src/mro.rs` | MRO linearisation (shared analyser ↔ VM; also each mixin-free branch spine) |
| `rust/tcl-lsp-core/src/oo_dispatch.rs` | In-document dispatch entry + effective export state |
| `rust/tcl-lsp-core/src/workspace_index.rs` | Cross-file dispatch chain (same fold, workspace records) |
| `rust/tcl-compiler/src/analyser/oo.rs` | Static OO analysis (class/method extraction) |
| `rust/tcl-compiler/src/analyser/class_hierarchy.rs` | Owner-aware hierarchy index + one-hop class resolution |
| `rust/tcl-registry/src/definer.rs` | Definer body grammars (TclOO / snit / itcl as data) |
| `rust/tcl-lsp-core/src/oo_body.rs` | Shared member walker (folding, semantic tokens) |

### Original forward prefixes

Runtime TclOO forwards retain an `obj::Owned` native List of the original
prefix members. Definition builds that List after the selected method-name
stage; it does not read each member's String. `info ... forward` returns the
same List, and declaration cloning shares its retained List owner. Invocation
uses the existing selected List accessor and
`Interp::dispatch_in_lookup_namespace` with the object's actual namespace.
The caller's variable frame remains in force and the prefix headers are not
reconstructed from reported bytes. Currency checks and header leases remain
independent of OO identity, namespace and List-protocol admission.

The original C 8.6.18, 9.0.4 and 9.1.0 controls in
the OO operand-stage proof
separate declaration, raw List access, explicit presentation and invocation
reads. They cover the fixed observed inputs. The initialized C 8.4.20, C 8.5.19
and Jim providers have no TclOO command in those controls. No unsupported OO
execution follows from their availability records.

### Original definition-frame reports

Definition and object-evaluation frames retain the original argument tape with
allocation-only header leases. Installing a tape reads no argument String and
adds no native object reference. The actual admitting caller still supplies
live original headers. A retained lease cannot revive a header whose native
roles have retired.

The shared `Introspect::level_argv` query returns a fresh List when a report
asks for the invocation. That List retains the same original members. Its
outer typed execution error remains distinct from an absent frame, so a
refused report cannot enter Guest bad-level formatting. Explicit presentation
uses the existing checked original String producer outside frame borrows;
frame or native-purpose authority does not follow from operation currency.

### Original method-record cache ownership

Runtime idle method-name primary and table caches keep weak observations of
the original Method records. A validated call entry retains every original
record in its reached chain; `next` frames share that entered owner. A missing
record refuses that cached entry instead of finding a replacement by its
reported name. Original receiver, table owner, mutation stamp and operation
checks remain separate from these reference roles. Cache String extraction
and primary retirement run outside the OO borrow and must preserve the
original world before later cache publication. The original `StashCallChain`
String/free stages precede the first `TclOOInvokeContext` ALL-record retain;
cache construction does not acquire that entered role. A declaration deleted
during cache installation cannot be revived for the pending entry.

Same-record replacement changes the record's current payload. Retaining an
entered record does not retain every old clientData payload; real procedure,
forward and property execution owners keep their independently required
roles. The original custom MethodType controls distinguish idle deletion,
self-deletion, inherited `next`, same-record replacement and cache cleanup.
Their observed native references do not attest a procedure-body activation or
a generation from a printed method address.

### Original method names and rename entries

Each named Runtime Method record retains the genuine original name header.
Its declaration table owns a separate native key reference to that header.
Authored method and forward installers use the supplied operand; generated
built-ins use their real generated String object. Constructors and destructors
have no named table entry. A shallow Rust observation shares these owners,
whereas declaration duplication acquires independent record-name and key
references alongside its copied payload.

Rename looks up the source before accessing the destination. It keeps the
original target and exact source-entry observation across the checked
original destination getter, and refuses deletion/recreation of that entry.
The selected Registry policy distinguishes the same entry from an occupied
different entry. It preserves counted keys, including NUL and literal colon
bytes, while native diagnostic `%s` fields use their separate C string extent.
A successful splice preserves the record and its visibility, installs the
destination's two actual name references, then retires the old record-name
and key owners outside the OO borrow. Reached callback effects remain visible;
original currency and entry checks precede later cache or result publication.
`self method` reads the current entered record's genuine name header, so an
active rename does not report the old selector spelling. Unnamed reporting
retains its existing separate sentinel path; native Foundation sentinel
identity and aggregate self reports remain independent purposes.

The original rename proof
separates stock worker stages from public Eval error-context and code-reset
stages for C 8.6.18, 9.0.4 and 9.1.0. Its custom MethodType payload does not
attest Runtime procedure, forward or property execution. Initialised C 8.4.20,
8.5.19 and Jim availability controls supply no supported OO rename purpose.

The reached method chain carries a selected `Named`, `Constructor` or
`Destructor` report role. The constructor and destructor entry producers issue
those roles; `next` inherits the same chain context. A literal method name such
as `<constructor>` or `<destructor>` stays `Named`.

The actual OO bootstrap separately owns the constructor and destructor report
headers selected by `NativeTclooMethodReportProtocol`. `self method` returns the
current named Method's original header in Runtime, or the actual Foundation
header for a special role. Unnamed Method records acquire no table-name hold.
Missing Foundation ownership or unsupported selected semantics refuse instead
of reconstructing a report header from a sentinel-looking name. The VM retains
its actual Foundation values and reports the same selected roles; its value
identity assertions make no physical C refcount claim.

The original method-report path uses the selected protocol. Ordinary CLI
report values remain an independent observation channel. Method cache and mutable
client-data lifetime observations do not establish these report header roles.

### Native property slot replacement

The selected C9.1 property slot worker releases its selected cached List, takes
one genuine property-list reference for every incoming member, then retires the
old member owners outside OO borrows. Only then does it obtain each original
member's counted String and deduplicate through the object-hash key role. A
unique member keeps its incoming property-list reference; the temporary hash
key owns a separate reference until the unique list is adopted. A duplicate
releases only its incoming property-list reference. A held old cached List
retains its original members independently of replacement.

The actual target and entered original-operation receipt are checked after
reached getters and retirement callbacks and before later adoption or epoch
publication. Reached String and retirement effects remain visible on refusal.
Host/context callback controls establish this software transport only; the
fixed original C9.1 property observer rows establish their separately matched
incoming, hash-key and cached-List reference stages. Older C cores and Jim do
not gain this property worker from those observations.

### Original object publication

Runtime object publication retains the actual allocated OO target and its
instance namespace token before binding the public command. The fallible
private-dispatcher installer checks that same target, namespace incarnation and
entered operation after each reached replacement callback. Refusal stops later
private bindings, constructor, definition and cloned-method work. Allocated
partial objects still pass through mandatory teardown, preserving the first
structured cause and reached callback effects. Foundation installation stops
its remaining OO stages when publication refuses; currency conveys no object,
namespace or creation-purpose permission.
### Property owner retirement

Runtime property registration and invalidation retain the original OO target and
entered operation through reached member and cache releases. Extracted owners
retire outside OO borrows; a structured refusal stops later direction, metadata,
epoch and result publication. Mandatory cleanup enters before extraction and
records an entry refusal on the actual interpreter before any payload callback.
Every captured original role still retires after refusal, preserving the first
cause and reached effects.

Each actual property producer stores its pure C9.1 release recipe with the
facet. Cleanup uses that originating recipe after a profile change, releasing
readable and writable cached Lists before either member array as inspected in
`TclOOReleasePropertyStorage`. Unissued compatibility storage uses ordinary
ownership cleanup. This selected source order conveys no header or invocation
authority and carries no external teardown callback measurement.

Property report construction retains the original operation, target, Foundation
epoch, existing method-world mutation facts and selected member identities. It
pins allocations without adding native references, sorts and reads private
variable operands outside OO borrows, and stops later getters after refusal.
Private-variable comparison also checks the original declaration headers and
names because declaration replacement need not change method mutation facts.
Cached List replacement releases extracted owners before adoption; a changed
world cannot be stamped onto a report still being constructed.

A materialized property report retains its original operation, target and header
lifetime while ordinary accessor mutations remain legal. Read-all owns the
source-authentic working List reference and traverses that frozen snapshot. A
multi-write name table keeps its original copied List and frozen Strings; each
later old-table index selects the corresponding member of the current property
header, as the selected C worker does. Neither role derives fresh authority for
an old table. Both report phases transport full structured errors through every
actual consumer and result transfer.

Whole Runtime Object and Class records are extracted before their owning method,
forward-prefix, declaration and name payloads retire outside OO borrows. The
actual target is marked torn down before payload retirement; callbacks cannot
adopt it as a live definition or invocation receiver. Destruction retains its
original operation before result or callback stages. A first refusal suppresses
optional destructor, result and rename-name publication, while original command,
namespace and registry cleanup still completes. Reached effects remain visible.
Active entered method records retain their independent real call-chain owners;
idle Weak cache observations add no owning role. These software cleanup stages
carry no aggregate Native payload or hash-iteration chronology claim.

Declaration replacement retains the caller's original operation and exact OO
target. The actual new declaration vector is adopted before the extracted old
headers retire outside the OO borrow. Reached callback effects remain visible;
a first refusal or target withdrawal stops later result publication. A callback
may legitimately replace the newly adopted declaration roster without a method
world or old-inventory veto. This software replacement order carries no Native
variable-map hash or retirement chronology claim.

Definition mutations retain the original target before name and option getters,
then check it before class, namespace, method-removal and result stages. A
withdrawn target cannot receive a new class facet or a successful namespace
publication. Each method removal retires its actual owning record outside the
OO borrow before a later operand or invalidation stage. The selected C9.1
export recipe interleaves each original name getter with its flag effect; a
later refusal preserves earlier reached flags. Other releases retain their
explicit compatibility schedule without an original chronology claim.

A Class continuation observes the actual class facet's birth independently of
its object's creation identity. New Class allocation and genuine class copying
issue a fresh birth; shallow Rust observations preserve the existing one. A
class demotion followed by promotion cannot make an earlier Class continuation
current, even when the same object and operation context survive. Replaced
facets refuse through the structured execution transport; ordinary method or
property mutations do not invalidate this lifecycle observation. This Runtime
owner supplies no Native Class pointer, payload lifetime or source authority.
Class introspection checks its retained facet after reached option and pattern
getters before later class access or report derivation.

Method-specific introspection retains the selected Object or Class before its
method-name getter. Object method, property, membership and variable reports
also keep their original receiver through later operands; instance-variable
reports join its namespace to the existing namespace ledger before reading the
table. Withdrawn receivers cannot become missing-method reports or successful
relation results. An initially absent `isa` receiver remains a false
precondition and leaves the later relation operand unread. Materialized reports
remain independent of subsequent ordinary metadata changes.


### Original class relation candidates

`ClassRelation` retains the enclosing definer and member source vectors with
selected target/body ordinals. The method barrier rejoins the original Module,
Registry and source purpose before reading ancestry. Source factory publication
or the canonical class configuration operand supplies the declaring allocation;
the shared completed class receipt and existing dependency-currentness owner
supply the inherited allocation candidates. Source and member geometry remain
independent of entered receiver and method dispatch facts.

The local graph unions those allocation candidates as MAY edges. A missing
source, unavailable target or dependency, conflicting class projection or
unmapped external ancestor widens the whole barrier before an empty local effect
set can enable propagation. Reporting tails cannot recover an edge. Braced
operand punctuation stays original source data; the source owner forms its value
and selects any class target. This projection supplies no Native class lifetime,
lookup absence, method entry or physical variable-cell licence.
