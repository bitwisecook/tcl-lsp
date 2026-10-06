# Evaluated script regions

Evaluated script operands belong to a resolved invocation, its variable frame,
its live implementation and its Tcl completion protocol. A `Body` argument
role supplies syntax and editor navigation; it does not establish these facts.

The registry owns `BodyExecutionSpec` and `CapturedLifecycleSpec`. The latter
selects setup/body/cleanup operands once, preserves option order and duplicate
last-value selection, and states exact package hook and option-version
contracts. Package versions remain independent of Tcl core versions.
`StockBodyProvider` records the explicitly selected distribution implementation,
its required native execution family, installed commands and exports, optional version alternatives and
parser-derived core lookup prerequisites. Core dependencies are checked in both
the root and provider namespaces before loading and at phase entry, so a local
shadow or globally replaced builtin withdraws the implementation proof. An unspecified version uses the authored
intersection; it does not infer a package version from the core release.
The source kernel checks the required family against the invocation dialect
before selecting a loader and at later implementation queries. An unversioned
explicit C Tcl dialect retains its native family; Jim and F5 do not inherit the
C provider contract from a matching package advertisement.
Private provider state also forms an implementation dependency. The source
kernel resolves writes through the shared place owner; a write to provider
state withdraws the phase contract unless that state is explicitly modelled.
Iteration configuration is modelled by the shared repetition topology. A
changed custom matcher remains opaque because it can run before cleanup.
Variable aliases use the central `VariableLinkBinding` dialect policy: C Tcl
retains a selected cell, while Jim resolves a selected-frame name on each access.
The compiler's point context, VM storage kernel and C runtime consume that same
policy. Namespace deletion and whole-array deletion therefore preserve C's
detached aliases without making Jim aliases miss a recreated name.

`SourceCommandBindings` supplies invocation-site dispatch and loader provenance.
`effective_command_words` composes alias prefixes and source arguments once and
retains each word's origin. The lowering attachment checks the selected provider,
its helper-command closure, absent extension hooks at phase entry, known script
values and accurate source ranges. An explicitly unknown or absent binding
cannot fall back to the written command spelling.
Region dependencies retain the exact entry dispatch, phase dispatch snapshots
and selected provider contract, including lookup and state prerequisites.
Original declaration and compiler-table advice is a separate diagnostic query.
Its candidates must agree on the selected expression operation and the same
unchanged written operand under the retained grammar. A generic expression can
therefore carry a simplification hint while retaining unknown execution and
operand effects. Such hints contain no replacement; read types select candidates
and the native read, conversion, sharing and store proofs separately license edits.
Procedure result advice likewise identifies original declarations and unanimous
returned-variable candidates without establishing that a caller returned normally.
Method command-head advice uses the same original declaration-local read and an
existing symbolic value version at that invocation. It creates no SSA read or
receiver identity. A declared formal or instance/global name without such a
local value remains unknown. Definite command and method absence use the actual
lookup and receiver receipts independently.

Original body ownership identifies the definition allocation, body image and
frame together. A declaration frame remains distinct from a frame recorded
after native argument binding. Operand-layout advice can retain either owner
without inventing the other; original formal input provenance uses the exact
owning frame and keeps later writes and aliases separate. Reconstructing
original command tokens also retains their invocation inventories, including
the independent declaration-flow query. These receipts add no SSA uses,
successful execution facts or rewrite permissions.

Selected paired-list iterators retain exact frozen list bytes and native
variable grouping, padding and assignment order. Each finite iteration resolves
its output addresses before entering the original body; writes with unknown
addresses or observers retain their effects as residuals. Body completion
controls the next iteration, so break, errors and partial failed stores cannot
license a later body or caller continuation. Stored text grants no numeric
representation, object cache or conversion-erasure authority.

An original TclOO allocation can execute a retained constructor only when its
class, native manufacturer, lifecycle inventory and dispatch dependencies are
closed. The source kernel binds the original payload to the original formals
and enters that body with the allocated receiver. Normal completion publishes
the object identity and discards the constructor result. A failed constructor
deletes the allocated command, executes its retained destructor and preserves
the failure's effects; unresolved destruction callbacks remain explicit
residual effects. An empty counted constructor or destructor body removes that
method record and selects no formal parsing or body activation. Constructor
entry advice alone supplies neither successful allocation nor object identity.

A closed TclOO class inventory can retain one original superclass or one local
mixin whose native member, base allocation and dispatch generation remain live.
The base must have no constructor or opaque dispatcher. Disjoint mixin method
entries retain their original declaring-class activations; overlapping entries
and multiple inherited assignments withdraw this bounded inventory rather than
inventing a method chain. Replacing or modifying a base withdraws its dependent
inventory.
Executable invocations retain effective argument origins alongside the original
runtime words. `GenericInvoke::registry_specialisation_arguments_exact` is the
shared backend condition for argument-position correspondence; inserted alias
arguments require an explicit backend composition before specialisation.

`EvaluatedBodyRegion` is the shared completion graph. Setup's non-OK completion
skips body and reaches cleanup; every body completion reaches cleanup; every
cleanup completion reaches the wrapper join. A selection edge can skip all three
phases. These edges preserve real writes on normal paths and partial writes on
abrupt paths. They never manufacture a definition to silence a diagnostic.
`exit` terminates the interpreter; it is not a captured Tcl return.

The compatibility CFG emits `EvaluatedArguments` before phase entry and
`EvaluatedWrapper` at the join. The first boundary evaluates original words but
has no direct invocation effects. The second projects residual wrapper effects
without evaluating those words again. `CommandTokens::evaluates_words` is the
common read-suppression contract. Exception edges carry non-OK paths;
`analysis_edges` carry ordinary external selection alternatives.

Executable IR evaluates argv once, then uses `RegionChoice`, existing completion
switches and captured joins. `CompleteEvaluatedRegion` retains original runtime
argv and invocation provenance, but represents residual wrapper completion.
World-state consumers project `region.residual_effects.resolve()` here, not the
full invocation footprint. Reporting and matching remain conservative at this
join until a narrower authored wrapper contract is available.

A runtime backend must either implement the complete wrapper protocol or decline
the entire region before entering a phase and dispatch the original invocation
with already evaluated argv. It must never execute expanded phases followed by
the original wrapper. Compatibility codegen retains the original call. Native,
mixed and WASM semantic planning decline unsupported expanded regions atomically.

Current attachment deliberately requires direct, unchanged braced script operands.
The single-list form first runs tcltest's `SubstArguments` and attribute `concat`
protocol; it is not plain Tcl list parsing. Scripted constraints run at the global
frame, output capture temporarily replaces `puts`, and custom matching and hooks
can introduce callbacks. Those contracts require additional proved phases and
remain opaque when their preconditions are unavailable. The registry selector
still retains their operand locations for syntax consumers. SpecTcl emits an
explicit `body_execution` GAPS notice and excludes unsupported proof contracts
from strong analysis rather than silently dropping them.

## Consumers

| Owner | Shared contract |
| --- | --- |
| Registry invocation facts | Body execution descriptor and dialect-stamped argument values |
| Source command kernel | Live dispatch, package loader, phase command/hook closure |
| Lowering ingress | Proof-gated attachment and script source ranges |
| Compatibility CFG / SCCP | Completion routes, exceptional and ordinary selection edges |
| SSA / variable bindings | Argument boundary reads and real phase definitions |
| Executable IR | Ordered argv, phase completion graph, wrapper join |
| World-state SSA / dispatch proof | Residual wrapper footprint at completion |
| Native / mixed / WASM planning | Atomic whole-region support or decline |
| Spec studio / SpecTcl | Explicit authored coverage and unsupported-contract notice |

## Provider lifecycle behavior

The selected tcltest package version determines hooks, repetition and matching
order independently of the core release:

| tcltest version | Lifecycle hooks | Repetition | Matching order |
| --- | --- | --- | --- |
| 2.2.11 | No SetupTest/EvalTest/CleanupTest hooks | One lifecycle | After cleanup |
| 2.3.8 | No SetupTest/EvalTest/CleanupTest hooks | One lifecycle | Before cleanup |
| 2.5.11 | Optional SetupTest/EvalTest/CleanupTest hooks | One lifecycle | Before cleanup |
| 2.6.0 | Optional SetupTest/EvalTest/CleanupTest hooks | testIterations selects complete lifecycles | Before cleanup |

These contracts apply only to the selected distribution implementation. They do
not establish the first release supporting a hook or the behavior of another
provider with the same package name. Jim's independently supplied tcltest
implementation does not inherit the stock C Tcl provider identity.

A write trace on the private testLevel variable can run before setup and after
cleanup. Those hidden wrapper accesses are provider-state dependencies even
when caller source does not name the variable. In 2.6.0, a read trace on
Option(-iterations) can install EvalTest before setup and replace the written
body. Modelling ordinary iteration-count writes therefore does not permit
traces on that storage. Custom matching remains opaque: attachment requires
unmodified stock matcher state and preserves the package's matching order.

The behavior specimens live in
[body_execution fixtures](../../../rust/tcl-syntax/tests/data/body_execution/README.md).
Provider metadata and interpreter-discovery usage are documented in
[scripts/dev/tcltest-provider/README.md](../../../scripts/dev/tcltest-provider/README.md).

Compilation ingress is part of the execution contract. A C source file's root
script evaluates directly, whereas a proc body or a stdin-evaluated script can
select bytecode operations before argument substitutions. Jim retains late
command lookup. A late runtime target and a selected native operation therefore
need distinct proofs. Static body selection also preserves the compiler's
procedure-local-table capability separately from the physical variable frame;
a caller-frame eval does not automatically inherit that compilation capability.

Jim's tcltest 1.0 runs the body after setup errors and exports
global `test`; it does not inherit C's captured lifecycle descriptor.


## Pending returns and source execution

`CommandRegistry::invocation_completion_route` retains the eventual return code
and remaining unwind level. Consumers apply
`InvocationCompletionRoute::through_procedure_boundary_in` at a procedure
boundary; a namespace or evaluation frame leaves the route intact. C Tcl
normalises `-code return` by increasing the level; Jim retains that code until
a procedure releases it. Raw break/continue escaping a C procedure becomes an
error, whereas Jim propagates it. A code released from a configured return
remains distinct from a raw loop completion until its next boundary.

The existing registry return parser supplies these facts and uses the shared
native completion-selector owner. Dynamic levels, dynamic option names,
expansions and unresolved numeric grammar retain explicit unknown alternatives,
including normal continuation where level zero is possible. C 8.x accepts
negative completion-code magnitudes through `UINT_MAX`; C 9.x uses `INT_MIN`
as its lower bound. Jim applies its native wide-to-int conversion. Past-wide
Jim selector behavior remains an explicit unknown static route.

`script_body_flow_for_invocation` selects the native catch layout through
`select_catch_invocation`. `CatchInvocation::route` owns its completion mask:
C process exit escapes every catch, while Jim code 6 is captured by `-exit`
and bypasses the default catch. Unknown masks never become unconditional
capture. Source execution carries normal and abrupt states separately, evaluates
later substitutions only from normal prefixes, and stops outer dispatch when
an argument substitution aborts. Known document math-function procedures reuse
ordinary parameter binding, activation restoration and return unwinding.

Expression and script concatenation are authored topology in the registry.
A faithful one-word source operand retains its original source mapping.
Multiword or changed decoded source has an explicit residual until a
materialised-source mapping contract exists; walking its first operand cannot
prove the complete invocation.

## Point-sensitive representation consumers

Shimmer analysis reads source names through `SsaSourceView::at_statement` or
`at_terminator`. Use-site hints, expression operands, byte-array provenance,
copy sharing, array exclusions and the committed-intrep fixpoint use the same
cell projection. A retargeted alias can therefore select different cells at
successive reads without merging their versions. Loop conversion ledgers key
on cell symbols, while warning labels retain their source spelling. Symbol
names used for display do not build execution environments.

## Variable reads during argument evaluation

A dispatch point follows all argument substitutions. It cannot resolve an
operand read that occurred earlier in the same word. For example,
`"$a[upvar 0 y a]$a"` reads two selected cells, even though both references
have the same spelling. Array-index substitutions run before the enclosing
array read; a read observer runs after that read selects its address.

`SourceVariableAccess` retains the exact original `SourceSite`, undecoded
spelling and immutable shared `ResolveContext` immediately before each reached
read's observer. `SourceCommandBindings::variable_accesses_in_span` supplies
the root's lexical inventory, and `SourceVariableAccess::find_at_source`
selects one exact reference. Missing, lazy-skipped and abrupt-skipped reads
have no context fallback. Repeated execution of the same lexical site joins
its contexts through the shared place owner, preserving uncertainty when
selected cells differ. A cached fingerprint makes retained token hashing
bounded by newly created snapshots; equality still compares the full context.

The central lexer `VarRef::source_range_in` recovers nested index-reference
extents through the native variable scanner. It rejects a different source
allocation rather than deriving delimiters or manufacturing source offsets.
Lowering carries these accesses alongside the original invocation; SSA and
value consumers select the recorded read independently from the later command
dispatch proof.

Finite native completion domains are similarly retained as alternatives:
`InvocationCompletionRoute::TclAlternatives` projects authored exact code sets.
Source execution splits OK from each abrupt code before attaching states. A
native variable-operation error therefore stays an error at a procedure
boundary, rather than joining a failed partial write into successful caller
state. Native variable primitives normalise trace callbacks returning return,
break, continue or a custom code to an error completion.

## Dictionary scope phases and possible body entry

`DictionaryScopePlan` separates mapping, missing keys, body completion capture,
and writeback failure. The physical owner retains the original keyset and
frozen variable names while resolving their addresses again at entry and
writeback. `VariableOperandBindingPhase::BodyProtocol` excludes these operands
from generic pre-body and post-body stores; `DictionaryScopeEntry` and
`DictionaryScopeWriteback(route)` markers project the shared physical effects.
`DictionaryScopeId` includes the actual source allocation and activation, so a
derived script at offset zero cannot reuse an unrelated scope. Authoritative
source snapshots retain the active scope across the body.

C Tcl update and with write back captured ordinary completions. The audited
Jim update implementation is a mutable standard-library procedure, including
its helper dispatch and procedure return boundary. Native bootstrap installs
its authored script only when the command is absent; calls subsequently resolve
that command in the actual caller namespace. A public registry spelling does
not prove that the original script or its helpers remain installed. Jim with
writes back only normal body completion; its native epilogue reports malformed
dictionary errors as an OK completion with an error result. Entry errors remain
errors. These policies are independent descriptor fields, not compiler name
checks. Analysis scope expansion atomically declines at executable backends
until those backends can implement the entire protocol faithfully.

`PossibleBodyInvocation` is a separate, purpose-limited projection from a
converged native handler and frozen argv/frame to possible body entry. It does
not expose opcode, purity or completion permission. The registry's
`PossibleBodyTopology` supplies alternatives, sequences and loop ordering;
`EvaluatedBodyRegion::possible_bodies` retains actual entered script origins.
The analysis CFG includes the unchanged entry path and body paths, retaining
abrupt completions and entered-script effects as May facts. The executable IR
rejects this analysis-only region before argv evaluation, preserving one runtime
invocation. Missing entered source contributes no invented script statements.

Typed returned expressions carry `Return.expr_base` through IR, CFG and source
rebasing. Lowering derives it from the original word extent and the nested
expression parser's relative base; changed or non-affine source has no base.
Implicit math proof queries consume the original AST and this base, never a
string search. Native math command dispatch similarly retains the installed
handler identity through rename and aliases; display argv does not select its
mathematical implementation.

Lifecycle phase reachability is a retained source proof. Each selected traversal
records whether setup, body and cleanup were entered; an incomplete or opaque
traversal remains `Unknown`. `NotEntered` requires every covered traversal to
skip that phase. Lowering may omit a phase and its phase-entry prerequisites
only with this proof, retaining `ExecutionRegionDependency::NotEnteredPhase`.
An absent entered-script record alone cannot prove that cleanup is unreachable.
The coverage key includes the original command allocation, preventing derived
scripts with overlapping offsets from borrowing another invocation's proof.

`VariableContainerModel` is independent of variable naming and alias lifetime.
C Tcl keeps scalar and array roots distinct. Jim array entries belong to an
ordinary dictionary-valued root: scalar reads return that dictionary, scalar
writes replace it, and element writes copy shared dictionaries before mutation.
Native Runtime tables apply this selected policy to ordinary, frame-addressed,
linked and retained-static cells. Dictionary mapping uses the same stores.
The shared array command core retains malformed-dictionary enumeration errors
through `VarStore::array_keys_checked_at`; existence and size queries keep their
native non-array result. Shared behavior specimens are available through
`tcl-test-support::variable_containers`; both Rust adapters consume those
original inputs with independent interpreter expectations.


Representation analyses consume `NormalRepresentationInvocation`, a restricted
projection of the same reached-handler and frozen-argv proof used by normal
variable transfer. A native compiler hook may be unresolved while the handler
converges on one registry-backed implementation. That permits the authored
operand conversion and successful result representation facts; it cannot
permit native emission, script entry, completion control or suppression of
compiler failures. The carrier keeps exact physical read sites and alias
prefix origins. It exposes no general `InvocationFacts`. Commitment dataflow
widens to unknown on exceptional edges, because a failing conversion may stop
partway through the selected operands. Quoted liveness placeholders cannot
establish a later copy-on-write read, and array exclusions use the shared
per-read physical place rather than a guessed name or array-root symbol.

Typed iterator headers retain `IterationBindings(Option<TclType>)`. The iterator
lowerer mints this input expectation from its selected protocol: list values
and dictionary values have distinct expectations, while an array-name iterator
has no container-value promise. The representation facade projects that input
conversion directly. It does not resolve a synthetic command spelling, invoke
the original iterator again, or expose completion/result facts.

Case-list regions use `CaseListSpec::possible_body_operands` for both source
execution and CFG phase construction. Each selected location is an effective
argument plus an optional decoded list-element ordinal. Source execution
records the exact entered script and origin for that location; lowering consumes
that record instead of treating an entire switch clause list as a Tcl script.
Fallthrough entries share the registry clause grammar and do not become script
phases. A dynamic subject in the two-operand subject/list shape retains a
separate unknown selection/error continuation. Other partially unknown option
layouts remain unresolved until the shared option selector can represent them.

A command boundary and an operand evaluation are distinct points. Implicit
function exits have no argument words, and an observed `Complete` follows the
original statement that already evaluated its arguments. Their SSA source-token
projection is empty even if a prior structured command left boundary metadata
on the block. Actual returned value tokens and condition tokens remain attached
to their own evaluation points. This prevents liveness and sharing analyses
from inventing another read of an earlier argument at function exit.
