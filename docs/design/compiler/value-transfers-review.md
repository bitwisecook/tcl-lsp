# Review: registry-owned specialisation and shared evaluation

Review of `claude/spectcl-optimization-discussion-5qhf42` at
`b0f0b432b1d31ba2e53c7e85dfda389d8d5d836a`, covering
[value-transfers.md](value-transfers.md) and
[registry-consumer-contracts.md](registry-consumer-contracts.md).
Prepared on 2026-09-09 by Codex, at the owner's request. This is a
review and proposed revision of the design, not an implementation or an
assertion that the proposals have been approved.

Reading map: [findings](#findings-that-should-change-the-design-before-implementation),
[pipeline and diagnostic boundaries](#end-to-end-integration-and-diagnostic-separation),
[constant-to-dynamic transitions and partial reduction](#constant-to-dynamic-transitions-and-partial-reduction),
[adversarial follow-up](#adversarial-follow-up-try-to-break-the-architecture),
[every-code coverage ledger](value-transfers-coverage-review.md),
[fact interfaces and Rust/SpecTcl examples, including EDA and eBPF](value-transfers-authoring-review.md),
and [owner decisions](#owner-decisions).

## Assessment

The objective is right and worth pursuing: an analyser should know how to
apply semantic facts, while the command registry owns which facts a command
produces and how its specialised behaviour is calculated. The strongest
parts of this branch are the inventory of duplicated knowledge, the shared
lattice proposal, and the recognition that evaluation should reuse the
project's existing semantic engines.

The current design is not ready to serve as an implementation contract.
It sometimes replaces command names with command-specific compiler IDs,
derives execution semantics from metadata that only describes types or
argument positions, and treats successful value calculation as sufficient
evidence for transformations that also remove effects. Its engine proposal
needs a more precise dependency, execution, and caching contract. Several
claimed prerequisites belong to a separate runtime programme.

The design is broadly connected to the analyser/compiler, but an inventory
of consumers is not yet an end-to-end contract. In particular, analysis and
diagnostic generation have good existing boundaries, not yet consistently
enforced ones. The design must finish that separation: reusable semantic
facts first, diagnostic rules second, presentation last. R14 and the
integration section below make this an explicit acceptance condition.

The follow-up inventory covers all 228 central catalogue entries and the
XC, BPF and TLS numbered catalogues: 275 numbered entries in total, plus
certificate-chain, extensible policy/scanner and uncoded verifier findings.
The answer to “does this already represent everything?” is **no**. Exact
value transfer must integrate with independently available type, existence,
alias, effect, completion, range, taint, representation, protocol and
external-world facts. Structural plans and backend safety proofs are not
replaceable by executing a command whose inputs happen to be constant.

The intended completion test should be:

> Adding a command that fits an existing analyser interface changes its
> registry declaration and, if necessary, its registry-owned evaluator or
> shared runtime core. It does not require editing SCCP, the analyser walk,
> diagnostics, or the optimiser to recognise that command or its ID.

This does not require a VM for every command. Direct calculations for
`string` operations and `incr` should use the existing value and numeric
owners. `expr` should use the shared expression engine with controlled
access to proven inputs. An explicitly supported Tcl implementation can
use bounded VM execution where that avoids duplicating an algorithm.
Unresolved runtime inputs still produce an unknown answer: running an
interpreter does not make unavailable information available.

The owner's requested end state is registry-owned specialisation behind
generic analyser interfaces. Whether temporary compiler-owned handlers
are acceptable during migration is a delivery choice, not a different
architectural destination. The review recommends explicitly declared
execution routes rather than inferring VM eligibility from purity alone;
that execution policy still needs agreement. The owner explicitly
confirmed reuse of the project's own regexp engine.

The owner also confirmed that workspace-authored analysis facts are
authoritative, including for reachability, diagnostic suppression and code
elimination. There is no additional trust opt-in, provenance-based precision
cap or certification requirement: authors are responsible for incorrect
facts in their environment. Binding validity, cache invalidation, structural
API validation and evaluator resource limits remain separate correctness
and execution concerns, not gates on whether to believe the author's model.

## Findings that should change the design before implementation

Priority describes the consequence of implementing the proposal as written,
not a claim that this documentation-only branch already changes behaviour.

| ID | Priority | Finding |
|---|---|---|
| R1 | High | A registry-selected compiler handler does not finish the ownership migration. |
| R2 | High | Type and role metadata cannot establish a command's transfer algorithm. |
| R3 | High | The transfer result cannot express preserved targets, ordered writes, or independent command results. |
| R4 | High | A known result does not authorise deleting the invocation that produced it. |
| R5 | High | Expression evaluation needs lazy input resolution and transitive binding evidence. |
| R6 | High | The proposed value transport loses exact values and overstates representation evidence. |
| R7 | High | Deterministic commands in a persistent sandbox do not make hook bodies deterministic. |
| R8 | High | The memoisation contract is broader than a command-trust snapshot or pack key. |
| R9 | High | Opaque switch arm facts cannot by themselves become CFG reachability. |
| R10 | Medium | Direct core reuse still needs release-aware, fallible, bounded adapters. |
| R11 | Medium | A concrete evaluator is not an interval transfer, and determinism is not a monotonicity proof. |
| R12 | Medium | The companion conflates two identity mechanisms and makes unrelated work a prerequisite. |
| R13 | High | Our regexp API currently hides resource exhaustion and capture approximation inside ordinary match results. |
| R14 | High | Diagnostic producers still recreate semantic facts, and presentation sometimes acts as a fact source. |
| R15 | High | Partial reduction needs proof-aware residual expressions; the existing reassociation helper changes floating-point results. |
| R16 | High | A value-only contract cannot cover existing dynamic type hooks, vendor iteration or BPF language/safety semantics. |

### R1. Specify the analyser interface, not just the dispatch catalogue

The proposed `ValueTransfer::Native(ValueTransferId)` explicitly means an
algorithm the compiler keeps. `Expr` then moves the current SCCP arm
unchanged behind that ID. This satisfies a command-name lint but leaves the
specialisation in precisely the consumer whose knowledge the project wants
to remove. The same problem can recur with analyser hooks: a switch over
`Hook::DictWith` is still command-specific if its handler understands dict
syntax and implements the command's binding rules.

Evidence: [the descriptor](value-transfers.md#the-interface),
[the proposed folding changes](value-transfers-migration.md#the-slices), and the current
`dispatch_analyser_hook` in
[analyser/commands.rs](../../../rust/tcl-compiler/src/analyser/commands.rs).

Define an interface in terms of generic operations the analyser already
owns: obtain a proven word value, resolve a storage place, query its prior
fact, enter a described scope, register a described definition, apply a
validated transition, and emit a fact with provenance. Registry-owned
descriptors or evaluators select and compose those operations.

Prefer returned plans and facts to callbacks that receive unrestricted
`&mut Analyser`. A plan can be validated, replayed, cached, and consumed by
more than one frontend. A restricted read-only host interface is appropriate
when an evaluator needs lazy access to input facts. The analyser owns its
symbol tables, SSA, scopes, diagnostics, source spans, and joins; the
registry owns the command-specific rules that use those facilities.

Keep genuinely generic interpreter operations in the compiler. A generic
"evaluate this expression using these input services" operation is a good
interface. An `Expr` arm containing the `expr` command's argument assembly,
release rules, and substitution special cases is migration debt. The review
does not demand moving code generation or runtime variable stores into the
registry: registry-owned specialisation can invoke those owners through
their existing typed interfaces.

Acceptance: add a small private command with a new spelling but existing
semantics, at command and subcommand scope, and make it work without a
consumer edit. Also audit command-specific ID arms, not only string matches.

### R2. Do not derive algorithms from weaker metadata

The derivation table maps `VarWriteTyping::ElementsOf` to
`Destructure(elements_of)` before it checks iteration traits. Both
[`foreach`](../../../rust/tcl-registry/src/commands/tcl/foreach_.rs) and
[`lmap`](../../../rust/tcl-registry/src/commands/tcl/lmap_.rs) already carry
that typing descriptor as well as `LOOP_LIST_HEADER`. The table therefore
classifies them as destructuring. Their `container_arg: 0` even refers to
the synthetic loop-header layout, not the source invocation's var-list.

Changing the order fixes that collision, but not the underlying inference:
"writes elements of this container" does not specify ordering, padding,
iteration, write conditions, or the command's result. Likewise `VarWrite`
says where a possible write occurs; it does not prove that the write always
happens. `HAS_LOOP_BODY` does not establish list iteration semantics, and a
case-list grammar does not by itself establish first-match dispatch.

Make executable semantics an explicit semantic descriptor, or derive them
only from an existing descriptor that already states the same operation.
`CellReadModifyWrite(Increment)` is substantially stronger evidence than
`ElementsOf`; deriving a value projection from the former is reasonable.
Longer term, let native lowering and analysis read a shared semantic
operation, so runtime optimisation support does not determine which commands
analysis can understand.

Extend the existing
[`ResolvedInvocation` / `InvocationSemantics`](../../../rust/tcl-registry/src/resolved_invocation.rs)
instead of creating a second independent command/subcommand/form resolver.
The current resolver already handles inheritance, form selection, source
word kinds, argument offsets, and instance invocations. A bare
`CommandSpec::value_transfer_for_call(&[&str])` cannot by itself retain all
that evidence. Define one argument coordinate system and an explicit mapping
back to source words and lowered storage places.

Include an explicit "no evaluator for this form" override. With
`Option<ValueTransfer>` meaning "derive when absent", a more specific form
cannot necessarily suppress a parent evaluator. Distinguish inheritance,
an explicit declaration, and an explicit abstention policy.

Acceptance: table-driven resolution tests cover `foreach`, `lmap`,
`lassign`, command/subcommand/form overrides, a pure getter beside a mutating
setter, instance methods, dynamic selectors, and synthetic loop headers.

### R3. Model a transfer as a result and a change to storage

The proposed `CellFoldFn(old, args) -> Option<String>` conflates the new
cell value and command result. That works for `incr`, `append`, and several
dict updates. It is not a universal cell-operation contract: the proposed
`ListPop` family, for example, needs to distinguish removed result values
from the remaining list. `values_from` also assumes one contiguous suffix
can represent all value operands, which does not describe interleaved
key/target layouts or body-taking operations.

`DestructuredFold::writes: Vec<Option<String>>` cannot express "preserve
this target": `None` explicitly widens. This can be conservative, but it
cannot deliver the precision claimed for no-match and partial-conversion
cases. It also leaves write order and aliases unspecified.

Concrete Tcl 9.0.4 and 8.6.17 observations from this review:

~~~tcl
set a before; set b before
regexp {(x)(y)} zz a b       ;# result 0; a and b remain before
scan {12 nope} {%d %d} a b   ;# result 1; a = 12, b remains before
lassign {first second extra} a a
                            ;# result extra; a = second
set d {a 1}
dict with d {incr a; set result done}
                            ;# result done; d = {a 2}
~~~

`dict update` has the same body-result/writeback distinction. Its actual
implementation in [cmd_dict.rs](../../../rust/tcl-vm/src/cmd_dict.rs)
performs binding, body execution, and reconciliation. These are not one
destructuring operation over literal arguments. Initial key binding may be
a useful projection, but it must not stand for the whole command.

Use a normal-outcome record with a separate result and ordered target
updates. Target updates need at least write, preserve, unknown/may-write,
and, when existence modelling is introduced, unbind. Resolve duplicate
names and aliases to storage places before composing updates. Early phases
can abstain when targets overlap, are arrays, or are trace-visible; say so
explicitly.

Distinguish a pending analysis input from an unbound Tcl variable and from
an unsupported evaluation. SCCP `Unknown` is not "variable absent".
Uninitialised `incr`/`append` behaviour needs an existence proof and the
selected release. Do not manufacture an empty old value from bottom.

Partial writes on error are a separate concern. If the framework cannot
represent them, decline the whole execution and retain conservative effects
on every possible successor. Do not publish speculative successful writes
before all required checks complete. The initial API can support only
proven normal completion without introducing pack-authored CFG edges.

There is already a useful concrete result contract to copy rather than
invent: `tcl_cmd_core::regex::RegexpResult::Count` carries an independent
count and `assign: Option<Vec<...>>`, with `None` explicitly meaning leave
match variables untouched. The generic transfer protocol should preserve
that distinction when projecting the existing core's answer.

### R4. Keep value knowledge separate from rewrite and deletion permission

The design says `dict incr`, `lset`, `lassign`, and variable-writing
`regexp`/`scan` join O129 "with no new code". That is not sound if O129
replaces a command substitution with its result:

~~~tcl
set n 1
set result [incr n]
puts $n                    ;# must still print 2
~~~

Knowing that `result` is 2 permits downstream propagation while retaining
the `incr`. Replacing `[incr n]` by `2` removes a required write. This
problem exists even with no traces, no aliasing, and a trusted builtin.
Printing or quoting the result correctly does not solve it.

Similarly, the proposed DCE rule "dead target plus pure argument words"
does not establish that a command can disappear:

~~~tcl
set unused "\{"
lappend unused value       ;# raises, although the stored value is dead
~~~

Purity and referential transparency do not imply totality. Malformed
arguments, absent variables, traces, and non-OK completion are observable.
The current `Statement::Incr` deletion predicate in
[elimination.rs](../../../rust/tcl-compiler/src/optimiser/elimination.rs)
is not sufficient justification for generalising that predicate to every
transfer. This review does not claim a new running DCE regression from the
documentation branch; it rejects the proposed generalisation.

Specify three distinct permissions:

1. Record a proven result/state fact on the normal path.
2. Substitute a later use while preserving the producing operation and its
   binding/trace assumptions.
3. Remove or replace the operation only after proving equivalent effects,
   completion, evaluation order, and result use, including implicit returns.

A computed result must carry the binding evidence that produced it. The
existing `ResolvedConstSubst` retains `command_bindings`; the proposed
`folded_types` side map does not replace that. When constants are propagated
through SSA copies, joins, calls, or branches and then emitted into code,
retain and combine dependencies, or re-prove them before emission.

### R5. Make `expr` the first demanding client of the interface

The project already shares its expression tree walk through
[`tcl_syntax::expr::eval` and `ExprOps`](../../../rust/tcl-syntax/src/expr/eval.rs).
`ExprOps` already has `var`, `command`, and `call` operations. The compiler's
[`FoldOps`](../../../rust/tcl-compiler/src/tcl_expr_eval.rs) supplies an
environment, rejects command substitutions, and calls the shared math
function dispatcher. This is the seam to extend; a new evaluator or an
unchanged compiler-local `Native(Expr)` arm misses it.

Also preserve the expression engine's full value result. The existing
compiler adapter's `eval_with_config` ends with `result.to_number(...)`,
and its public `TclValue` has numeric variants only. Reusing that adapter
unchanged still cannot fold a string-valued expression such as
`expr {"x"}` or the proposal's string-valued ternary. Reuse the generic
engine while extending the analysis result boundary appropriately; do not
force all expression outputs through a numeric-only API.

There are three distinct layers:

1. Tcl word evaluation obtains the actual argument values, under the
   source's braced/quoted/bare/expanded substitution rules.
2. The registry-owned `expr` implementation assembles those arguments into
   an expression as Tcl specifies.
3. The shared expression engine evaluates lazily, using analysis services
   for proven variable values and supported nested calls.

The expression engine must not eagerly execute every syntactically present
substitution. Both tested shells return 0 for
`expr {0 && [error never]}`. Conversely, a quoted argument can execute
substitutions before the expression engine receives it. The probes also
confirm that, with `a = alpha` and `b = beta`, `expr {$a == $b}` returns 0
while `expr "$a == $b"` errors. Resolving words as structured values avoids
building a script by interpolating values that contain Tcl syntax.

Binding validity must include every command implementation actually used.
This is not an author-trust check. Math functions are command bindings too:

~~~tcl
rename ::tcl::mathfunc::abs ::tcl::mathfunc::saved_abs
proc ::tcl::mathfunc::abs {x} {return 99}
expr {abs(-2)}             ;# 99 in both tested shells
~~~

Checking only `expr` or retaining a builtin `abs` in a private VM does
not prove what the analysed program calls. A successful result needs
transitive dependency evidence for math functions and nested commands,
alongside variable observability and the target's numeric/word rules.
Support can initially abstain on user math functions or ambiguous binding.

Use the direct expression engine whenever its supported operations can
calculate the answer. A bounded VM route is useful for an explicitly
declared implementation that needs more Tcl execution, provided its inputs
and dependencies are closed. If `$request_value` is unknown, the VM must
not read a host environment, substitute a dummy, or turn one sampled run
into a proof. Partial abstract reasoning remains the analyser's job.

Acceptance includes multi-argument `expr`, braced versus quoted forms,
short-circuit operators and ternaries, strings that look like code, math
function rebinding, nested pure substitutions, errors, bignums, and target
release ambiguity. Measure direct-expression and VM entries separately.

### R6. Preserve exact values; type is not representation provenance

The design says every evaluator result re-enters via `parse_literal_value`.
That function currently starts with `text.trim()` and also trims the string
fallback in [sccp.rs](../../../rust/tcl-compiler/src/sccp.rs). A computed
value is already a value, not a source token: `string range { a } 0 end`
returns the three-character string ` a `. Passing that output through this
helper would change it to `a`.

Require a value-ingress API that preserves the exact result, including
leading/trailing whitespace, NUL, backslashes, leading-zero numbers, signed
zero, and Unicode. Numeric classification may add facts; it cannot replace
observable string spelling without proof that the value has that canonical
spelling. This is a concrete existing helper hazard exposed by the proposed
new ingress, not a reason to preserve its behaviour as a compatibility test.

A side map is a possible implementation choice, but `TclType` alone is not
enough to justify the claims about lists, byte arrays, or shimmer:

- A list type does not give its elements or eliminate the need to recover
  element facts from an exact value.
- Per-target types in `scan` can differ. One `written_type` cannot describe
  every destructured target.
- A runtime byte-array value, its Tcl string representation, and UTF-8 bytes
  of that representation are distinct. A folded `binary format` result needs
  a lossless conversion/materialisation contract.
- After a branch joins identical strings produced with different internal
  representations, the string may remain exact while representation is
  uncertain. A single last-written side-map type is not a sound join.
- Replacing a byte-array-producing call by a source literal can change
  representation costs even when the Tcl string value is equal. The
  original command's return type does not prove the replacement has its
  runtime representation.

Separate exact value, semantic type/shape, and representation evidence.
Define joins and invalidation for each, and attach source provenance only
where a real mapping exists. The proposal correctly says folded values have
`literal_span: None`, but later promises to paint a folded pattern at its
definition site. Computed characters generally have no such source span;
show a computed-value explanation or retain existing literal mappings rather
than fabricating token ranges or rename edits.

### R7. The hook host needs per-evaluation state and capability rules

The proposal states that determinism follows from removing clock, I/O,
`rand`, and `srand`. That is insufficient. The current
[`HookHost`](../../../rust/tcl-spec-hooks/src/host.rs) retains one engine
per pack, and the
[`sandbox`](../../../rust/tcl-spec-hooks/src/sandbox.rs) permits `incr` and
`set`. A body such as `fold [incr ::counter]` uses no random or I/O command,
but its result depends on earlier invocations. Qualified global variables
do not require the `global` command. Per-pack containment is not
per-invocation purity.

A fixed probe against this worktree's freshly built `tcl-spec-hooks` and
`tcl-registry` libraries confirmed it through the actual host, slot, and
folder thunk. Install
`HookProgram::new("review::counter", HookFamily::ConstFold,
"fold [incr ::counter]")` in `tclvm_host()`, install that host through
`pack_hooks::install_host`, obtain `const_fold_fn` for its slot, and call
the folder twice with `&[]`. The results are `Some("1")` and `Some("2")`.
This uses the default unrestricted input declaration (uncached); the
whitelist and host budgets remain enabled.

Define whether evaluators run in a resettable snapshot, whether writes
outside their local activation are denied, or whether a separate execution
context guarantees that all mutable state is discarded. The dependency and
state-isolation rule must cover multiple hook bodies in the same pack and
multiple analysis threads. A content cache can hide statefulness; it cannot
make the computation pure.

Separate two policies: commands available to implement a hook, and subject
invocations eligible for evaluation. Local `set`, loops, and `lappend` are
necessary implementation facilities although they are not pure commands.
Conversely, an apparently pure subject form may invoke a callback or use
replaceable math functions. Whitelisting an entire ensemble by one form's
purity exposes all its subcommands unless dispatch is also constrained.

Engine execution should be an explicit registry capability, with an
implementation identity, supported target semantics, declared dependencies,
and budgets. The proposed resolver cannot currently derive `Pure` for a
command without a folder, yet Level 2 requires exactly that state. Model
the evaluator route directly instead of making purity serve as both
classification and executable backing.

The current sandbox excludes `source` and `package`, and the hook setup
does not load arbitrary package implementations. "Run the real tcllib/EDA
command" therefore needs a separate, pinned provisioning path. Existing
iRules simulator stubs returning an empty string are not valid evaluators.
Do not load workspace packages as a consequence of merely opening a file;
an explicit supported implementation can be embedded or installed through
the established package policy.

The companion correctly identifies that containment does not establish
semantic truth. A wrong constant can remove branches and suppress taint
findings. "A wrong answer is bounded" and "folds may only widen" need
correction: producing any exact constant narrows abstract state and can
affect the entire downstream graph. The owner accepts author-supplied
semantics for that whole graph: loaded workspace facts may narrow values,
summaries, types and executable edges and support rewrites without an extra
trust gate. A1 records this decision. Do not retain an indirect restriction
through a consumer-specific provenance cap after accepting a fact upstream.

### R8. Define one evaluation context and complete memo dependencies

The proposal correctly identifies the existing split:
[`compilation_unit` and `function_lattice`](../../../rust/tcl-lsp-db/src/lib.rs)
use `db.registry`, while the analyser can receive `spec_pack_key` and an
overlay. Adding the key at the outer query alone is insufficient if the
inner `FnLatticeKey` or registry lookup still drops it.

Carry an immutable analysis context through lowering, unit construction,
per-function queries, optimiser consumers, and evaluator calls. Its identity
must cover the facts that can change the answer: effective registry/overlay
generation, command bindings and namespace context, target semantic profile
and grammar overrides, trace/escape facts, seeds, and any evaluator or
implementation revision not already fixed by the host/registry identity.
Do not duplicate these into uncoordinated keys: use interned identities and
the existing query infrastructure.

At the invocation memo, key the resolved evaluator, exact input values,
incoming target values/existence, and every declared context dependency.
The current [hook cache](../../../rust/tcl-registry/src/pack_hooks.rs) has
no `target-value` input or hash component. A new spelling in the DSL is not
enough; restricted input exposure and cache eligibility must move together.
Review collision handling when a content hash substitutes for full equality
in a proof-bearing cache. Hashing is an index, not evidence that two
different inputs are equal.

The engine host is currently thread-local and can quarantine hooks. Its
availability or mutable health must not silently change the answer to an
otherwise identical memoised query. Either supply a stable evaluator
snapshot for a query, track capability generation, or keep optional
execution results outside canonical proof facts until validated. Test both
host-present and host-absent workers and pack reload/quarantine.

Do not describe the optimiser re-run as repair for a "stale memo". A stale
memo is an invalidation defect. A second run is justified by additional
explicit assumptions, such as a proven method frame, and should be keyed
as that different analysis context. Also keep interprocedural summary
dependencies acyclic: changing `summarise_returns` to invoke a lattice that
itself depends on those summaries needs a staged or explicit fixed-point
protocol, especially for recursion.

### R9. Separate switch grammar, arm selection, and executable edges

The narrow whole-variable `Raw` fix is well motivated. It should reuse the
existing word/variable-name owners and prove that the operand is exactly
one variable reference. Do not make arbitrary `Raw` text executable merely
because one synthetic operand uses that variant.

For opaque switches,
[`lower_opaque_switch`](../../../rust/tcl-compiler/src/cfg_builder/cfg_lower.rs)
stores the structured statement in one block; it does not create a CFG
block for every arm. A post-pass can report arm selections, but cannot
remove nonexistent arm blocks from `executable_blocks`. The promise that
O107 and every reachability consumer automatically gain all switch forms
therefore needs either real lowering support or a distinct structured-arm
fact with explicit consumers.

Registry-owned selection semantics must include ordered first-match
behaviour, final-default rules, fall-through to a later body, regexp capture
writes, option parsing, and match errors. A pattern that never matches can
still supply the body reached through a preceding `-` arm. Deleting that
arm/body pair as O131 can change behaviour. `ConstSet` reasoning must join
selected bodies and writes across every possible subject, and retain error
possibilities if a pattern cannot be evaluated.

A `case_list` description establishes locations and grammar. A private
dispatch-table command gets Tcl switch execution semantics only if its
registry declaration explicitly supplies that semantic contract. Structural
similarity alone is not proof of first-match execution.

Reuse the existing
[`tcl_cmd_core::switch`](../../../rust/tcl-cmd-core/src/switch.rs) option and
selection owner, which already supports exact/glob/regexp selection and
capture-value construction over `RegexEngine`. Do not create a fourth
switch matcher in the registry. Its runtime adapters explicitly retain
fall-through body resolution and body execution; the analysis adapter must
model those remaining steps too.

Land the exact whole-variable case first, then selection facts for opaque
forms, then any CFG integration and edits. O131 is optional presentation
work after those semantics are established.

### R10. Shared cores are the right route, but adapters matter

The direct-core proposal is a good fit for `string`, list/dict value
operations, binary conversion, and numeric updates. The branch overstates
what introducing `ConstOps` alone accomplishes:

- [`ValueOps::char_len`](../../../rust/tcl-syntax/src/value.rs) returns
  `usize`, not a fallible result. It cannot directly implement the proposal's
  "unknown release and disagreeing character counts means abstain" rule.
- [`string::range` and `string::index`](../../../rust/tcl-cmd-core/src/string.rs)
  use Unicode scalar vectors and `index::resolve` directly. An override of
  `char_len` does not change either their indexing model or numeral grammar.
- [`tcl-vm::value_ops::int_add`](../../../rust/tcl-vm/src/value_ops.rs)
  already promotes through `i128` to `BigInt` at the reviewed revision.
  The repeated claim and diagram that it errors past `i64` are stale.
- The runtime value adapters still own coercions, byte access, and result
  construction. Calling the same generic function does not prove that
  different adapters produce the same observable result.

Place release-sensitive checks in the semantic owner or a thin explicit
admissibility adapter. Retain conservative preconditions until the core
supports the necessary target axes. For a release range, compare all
relevant semantic cases or use an established invariance proof; one engine
run at a default release does not establish unanimity.

Apply resource limits to direct evaluators too. Checking output size after
allocating a giant repeated string, exponentiation result, or binary buffer
does not bound memory use. Bound before allocation and charge significant
native work. The VM command counter does not count every inlined bytecode
operation; the engine's own documentation says its wall-clock polling covers
that case. A per-hook 250 ms maximum can still cause a long editor stall
across many distinct invocations. Add a request-wide budget, cancellation,
and per-evaluation caps; resource exhaustion means decline, not an exact
semantic error fact.

### R11. State the abstract transfer contract explicitly

The claim that determinism makes transfers "monotone by construction" is
not a proof. It only establishes repeatability for the same concrete input.
Define the lift: pending inputs remain pending where appropriate; exact
inputs can evaluate; finite sets are joined; unsupported cases become top;
updates and auxiliary facts join with their previous information. No
consumer may re-narrow a value already widened by aliases or traces.

For finite sets, clarify whether "one ConstSet" means one distinct SSA
value or one operand position. Two uses of the same SSA value are
correlated. Repeated targets are correlated too. The conservative first
implementation may decline multiple independently varying values; say that
this is a precision limit, and bound total combinations and result bytes.

An interval transfer cannot simply call `CellFoldFn` on an interval. It
needs a sound abstraction of the operation. For addition, a generic interval
domain can interpret a registry-described integer-add operation with the
target's overflow rules. For operations without an abstract model it should
return top, even if exact concrete evaluation exists. Bounded loop
simulation can reuse the concrete evaluator while keeping separate
termination, trip-count, and side-effect reasoning.

Acceptance includes bottom-to-constant-to-set-to-top progressions, joins in
different predecessor orders, loop backedges, correlated operands, budget
declines, and consistency of value/type/provenance facts. Use deterministic
fixed-input tests in CI; generator-driven campaigns stay manual under the
repository's test-tier policy.

### R12. Untangle the companion's identity and sequencing claims

The companion's separation of description, identity, and executable backing
is useful. Its opening conclusion that both runtimes attest only one command
and lose identity before execution is too broad.

In the VM, `bump_cmd_epoch` clears `guarded_commands`, the specialised
intrinsic guard table. But `command_binding_matches` uses
`builtin_identity_for_key` and `registry_object_roots`, with alias following
and execution-trace checks. Those are distinct mechanisms in
[interp.rs](../../../rust/tcl-vm/src/interp.rs). The ordinary bytecode
binding checks are not demonstrated broken by clearing the intrinsic table.

Distinguish command binding provenance, intrinsic guard eligibility, and
pack/implementation attestation in the text and diagrams. Persisting an
intrinsic guard identity must still invalidate eligibility when dependencies
change; attaching one by name after registration is not proof that a
replacement handler implements the builtin.

The analyser interface and direct registry value evaluators can proceed
without deciding the C extension ABI, producing a WASM engine adapter,
expanding intrinsic dispatch, or adding a universal artefact manifest.
Those are relevant future consumers, not universal prerequisites. The
companion's build order currently puts runtime guard changes before the
core consumer migration, obscuring the shortest route to the owner's goal.

Its conservative extension default also needs care: "may complete with any
code" must retain a possible normal successor. It is not equivalent to
"always terminates this block with no continuation". Use the existing
completion/effect domains rather than inventing a blanket terminator rule.

### R13. Make regexp resource declines distinguishable from semantic answers

Reusing our own regexp engine is the confirmed direction, and inspection of
that engine reveals a prerequisite the original proposal omits.
[`Regex::exec`](../../../rust/tcl-regex/src/lib.rs) and
[`RegexEngine::exec`](../../../rust/tcl-cmd-core/src/regex.rs) return an
`Option` of captures. The adapter cannot distinguish a completed no-match
search from an incomplete search.

In [`exec.rs`](../../../rust/tcl-regex/src/exec.rs), `Bt::m` returns false
when fuel or recursion depth runs out; its comment explicitly says the
search then reports no match. `Matcher::dissect_repeat` also documents a
depth-limit fallback that approximates a subgroup's capture span. These are
direct source observations, and two fixed-input probes reproduced their
observable consequences using this worktree's freshly built `tcl-regex`
library:

| Pattern and subject | Our engine | Tcl 9.0.4 and 8.6.17 |
|---|---|---|
| `^a*(b)\1$` on 300 `a` characters followed by `bb` | `None` (reported as no match) | Match; whole range 0..302 and group 1 range 300..301 |
| `(x)*` on 300 `x` characters | Whole range 0..300; group 1 range 256..300 | Whole range 0..300; group 1 range 299..300 |

Ranges in this table are half-open; Tcl's `-indices` outputs were converted
from inclusive ends. The Rust probe used `Regex::compile_str(pattern,
REG_ADVANCED)` and `exec(&subject_codepoints, 0, 0)`. The independent shell
probes were:

~~~tcl
set subject [string repeat a 300]bb
regexp -indices {^a*(b)\1$} $subject whole capture
# 1; whole = {0 301}; capture = {300 300}
set subject [string repeat x 300]
regexp -indices {(x)*} $subject whole capture
# 1; whole = {0 299}; capture = {299 299}
~~~

Those outcomes cannot become exact compile-time facts. An incomplete search
does not prove a branch false, and an approximate capture cannot safely
become a constant variable value or replacement string. Putting the same
matcher behind a VM does not repair the lost information.

Add a fallible, precision-aware result path in the regexp owner and carry it
through the shared command plumbing: exact match, completed no-match, or
decline/error with a typed reason. Fuel/depth exhaustion and approximate
capture recovery must be visible to analysis. A consumer that needs only
match existence could use a separately certified exact-existence result;
capture consumers require exact captures. Initially, declining the whole
result on any approximation is simpler and sound.

This is a contract change for existing regexp consumers and needs focused
compatibility tests. It is a prerequisite for widening regexp-derived
constants and branch pruning, not a reason to add another regexp engine.
Use deterministic witnesses for matching/capture outcomes and resource
exhaustion; retain fuzz campaigns in the manual tier.

### R14. Make diagnostics consumers of facts, not another semantic engine

The existing architecture has sound foundations. `CompilationUnit` and
per-function lattice queries supply reusable analyses. Compiler checks emit
a protocol-independent `Diagnostic`; `CompilerDiagnostics` retains checks
and optimisation findings independently of display-time optimisation gates.
The server's lifts convert spans and severities, and finalisation applies
tags and severity overrides. These are useful boundaries to strengthen,
not grounds for replacing the pipeline with a new diagnostic framework.

However, three concrete paths show that the separation is incomplete:

1. `emit_provably_unset_w210` recognises `regexp` and `scan` by name, parses
   their forms, and calculates no-match consequences inside a diagnostic
   producer. Embedded conditions have another traversal. Its own comment
   explains that the registry lacks these per-form value semantics. The
   desired owner is the registry transfer plus generic existence/storage
   analysis; W210 should consume the resulting proof. In particular,
   no-match means **preserve**, not intrinsically **unset**: the prior cell
   fact is necessary. This is an end-to-end acceptance test for R3 and the
   confirmed regexp-engine direction, not just a name-dispatch cleanup.
2. `FunctionUnit::build` appends existence-derived constant branches to
   `sccp.constant_branches`, but
   `emit_existence_constant_branch_diagnostics` calls the same semantic
   helper again. The reason is documented: those post-pass facts do not
   update `executable_blocks`, while the ordinary branch emitter requires
   that reachability evidence. Sharing a helper prevents algorithm drift,
   but does not give consumers one complete stored fact. Represent the
   distinction between a proven condition, a selected edge, and applied CFG
   reachability explicitly; diagnostic emission should not rerun the proof
   to recover which kind it was. R9 needs precisely this distinction too.
3. `append_brace_expr_perf_hints` lives in the server and creates O111 by
   searching already lifted diagnostics for W100. W100 has already passed
   analyser filtering. Consequently O111's production depends on another
   diagnostic surviving presentation policy, not independently on an
   unbraced-expression fact. This is visible directly in the call order;
   it was not exercised as an end-to-end configuration probe here. Both
   rules should consume the same semantic/syntax evidence. If the product
   deliberately wants them coupled, encode a shared rule-group policy,
   not a semantic dependency on a displayed warning.

Evidence: [analyser dataflow diagnostics](../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs),
[function construction](../../../rust/tcl-compiler/src/compilation_unit.rs),
[compiler diagnostic envelope](../../../rust/tcl-compiler/src/compiler_checks.rs),
[database queries](../../../rust/tcl-lsp-db/src/lib.rs), and
[server lifting and O111 synthesis](../../../rust/tcl-lsp-server/src/lib.rs).
The latter also contains a good counterexample: encoding abstention uses
byte-decode evidence, explicitly not whether W109 is displayed.

Do not turn this into a rule that every diagnostic predicate needs a new
globally stored lattice. A rule-specific analysis may calculate its finding
from shared facts: the interval diagnostic emitters already delegate to
the canonical interval-bounds owner. The boundary to enforce is that rules
do not reimplement command semantics or make reusable semantic conclusions
available only as a side effect of emitting a warning. Materialise or memoise
reusable facts where multiple consumers need them; keep cheap one-rule
predicates local to their owning checker.

### R15. Partial reduction is not supplied by an exact-value evaluator

The owner's follow-up asks whether this design identifies loss of constant
knowledge and reduces mixed constant/dynamic calculations. Those are
essential capabilities, but not consequences of wiring an evaluator into
SCCP. The former needs per-definition/use facts and loss evidence; the
latter needs partial simplification with operation-specific equivalence
proofs. The dedicated section below specifies both.

There is already relevant machinery in
[optimiser/helpers/expr_simplify.rs](../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs):
constant substitution, fixpoint simplification and O110 reassociation.
[optimiser/propagation.rs](../../../rust/tcl-compiler/src/optimiser/propagation.rs)
also substitutes known operands and attempts further simplification.
Extend and connect these owners instead of adding a second partial
expression evaluator inside the registry transfer driver.

However, the existing helper cannot be taken as the correctness baseline.
A direct probe linked this worktree's compiled `tcl-compiler` and called
`instcombine_expr_typed` with `Some(&OperandTypes::default())`: a real,
empty type-proof context, not its legacy aggressive `None` mode. It returned
`$x + 3` for `$x + 1 + 2`, with `changed = true`. `reassociate_node` does
not receive the numeric context; preserving non-constant terms protects
some coercion errors but does not establish associativity for doubles.

Both independent Tcl oracles show the semantic difference:

~~~tcl
set x 10000000000000000.0
expr {$x + 1 + 2}  ;# 10000000000000002.0
expr {$x + 3}      ;# 10000000000000004.0
~~~

This reproduces the helper's rewrite and the target-language inequivalence,
not a complete LSP code-action/application test. The production expression
pass calls this helper, and the proposal currently lists O110 as merely
benefiting from better types. Reassociation must actually consume the
relevant type/target proofs; supplying a richer lattice to neighbouring
rewrites is not enough. Integer-only reasoning must still account for
target overflow/bignum behaviour, errors, effects and evaluation order.

### R16. Require multi-domain integration and distinct iteration/target contracts

The existing `return_type_for_call` and registry-owned dynamic algorithms
already serve type inference, sanitiser queries and representation checks.
They must feed the new interface, preserving authoritative unknown answers
and the distinction between semantic type and guaranteed intrep. A failed
exact-value query must not erase these independently available facts.
Per-subcommand hooks and expansion-aware resolved operands are natural
extensions of this existing owner, not reasons to create another type table.

The concrete EDA counterexample is `foreach_in_collection` in the bundled
SDC pack: it currently declares `LOOP_LIST_HEADER` and dispatches to the
native `Foreach` analyser hook. That handler does more than walk a body:
it can split a braced literal iterable as a Tcl list and simulate selected
definition effects. Vendor collection iteration needs its own declared
protocol and opaque collection/object facts. A constant collection handle
does not prove its contents, cardinality, order or stability. Analyse its
body with typed unknown yields without requiring a vendor interpreter.

The BPF counterexample is even stronger: BPF-Tcl is a distinct statically
typed language, not Tcl running on a different deployment target. Its
signed division truncates towards zero (`-7 / 2` is `-3`), whereas Tcl's
integer division floors (`-4`). Shared syntax/engine infrastructure needs
the correct language-semantic adapter. Do not use a Tcl VM result as a BPF
constant, and do not let calculability imply subset acceptance or packet,
map, pointer, stack, completion or capability safety.

The [authoring supplement](value-transfers-authoring-review.md) specifies
the multi-domain interface, construction-versus-solving boundary, existing
hook integration, EDA iteration protocol, existing `BpfOpSpec` integration
and worked proposed Rust/SpecTcl declarations. The
[coverage ledger](value-transfers-coverage-review.md) maps every numbered
code to its required evidence and migration/proof obligations. It also
identifies TLS report-side grade/finding policy and stale explorer taint
code spellings as additional centralisation boundaries outside SCCP.

## Proposed architecture

### Regexp evaluation: confirmed owner direction

The owner explicitly requested reuse of our own regexp engine. Route
concrete matching through
[`tcl_regex::cmd_core::AreEngine`](../../../rust/tcl-regex/src/cmd_core.rs),
with the existing
[`tcl_cmd_core::regex`](../../../rust/tcl-cmd-core/src/regex.rs) command
plumbing where applicable. The registry already depends on `tcl-regex` with
its `cmd-core` feature, and its
[`regsub` folder](../../../rust/tcl-registry/src/commands/tcl/regsub_.rs)
already uses this route. This is an existing integration seam, not a new
regexp implementation proposal.

Use it for supported `regexp` and `regsub` value calculations, regexp-mode
switch selection, `lsearch -regexp`, and expression/dialect operations that
need a concrete regexp match. Registry-owned specialisations describe the
command/form and map results to the generic fact protocol; the shared
plumbing owns command algorithms it already implements; the regexp engine
owns ARE syntax and matching. The analyser should not gain another regexp
parser or matcher, and invoking a VM is unnecessary just to match two known
values.

Carry match options and target character/index semantics explicitly.
`-inline` returns a list without writes; ordinary no-match preserves match
variables; unmatched subgroups on a successful match have their own empty
string or index result; `-all` and zero-length matches require the core's
progress rules. A regexp-based branch fact and a regexp output-variable
transfer should derive from the same evaluated match outcome. `regsub
-command` additionally executes a callback and therefore needs a declared,
supported execution route or abstention.

Cache compiled patterns by exact pattern, flags, and any relevant semantic
profile/engine identity, with a bounded cache. Bound pattern compilation and
matching as well as returned captures; a command-level deadline does not
necessarily interrupt one long native match. If resource exhaustion is
reported, preserve it as an evaluation decline, never as "no match".
Validate capture indices, Unicode, flags, malformed patterns, substitutions,
no-match writes, and target-release differences against C Tcl. Reusing our
engine is the ownership decision; compatibility remains an evidence question.

Static pattern diagnostics such as ReDoS analysis are a different question
from matching one concrete subject. They should consume the appropriate
shared pattern structure where available, without treating a successful
bounded example match as proof about all possible subjects.

### Interface and dependency direction

The following is an interface sketch, not proposed public Rust syntax or a
request for another new crate. Fit it into existing owners first.

~~~text
Resolved invocation + immutable analysis context
                    |
          registry-owned semantic plan
                    |
        generic analyser transfer driver
          |             |             |
     direct core   expression engine   declared VM implementation
          |             |             |
          +--- evaluated outcome + dependencies ---+
                    |
       validate and join result / storage facts
                    |
       diagnostics, optimiser, editor consumers
~~~

| Owner | Responsibility | Must not acquire |
|---|---|---|
| Registry | Invocation semantics; specialisation implementations; operand/target selection; evaluator route; stable semantic identity | SSA internals, LSP ranges, mutable analyser access, command handlers copied from shared cores |
| Shared syntax/numeric/command cores | Parsing and exact value algorithms under explicit semantic policy | Registry lookup or analyser policy |
| Analyser/compiler fact producers | Scope and place resolution; input facts; solver; effects/completion validation; joins; source provenance | Command-specific argument grammars or arithmetic selected by name/command ID; diagnostic enable flags |
| Diagnostic rules and edit planners | Interpret shared facts; select codes and explanations; validate and anchor suggested edits | Private command evaluation, duplicated semantic proofs, dependence on displayed diagnostics |
| Frontend presentation | Projection, suppression, severity overrides, protocol ranges and edit serialisation | New command semantics or facts inferred from diagnostic messages/codes |
| Engine adapter | Bounded execution of a declared implementation with structured values and fixed context | Ambient workspace execution or guesses for unknown inputs |
| Composition root | Install evaluator services and immutable registry context for native, browser, CLI, MCP, and workers | Hidden per-thread changes to canonical query meaning |

The dependency direction already supports direct reuse: `tcl-registry`
depends on `tcl-syntax` and `tcl-cmd-core`. `tcl-engine-tclvm` depends on
`tcl-compiler`, so adding that engine as a registry dependency would create
a cycle. Keep the execution interface below the compiler, with the concrete
engine injected by a higher composition root. Existing hook installation
demonstrates the direction, but its mutable thread-local availability needs
the R8 contract before it becomes a canonical lattice input.

A useful result protocol has three outcomes:

- **Pending:** an analysis input has not reached a usable fact yet.
- **Declined(reason):** unsupported input, context, identity, target
  semantics, resource limit, or unknown effect; no speculative exact fact.
- **Evaluated:** a normal result, ordered storage updates, semantic
  type/shape facts, and dependency/provenance evidence.

Completion/effect proof is validated through the existing domains, not
inferred from the presence of an evaluator. Additional completion outcomes
can be designed later without pretending an interpreter error is a value.

Examples of registry-owned plans:

| Invocation | Plan | Execution needed |
|---|---|---|
| `string range S I J` | Resolve value operands; apply target-aware shared string/index owners; return exact value | Direct calculation |
| `incr N A` | Resolve cell and amount; read proven old value/existence; apply shared numeric owner; return new value and write | Direct calculation |
| `expr {E}` | Assemble expression arguments; invoke shared expression evaluator with restricted lazy input services | Expression engine; VM only for explicitly supported extra execution |
| `scan S F A B` | Parse format using its owner; compute partial conversions; return count plus ordered write/preserve outcomes | Shared core if supported |
| `regexp P S A B` | Use the shared regexp command core over our ARE engine; return count/captures and write/preserve outcomes | Direct engine call, without VM setup |
| Private Tcl helper | Resolve its declared implementation and dependencies; invoke with structured arguments | Bounded VM when explicitly declared |
| `dict with D BODY` | Describe binding and reconciliation around a generic body-analysis operation | Structured analysis; concrete execution only for a fully supported closed case |

The API is successful when a command spec chooses these plans while the
consumer stays unchanged. Avoid building an unrestricted second programming
language inside descriptors. Use small declarative operations for common
cases, registry-owned native functions for irreducible algorithms, and
SpecTcl bodies where authoring an executable value calculation is useful.

## End-to-end integration and diagnostic separation

The answer to "is this solidly linked to all aspects?" is **not yet**.
The proposals identify many relevant consumers and therefore provide a good
migration inventory. They do not yet specify the input/output, availability,
invalidation, and evidence contracts that make those consumers agree. The
answer to "have we got a great separation?" is **a useful foundation with
specific leaks**, rather than either a clean separation or a wholly mixed
architecture.

### Extend the existing contracts, do not create competing owners

Make the implementation update
[pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md),
[downstream-pass-contracts.md](downstream-pass-contracts.md),
[diagnostics-integration.md](diagnostics-integration.md), and
[diagnostics-calculation.md](diagnostics-calculation.md) together with the
transfer contract. Each new fact needs one producer, an explicit list of
consumers, its context dependencies, and what consumers must do when it is
unavailable. Merely linking those documents is insufficient; their affected
rows and invariants must change in the implementing slice.

| Pipeline boundary | Required connection | Failure the contract must exclude |
|---|---|---|
| Syntax, segmentation and invocation resolution | Preserve source words/expansion and canonical binding evidence; use the same registry overlay and target grammar | Evaluating reconstructed text with changed substitution semantics, or diagnosing a different command from the one lowered |
| Analyser walk and scope/definition discovery | Registry supplies operand, binding and body plans; analyser applies generic scope/place operations | Moving value folding while leaving command-specific body/target knowledge in the walker |
| Lowering, CFG and SSA | Translate supported plans into structured IR; retain opaque calls when proof is insufficient; preserve ordered reads/writes | Treating an arm-selection observation as an edge that the CFG does not contain |
| Value, existence, type and shape | Share result and storage outcomes, joins, completion and provenance; distinguish pending/unknown from absent | W210 contradicting propagation, or a scalar command result being assigned to every target |
| Effects, aliases, traces and memory facts | Feed invalidation and rewrite admissibility independently of known values | Deleting a known-result operation that writes, throws, invokes a trace, or observes mutable state |
| Intervals, taint and representation | Consume appropriate domains and existing semantic owners; specify any separate abstract transfer | Assuming a concrete folder also proves ranges, sanitisation, or internal representation |
| Interprocedural and workspace analysis | Carry summary dependencies and settled binding evidence across files and recursive calls | Per-file facts surviving a relevant procedure, import, registry or trace change |
| Optimiser and code generation | Consume validated facts and separate transformation proofs; preserve runtime fallback and backing contracts | Promoting a diagnostic observation or evaluator success into unconditional code replacement |
| Diagnostic checks and code actions | Consume the same facts, with rule-specific evidence and independently validated edits | Re-evaluating commands for warnings or manufacturing a fix from an informational finding |
| Incremental DB, CLI, LSP and MCP | Equivalent semantic contexts and results for supported entry points; presentation outside semantic keys | Memoised and direct paths disagreeing, or an overlay influencing one consumer but not another |
| Lightweight editor features | Request only the facts needed for their feature/tier | Every token, symbol or completion request starting a VM or full interprocedural solve |

This does not mean every domain must move into one giant lattice. Taint,
effects, existence, intervals and concrete values have different meanings
and convergence rules. They need explicit communication and common identity,
not an untyped bag of facts or an evaluator that pretends to own all of them.

### The direction of dependence

~~~text
registry plans + shared engines + immutable analysis context
                           |
                 owned semantic facts
                   /               \
       diagnostic rules       optimisation/codegen proofs
                   |               |
             typed findings    validated edit/transform plans
                   \               /
              frontend selection and presentation
~~~

Five practical rules make that boundary testable:

1. **Display policy cannot change semantic truth.** Disabling W210, W100,
   I230 or the optimiser UI must not change values, storage/existence facts,
   reachability or summaries consumed elsewhere. A disabled expensive rule
   can skip its own private calculation, but cannot withhold a shared fact
   required by an enabled consumer. Separate semantic context inputs from
   diagnostic/style settings even if an existing configuration object
   initially carries both.
2. **Facts carry evidence; findings carry policy.** A branch fact identifies
   the condition, proof context and edge/reachability status. A rule chooses
   I230, I231 or an optimisation finding and a useful explanation. Existing
   O100 conversion and analyser branch diagnostics already use related
   facts; define their intended coexistence instead of making accidental
   duplicate suppression the specification. Value-transfer specialisations
   do not choose warning severity, diagnostic text or editor ranges.
3. **A diagnostic is not a rewrite certificate.** An unreachable-arm
   observation may be useful without a safely editable source region.
   Edit planning must additionally check original syntax, comments,
   substitutions, effects, source revision and exact replacement range.
   Keep the existing distinction between hints and actionable replacements;
   never give a fabricated span to a value merely to obtain a code action.
4. **Aggregation does not recover semantics.** Lifts may filter, map ranges,
   attach tags and serialise edits. Workspace resolution/refinement belongs
   to its semantic owner and supplies settled evidence to rules. The final
   publication path must not parse messages, evaluate expressions, or infer
   semantic facts from whether another diagnostic survived suppression.
   Keep intentional overlap policy, such as W110/O120 precedence, explicit
   and separate from fact production.
5. **Availability and revision are part of the contract.** A fast-tier
   request can lack a deep fact without that fact being false. Keep the
   existing workspace-refinable diagnostic policy and lightweight structure
   queries. A result or edit from an old source/context revision must not be
   attached to the current document; canonical relative spans must be
   rebased using the source mapping owner, not guessed by consumers.

The last rule matters for performance as well as correctness.
`file_token_facts` deliberately uses structure-only analysis: its source
documents the reduction from roughly 19 seconds to 2.9 seconds over an
883-file workload when deep diagnostic work was removed. Those are existing
source-reported measurements, not fresh measurements made for this review.
"All consumers share the same semantic contracts" must not become "all
consumers eagerly compute every semantic fact".

The minimum convincing vertical slice is therefore not just a folded value.
For one registry-declared command, demonstrate the value/storage fact, the
corresponding existence/type consumer, an appropriate diagnostic, a safely
declined or validated rewrite, and identical behaviour through incremental
and direct analysis. Then disable the diagnostic and prove the other facts
are unchanged. `regexp` no-match and partial `scan` are especially revealing
tests because a superficial return-value folder cannot satisfy them.

## Constant-to-dynamic transitions and partial reduction

**The direction supports these capabilities, but the current design does
not yet provide everything needed.** Make them explicit interface and
integration requirements, not incidental improvements expected from more
folders. Also define "dynamic" carefully: inability to prove a value is
constant does not prove that it varies at runtime.

### Track knowledge at definitions and uses, not on a variable name forever

The existing [SCCP vocabulary](../../../rust/tcl-compiler/src/analyses.rs)
already distinguishes `Unknown`, `Const`, `ConstSet` and `Overdefined`.
Preserve those distinctions:

| Knowledge | Meaning | What must not be inferred |
|---|---|---|
| `Unknown` / pending | The solver has not obtained usable input evidence yet | Runtime dependence, missing variable, or a reason to erase a previously joined fact |
| `Const(v)` | One exact value under the recorded assumptions | Permission to delete its producer, or permanent immutability of the source variable |
| `ConstSet(S)` | A bounded collection of possible values | An arbitrary chosen member, correlation with another independent set, or ordered loop iterations |
| `Overdefined` / no exact value | This domain cannot represent a sufficiently precise value | Proof that no future assignment can be constant, or that all other domains know nothing |
| Evaluation decline | This execution route could not establish an answer, for a recorded reason | A Tcl error, no-match, missing cell, or inherently dynamic program behaviour |

For example, consider a local variable with no aliases, traces or external
observability:

~~~tcl
set x 3
incr x 2
set x $request_value
set x 7
~~~

The successive definitions can have exact values 3, then 5, then an
unproven value, then 7. The unknown input does not retroactively invalidate
the earlier definitions. The final assignment creates a new constant
definition; it does not violate monotonic convergence of the earlier SSA
value. At a branch join, equal incoming constants can stay constant,
different bounded constants can form a `ConstSet`, and an opaque incoming
value forces a less precise join. A later source edit starts a new analysis
snapshot rather than reversing a solver update in the old snapshot.

An alias write, trace, callback, namespace mutation or opaque call may
invalidate knowledge without an explicit `set x`. The value-loss fact must
use the existing place/effect/observability owners. With today's
whole-function conservative barriers, precision can be lost earlier than
the actual runtime mutation: the design cannot promise the exact transition
point in every dynamic Tcl program without more flow-sensitive evidence.

Provide a query over the SSA use or storage place **at the requested program
point**, returning the value-domain answer plus relevant type/shape/range,
dependencies and bounded explanation evidence. Record why a fact ceased to
be exact: dynamic input, conflicting incoming values, alias write, trace,
binding uncertainty, unsupported semantics, release ambiguity or resource
decline. Link that evidence to the responsible statement or incoming edge
where known. Explorer explanations and diagnostic rules can consume it;
they must not reconstruct it from warning messages or compare two arbitrary
solver iterations. A permanent append-only log of every solver event is
neither required nor desirable.

### Separate exact evaluation, residual simplification and algebraic regrouping

These are three different operations:

1. **Exact evaluation:** every required input is proven, so the shared core,
   expression engine or declared VM implementation computes a full result.
2. **Partial simplification:** evaluate proven closed subexpressions, retain
   unresolved operands and their evaluation structure, and return a simpler
   residual expression. The whole expression remains non-constant.
3. **Algebraic regrouping:** move/combine constants across unresolved terms
   only when the operation, operand domains and target semantics justify
   the equivalence. This requires more proof than the first two.

| Input | Useful reduction | Required justification |
|---|---|---|
| `expr {2 + 3 + $x}` | `expr {5 + $x}` | Fold the already closed `2 + 3` subtree; preserve the remaining operation |
| `expr {$x + 2 + 3}` | `expr {$x + 5}` only when proven safe | Reassociation across an unknown term; not valid for arbitrary doubles |
| `expr {(2 + 3) * ($x + (4 + 5))}` | `expr {5 * ($x + 9)}` | Fold two independent closed subtrees without regrouping the dynamic calculation |
| `expr {$x * 0}` | Usually retain it without further proof | Dropping `$x` can remove errors/effects or alter numeric result semantics |
| `string cat {prefix:} {abc} $x {:} {suffix}` | Merge the two constant runs around `$x` | Registry-declared concatenation semantics; preserve exact strings, word evaluation and quoting |

Do not replace the dynamic operand with a dummy and run the VM. That
produces one sample, not a symbolic result. Reuse the shared expression
parser and exact evaluator for closed subtrees; make the partial rewriter
consume the same semantic facts and return a typed residual plus its
dependencies and transformation proof. Keep it distinct from the evaluator
outcome: `Residual(expr)` is not `Const(rendered_expr)`.

Registry-owned command semantics should expose the supported generic
operation or specialisation plan. Thus a concatenation command can use a
generic segment simplifier without making the analyser recognise its name.
Purity does not imply associativity or a valid identity element. Arithmetic
laws belong with the expression/numeric semantic owners, with target and
type proof obligations; they should not be copied into each command spec.

### Preserve useful partial facts after the exact value is lost

A non-constant integer can still have an interval. A non-constant list can
have a known length and element type. A string assembled around an unknown
segment can have an exact prefix/suffix or a length bound. Those facts can
support diagnostics and further reductions without pretending the whole
value is known. Conversely, a constant textual result does not prove a
particular internal representation or absence of effects.

Use the existing domains where they suffice; add bounded segment/residual
facts only for concrete consumers that need them. Avoid putting an
unbounded symbolic expression tree into every SCCP entry. Specify caps,
joins, dependency invalidation and loop widening; on complexity exhaustion,
drop residual precision without changing semantic truth. Cross-statement
reductions must additionally respect storage versions and effect barriers,
not merely recognise a sequence of syntactically constant operands.

Acceptance tests must show constant → unknown → newly constant definitions,
branch/loop joins, alias and trace invalidation, calculation declines,
known type/shape surviving value loss, closed-subtree reduction, safe integer
regrouping, unsafe floating-point regrouping refused, and mixed string runs
with Tcl metacharacters preserved. Compare original and reduced programs
for values, completion and effects, not only the generated source text.

## Adversarial follow-up: try to break the architecture

This second pass was requested explicitly after the initial review. It
attacks both the branch and this review's recommended replacement. The
earlier findings are not softened by the fact that the branch is only a
proposal: these are conditions under which the proposed implementation
would miscalculate, suppress a real warning, cross an ownership boundary,
or fail to deliver its claimed coverage.

The table distinguishes **observed** evidence from **design attacks**.
An attack identifies a negative test, missing contract or explicitly
accepted risk, not a claim that an unimplemented API has already exhibited
a runtime vulnerability. A1 records the owner's accepted author-responsibility
policy, not an implementation blocker.

| Attack | Main boundary | Evidence/status | Consequence if unanswered |
|---|---|---|---|
| A1. Return a false but well-formed constant | Author responsibility versus analyser correctness | Owner accepts author-supplied semantics without an extra trust gate | Incorrect facts can hide findings or misoptimise code; the pack author owns that risk |
| A2. Supply mutually inconsistent descriptors | Central resolution across axes | Concrete derivation collision in R2; broader conflict attack | Values, effects, target writes and lowering describe different invocations |
| A3. Mutate an input during expression evaluation | Evaluation versus abstract state | Independent Tcl probes below | Lazy evaluation still produces the wrong result if callbacks read a frozen environment |
| A4. Fail after a successful output write | Completion versus storage | Independent Tcl probe below | An error path incorrectly retains the old value or loses a real write |
| A5. Return a structurally invalid semantic plan | Registry versus analyser authority | Design attack against the proposed replacement too | Out-of-bounds indices, alias corruption, invented facts or partially applied analysis updates |
| A6. Stay below every individual budget | Evaluation versus editor scheduling | Cost construction; current host limits inspected | Many individually valid evaluations monopolise a document update |
| A7. Change bindings after a proof was obtained | Static evidence versus runtime lifetime | Design attack; R5's math-function rebinding is observed | An admission-time identity check is mistaken for indefinite validity |
| A8. Create a cyclic query through evaluation | Shared services versus dependency direction | Design attack against both proposals | Re-entrant SCCP, recursive host construction, or input-order-dependent facts |
| A9. Remove diagnostic-only command semantics | Analysis versus diagnostic generation | R14 observed; existing good registry interfaces inspected | Migration either leaves duplicate evaluators or moves all diagnostics into the registry |
| A10. Claim release support by passing a version flag | Shared implementation versus target fidelity | R10/R13 observed adapter/engine limits | The same incorrect algorithm is consistently used everywhere |
| A11. Read a correlated or unavailable fact as exact | Domain precision and tier integration | Correlated-loop Tcl witness; R9/R14 source evidence | Impossible states enter summaries or an absent fact becomes a negative proof |
| A12. Add a new command with no consumer changes | Extensibility and design completeness | Acceptance experiment still missing | Centralisation is achieved only for today's command catalogue |

### A1. Accept author-supplied semantics without a second trust gate

Suppose a private command reads request state, but its pack declares a pure
implementation that always folds to `0`. Every structural check can pass;
the hook can be deterministic, tiny, and perfectly sandboxed. If its result
controls an `if`, the analyser can mark the live arm unreachable and stop
reporting taint there. No runtime artefact is needed, so runtime attestation
does not protect this use.

The owner explicitly accepts that risk: authors implementing analysis facts
in their own environment are responsible for their correctness. Once a pack
is loaded, its well-formed semantic declarations are authoritative inputs
to analysis and optimisation. This applies to constants, effects, aliases,
return types, sanitiser claims and body descriptions alike. There is no
advisory-only tier, separate approval to narrow facts, or extra permission
to prune executable edges, suppress unreachable findings or eliminate code.

This decision supersedes the companion's proposed restrictions that pack
folds may only widen or that workspace provenance bars them from feeding
`executable_blocks`. Do not implement those restrictions as a security
floor, require a pinned-pack opt-in, or require an oracle certificate before
using a workspace author's facts. Provenance remains useful for explanation,
binding selection, dependencies and invalidation, not authority ranking.

Keep three independent engineering contracts: apply the declaration to the
resolved binding it describes; validate the API's structural invariants;
and bound/isolate evaluator execution so analysis remains deterministic and
responsive. None proves a model's semantic truth, and none should become
a disguised author-trust gate. A well-formed but false constant is accepted
under the declared model; a malformed target index is still an API error.
Tests and independent oracles remain quality tools for implementations the
project ships, not prerequisites for users to author their own semantics.

### A2. One query per axis can still produce contradictory answers

Attack with a command whose base declaration supplies a pure folder, whose
selected form writes a variable, and whose subcommand declares a different
cell operation. Add an overlay that changes argument roles without changing
the inherited transfer. Each field can be individually legal; separate
resolvers can still assemble a semantically impossible combination.

Resolve the invocation's binding, argv shape, selected form, target profile
and overlay once into a common immutable identity. Axis queries may remain
separate, but must interpret that same selection and obey documented
inheritance, explicit-disable and conflict rules. Validate cross-field
requirements where expressible: a cell transfer needs its matching
read/write layout; a body plan must identify real body operands; a declared
no-write effect cannot coexist with an applied write plan.

Reject or quarantine an inconsistent specialisation with an author-facing
reason and retain conservative generic behaviour. Do not silently choose
the most specific-looking field, or let the optimiser and diagnostics pick
different fallbacks. Structural validation catches contradictory metadata;
it cannot prove an otherwise consistent declaration is truthful (A1).

### A3. Lazy callbacks need an explicit state model

Both tested Tcl releases produce the following results:

~~~tcl
set x 1
expr {$x + [incr x] + $x}    ;# result 5, x becomes 2
set x 1
expr {0 && [incr x]}         ;# result 0, x remains 1
set x 1
expr {$x + [set x 10] + $x}  ;# result 21, x becomes 10
~~~

A callback that obtains every variable from the same incoming SCCP map
cannot model the first and third expressions once it permits stateful
nested commands. A callback that evaluates all substitutions first also
fails, including on the short-circuited second expression. Merely sharing
the expression tree walk does not supply the missing temporal semantics.

The first slice should explicitly accept only nested operations whose
effects cannot invalidate the observed inputs, and decline other cases.
Later support needs a generic ordered evaluation state: reads see prior
validated writes, nested outcomes carry those writes and binding effects,
and expression control flow decides which callbacks run. This state is not
the mutable analyser or the real program's interpreter. A registry-owned
`expr` specialisation chooses expression semantics; the generic services
own storage, dependency and effect application.

There is a legitimate scope choice here: rejecting stateful substitutions
initially is solid. Claiming general expression evaluation while using
read-only callbacks for them is not. This also tightens the replacement
API sketch in this review: its read-only services are sufficient for the
restricted slice, not a complete evaluator of effectful Tcl expressions.

### A4. An interpreter error is not an atomic rollback

The follow-up probe under Tcl 9.0.4 and 8.6.17 was:

~~~tcl
set a old
array set b {k keep}
catch {lassign {new second} a b} message
# code 1; a is now "new"; b(k) is still "keep"
~~~

The first target was written before the scalar assignment to the second
target failed. A transfer that returns `None` on error and simply preserves
all incoming state would be wrong if that state reaches a handler. A
transfer that treats all writes as successful would also be wrong.

Keeping `catch` opaque and invalidating affected facts conservatively is a
valid initial boundary. If structured exception/body analysis is added,
storage outcomes must be indexed by completion path and retain write order.
No exact facts may escape an unsupported path. The earlier recommendation
to apply a **validated analysis result** atomically does not mean the
**analysed Tcl command** has transactional runtime semantics: those are
different operations, and the contract must say so.

### A5. A read-only interface does not make returned plans trustworthy

Try returning a write to an operand that is not a permitted target, an
incorrect output count, duplicate writes disguised as independent targets,
a scope referring to a nonexistent body, or a precise value with an
incompatible type/representation claim. An extension does not need
`&mut Analyser` to corrupt analysis if the consumer blindly applies its
returned plan.

Validate the complete response before publishing any fact. Use invocation-
scoped operand/place handles where useful; check ordering, cardinality,
indices, overlap, permitted effects, dependency identity and value limits.
Specify explicit join behaviour for conflicting repeated targets. General
variadic positions also need a checked range policy: the proposal's
`Vec<u8>` targets and `u8` value index cannot describe arbitrary-length Tcl
argv. Use an adequate index type or decline on overflow, never truncate.

Malformed plans are pack/implementation failures, distinct from ordinary
unknown program inputs. They should produce an actionable load/evaluation
notice and a conservative result, not an apparently valid partial lattice
update. Slot exhaustion and hook reload must likewise be visible; the
current fixed-slot machinery cannot silently become the definition of
supported catalogue size.

### A6. Per-call limits do not establish interactive cost

The existing host has command, wall-clock and maximum-value-size limits.
Those are useful containment. They do not make "cost is bounded by
construction" an adequate LSP performance contract: 10,000 cache misses at
20 ms each cost 200 seconds, while every invocation remains far below the
250 ms individual limit. This is a cost construction, not a benchmark.

Budget the document/request and solver iteration as well as the individual
evaluation. Charge direct cores, regexp compilation, nested expression
callbacks, VM calls, result construction and cache growth to that budget.
Cancellation must reach expensive native operations; checking only at Tcl
command boundaries does not by itself bound one long-running core call.
Limit aggregate retained memory, not only the largest individual value.

On cancellation or budget exhaustion, return sound incomplete analysis,
never an exact negative answer. Distinguish deterministic unsupported cases
from transient scheduler/host failures in caches. A transient failure must
not permanently disable future precision; a retry must not mutate an
already published snapshot's meaning. Test warm/cold caches, deliberate
cache churn, many packs and worker migration, not only a fast repeated hook.

### A7. Prove how long identity evidence remains valid

Attest the subject implementation and every dependency actually used. Then
ask what can mutate them between the proof and the operation: callbacks,
command-table changes, namespace resolution, aliases, package reload, or
traces. Admission-time checks and runtime fast-path guards are different
mechanisms; neither should be described as protecting the other without a
specific dominance/lifetime argument.

Conservative whole-module rejection of opaque mutation is acceptable for
analysis. More precise execution needs a documented guard/invalidation
strategy covering each transformed use. Source edits deserve particular
care: once a user applies a constant replacement there is no accompanying
runtime guard unless the edit explicitly emits one. Therefore guardable
runtime optimisation is not automatically a safe source code action.

### A8. A dependency DAG is also an execution constraint

Attack the callback graph: SCCP invokes an expression; its command callback
asks for the same canonical function lattice; computing that lattice
requires the same expression. Or constructing a VM hook compiles its body
using the very pack evaluators that the host is still installing.

The Rust crate graph can remain acyclic while these dynamic dependencies
recurse. Callbacks must read the supplied current analysis state or invoke
a separately specified nested evaluation protocol, not recursively demand
the completed query being computed. Use the existing solver's pending and
recursion rules. Define an installation/bootstrap mode independent of the
uninstalled evaluator, and keep the host's execution realm distinct from
the subject program's proven bindings. Reject unsupported cycles with a
typed reason rather than hoping a depth cap makes the answer meaningful.

### A9. Centralise semantic validation, not all diagnostic policy

The repository already has a better model than either extreme:
[`LiteralArgumentValidator`](../../../rust/tcl-registry/src/literal_validation.rs)
returns `Valid`, `Invalid(LiteralArgumentIssue)` or `Abstain`, with an
argument index and semantic reason independent of diagnostic/LSP types.
[`ConstraintReport`](../../../rust/tcl-registry/src/spec.rs) lets a hook
report a slot and explanation while the analyser owns code/span policy.
The latter deliberately permits authored message text; the value-transfer
boundary must not accidentally outlaw that existing diagnostic-extension
surface or force it into an unrelated evaluator API.

Extend these patterns: command-specific validity and operand relationships
remain registry-owned, shared grammar algorithms stay in their owners, and
generic checkers turn typed violations into findings. A value query should
not emit W146 as a side effect, and disabling W146 must not prevent another
consumer learning the operand's value. When a constant-derived argument
can be validated, feed its exact value with honest provenance through the
validation interface; do not relabel its original source token "literal".

There is also an architectural dependency that the blanket rule "all rules
live in the analyser" would miss: a newly declared command-specific
constraint must not require a new diagnostic command-name arm. Registry
authorship of the constraint and consumer ownership of diagnostic policy
are complementary, not competing centralisation claims.

### A10. Shared execution is an owner decision, not a compatibility proof

Passing `TclVersion` does not make a VM implement that release's semantics.
For each route, state supported numeric syntax, character/index model,
regexp features and limits, binary representation, platform behaviour and
completion semantics. Unsupported combinations decline. The concrete
regexp defects in R13 demonstrate why agreement between our compiler and
runtime can merely mean both share a bug.

Keep independent target oracles and adapter-level tests. Missing target
facts cannot fall back to the review machine's OS or the installed VM's
default profile. Crucially, this is not an argument against using our own
regexp engine: it is the acceptance standard for making that engine a
source of proof-bearing compiler facts.

### A11. Exact values do not eliminate domain and control-flow limits

This fixed loop produces `x = 20` in both tested shells:

~~~tcl
set x 0
foreach {a b} {1 10 2 20} {
    incr x [expr {$b / $a}]
}
~~~

The pairs are `(1,10)` and `(2,20)`. Independent sets `{1,2}` and `{10,20}`
lose their relationship. A Cartesian abstraction is conservative but less
precise; arbitrarily pairing members can be unsound. An exact loop result
needs ordered iteration simulation or a relational fact, not just a
`ConstSet` for each loop variable. The proposal's refusal to multiply two
sets is safe; it must also constrain the advertised loop and summary gains.

Likewise, there is no universal meaning for an empty reachable-block set
outside its producing analysis contract. Missing/deferred analysis, no
normal successor and a proved unreachable branch are different. Require
typed availability and edge evidence across fast/deep tiers, exception
paths and summaries. Enlarging the value lattice must not silently change
the interpretation of those other domains.

### A12. The catalogue-extension experiment is the completeness test

Do not count migrated command names as the final proof. Author one private
command with a supported existing transfer/validation/body pattern, then
rename it and add a subcommand form with different operand positions. The
only semantic edits should be in its registry-owned declarations and
implementation. Exercise analysis, a relevant diagnostic, the optimiser,
incremental overlay changes, export/renderer/studio preservation, CLI and
the applicable runtime fallback. Assert that no consumer changes are needed.

Then author a command requiring a genuinely new semantic capability.
Extending the generic interface is legitimate; disguising a new command
ID as a family-neutral operation is not. The design needs an explicit
review rule for that distinction, supported by two unrelated clients of
the operation where practical. Keep a ledger of remaining command-specific
consumer handlers, rather than calling them irreducible by default.

### Adversarial verdict and exit criteria

**Keep the idea; revise the contract before implementing its broad claims.**
Registry-owned specialisation over existing engines is a strong direction.
The branch is not yet complete enough to establish cross-consumer
soundness, and neither a shared lattice nor a central resolver alone
establishes separation of concerns. The replacement proposed in this review
also needs the validation, temporal-state and budget boundaries above.

Before calling the design implementation-ready, require:

1. A resolved policy for which provenance can justify exact facts and
   behaviour-removing conclusions, separate from hook execution permission.
2. One coherent invocation/context identity and documented cross-axis
   resolution, inheritance, conflict and unsupported-capability rules.
3. A validated result/storage/dependency protocol, with explicit restrictions
   on stateful nested evaluation and exceptional completion.
4. A producer/consumer/invalidation contract for every changed fact, including
   diagnostic policy independence, availability and source-edit evidence.
5. Request-wide resource and cancellation policy across all execution routes.
6. The direct, expression, regexp and private-command vertical slices, with
   independent semantic witnesses and negative integration tests.
7. Explicitly deferred capabilities and their conservative fallback, so
   "not supported yet" cannot be mistaken for "proved impossible".

No finite review can prove this architecture complete for arbitrary Tcl
extensions. It can make its supported boundary complete and falsifiable.
That is a stronger and more useful completion criterion than "every pure
command folds" or "every consumer calls the query".

## Restructure the branch around implementable contracts

The main proposal is nearly 2,000 lines, with useful but overlapping surveys,
normative rules, speculative features, and migration plans. The companion
adds another programme spanning compiler admission, package management, and
C hosting. Preserve the research, but make the implementation path easier to
review:

1. **Consumer interface contract:** ownership boundary, invocation context,
   fact/result protocol, proof dependencies, and one complete `expr` example.
2. **Evaluation contract:** direct cores, shared expression engine, optional
   declared VM route, target semantics, state isolation, budgets, and caches.
3. **Value-transfer migration plan:** a versioned source inventory and small
   independently shippable slices, linked to the two contracts.
4. **Runtime and package follow-ons:** retain the companion's identity,
   backing, C extension, and manifest work as separately gated designs.

Keep the inventories in an appendix or generated ledger with the source
revision, scope, exclusions, and reproduction command. Counts such as
"62 sites" are useful observations but are not architectural invariants.
A name-based lint is a warning mechanism; renamed local bindings, helper
tables, or command-specific hook variants can evade it. Contract tests and
ownership review remain necessary. Waivers should state the actual owner
and tracked migration, rather than making all axes permanent exceptions to
a gate named `value-transfers`.

Repair internal contradictions while restructuring:

- "Five kinds" lists six enum variants.
- Phase 1 derives `Cell` for `append`/`lappend` but requires them to remain
  gaps; distinguish descriptor availability from enabled evaluator support.
- The overlay is called a prerequisite of phase 4 but delivered in phase 5.
- The hook-entry counter must stay zero for shipped commands, while Level 2
  makes engine execution their default fallback; define separate counters
  and an explicit baseline workload.
- No-folder pure evaluation has no representation in the proposed resolver.
- `Destroy` is "never authorable" but is derived from an authorable trait;
  the exclusion prevents an emitter verb, not the semantic claim.
- "Every pure command has no gap" is an unsuitable completion criterion.
  Purity does not establish executable backing, support for every target,
  affordable evaluation, or availability of all dependencies.
- Several body-wrapping and branch features contradict the statement that
  every phase is merely a value-axis change. Mark their separate contracts.

## Recommended delivery order

1. **Resolve ownership and evaluator-selection policy.** Write the short
   interface contract and correct the concrete derivation/value errors.
   Existing code supplies adapters initially; any command-specific consumer
   implementation retained temporarily has an explicit migration entry.
2. **A direct vertical slice.** Migrate `string range` and `incr`, preserving
   exact values and target semantics, then add `append`/`lappend`. Exercise
   standalone analysis and the memoised LSP path through the same immutable
   context. Keep stateful calls when propagating their results.
3. **The expression slice.** Registry-owned argument semantics over the shared
   expression engine, with lazy input services and transitive binding facts.
   This validates the interface on a command that cannot be reduced to a
   suffix of literal operands. It should not wait for the long-tail VM work.
4. **A private SpecTcl command through the same interface.** Deliver loader,
   renderer, studio, cache inputs, overlay invalidation, and host isolation
   together. Use one small executable example before migrating hundreds of
   commands. Keep direct builtins on the direct route.
5. **Destructuring and structured bodies.** Add write/preserve outcomes and
   heterogeneous types; handle duplicate targets conservatively. Model dict
   body bindings/reconciliation explicitly. Connect existing diagnostic
   consumers without duplicating their source scans.
6. **Branch integration and optional rewrites.** Exact switch variable
   resolution first; opaque-arm selection, CFG integration, and O131 only
   when their proof and source-edit contracts are implemented.
7. **Broader execution and runtime consumers.** Grow the declared executable
   catalogue using independent oracle evidence, then handle package backing,
   intrinsic guards, WASM adapters, and extensions in their own changes.

Each slice should demonstrate the same behaviour through the relevant
direct, incremental, and command-line entry points. Runtime parity work is
a dependency only where that slice actually executes the affected runtime.

## Validation needed for implementation

Use independent Tcl oracles for semantics and repository contract tests for
ownership, resolution, caching, and consumers. A registry-as-oracle sweep
cannot prove the registry's own evaluator is correct; a direct-core versus
runtime-core comparison cannot detect a shared bug. Shared implementation
tests are still useful for adapter, version, and materialisation differences,
so do not dismiss all cross-checks as tautologies.

| Area | Required fixed-input evidence |
|---|---|
| Ownership | A new spelling and subcommand fit the existing interface without changing consumers; private SpecTcl and native declarations behave alike |
| Values | Whitespace/NUL/backslash preservation, noncanonical integer strings, Unicode target differences, binary round trips, bignum promotion and cancellation |
| Storage | Missing/unknown/existing cells, partial scan, regexp no-match, repeated targets, array/base overlap, traced and escaping variables |
| Expressions | Lazy branches, quoted versus braced arguments, multiple arguments, nested commands, rebound math functions, unknown external inputs, errors |
| Regexp | Our ARE engine through shared plumbing, capture/index semantics, no-match preservation, `-all`/zero-length progress, flags, malformed patterns, resource declines |
| Rewrites | Stateful producer retained, failing dead write retained, implicit return retained, effect ordering and trace behaviour preserved |
| Analysis | Solver join order, loop convergence, finite-set correlations, per-target types and provenance joins, summary recursion |
| Incrementality | Overlay-only edit, body edit, rename in another proc, target change, trace installation, pack reload, shifted source spans, worker migration |
| Execution | Persistent-state attempts, undeclared inputs, absent implementations, budget/cancellation, expensive direct cores, host unavailable or quarantined |
| Branches | Ordered patterns, final default, fall-through bodies, regexp captures/errors, finite subject sets, opaque versus lowered representations |
| Diagnostic separation | Disable a rule or optimiser presentation without changing semantic facts; W100/O111 independence or explicit group policy; no private regexp/scan evaluator in W210 |
| Diagnostic integration | Fast/deep availability, workspace refinement, intentional code overlap, suppression/severity parity, relative-span rebasing and stale-edit rejection |
| Consumer parity and cost | Direct/memoised fact and finding equivalence under the same context; lightweight tokens/symbols do not trigger unnecessary evaluation |
| Partial knowledge and reduction | Per-version constant loss/recovery with reasons; bounded residuals and shape facts; closed-subtree folding; type/target-guarded regrouping; floating-point counterexample in R15 |
| Full fact integration | Existing dynamic return hooks retain intrep/unknown semantics; independently available type/taint/range facts survive exact-evaluation decline; graph changes rebuild SSA rather than mutate it inside hooks |
| EDA iteration | New pack loop spelling, opaque list-looking handle, typed unknown yields, zero/multiple iterations, completion and vendor-world invalidation without consumer name cases |
| BPF semantics and safety | Tcl/BPF division difference, literal loop-bound contract, pointer/map provenance, dominating packet guards, stack/capability checks and source mapping after unrolling |
| Catalogue completeness | Every row in the coverage ledger has an owner and positive/negative/unknown case; independent diagnostic display; XC/BPF/TLS and extensible finding families have explicit gate owners |

Record oracle release and platform per test run. Test the oldest relevant
release for each supported behaviour, not only whatever `tclsh` happens to
be on PATH. Keep seeded/generated exploration in the manual exhaustive
tier; use fixed witnesses in ordinary CI.

Performance acceptance should compare the unchanged tree, direct-core
evaluation, expression evaluation, and declared VM execution on the same
workloads. Measure cold host setup, warm evaluation, cache hits, changed
inputs, solver iterations, cancellation latency, memory, and incremental
LSP latency. The quoted 28 microseconds and 24.5 nanoseconds describe
existing hook workloads; they are not measurements of this proposed
transfer service or arbitrary command implementations.

## Owner decisions

These decisions concern the future implementation. None prevents publishing
this review.

| Decision | Recommendation | Why the owner's call matters |
|---|---|---|
| Final ownership boundary | Requested: specialisation lives in registry-owned code/data; analyser exposes generic operations | Only the allowance and expiry of transitional compiler-owned handlers remain a delivery choice |
| Automatic engine fallback | Require an explicit registry evaluator capability | Purity alone does not supply executable backing, dependencies, or bounded cost |
| Regexp owner | Confirmed: reuse our own `tcl-regex` engine through existing shared command plumbing | Registry specialisations and analyser consumers must share matching semantics |
| Which pack facts may drive proof-bearing optimisation | Confirmed: loaded workspace facts are authoritative without another trust gate, including for pruning and code elimination | Authors own incorrect semantics; provenance tracks dependencies, not an advisory-only or widen-only tier |
| Authoring executable shipped specialisations | Keep Rust shared cores; allow SpecTcl calculations through the same interface where useful | Distinct from whether SpecTcl becomes the source for all shipped specs or emits Rust |
| Scope of the first delivery | Direct values, `expr`, one private pack, and shared analysis context | Avoids making runtime manifests and C hosting prerequisites for analyser extensibility |

The C shim/ABI choice, runtime manifests, and new optimisation-code numbering
can remain open until their own implementation phases. They should not
obscure the decision this branch is actually trying to enable.

## Review evidence and limits

The aggregate changes across all seven branch-only commits and all seven
changed files were inspected.
The branch is documentation-only. Code references in this review were read
from its own worktree, not from another active development branch. The
source baseline is the common ancestor
`16ba98010505f67484a3f1c90e087539bf4919d8`; remote `rust` was 14 commits
ahead of that ancestor at review time. This review evaluates the discussion
as it stood, without rebasing or rewriting its seven existing commits.

Read-only Tcl probes ran under Tcl 9.0.4 and system Tcl 8.6.17. They confirmed
the concrete examples above, including version-dependent `incr` of `010`
(11 versus 9), promotion at the wide-integer boundary, preserved regexp
targets, partial scan writes, duplicate lassign targets, dict body results,
exact whitespace, lazy expression evaluation, and math-function rebinding.
They do not establish complete compatibility across Tcl 8.4 through 9.1,
vendor environments, or our two runtimes.

Two additional fixed regexp probes linked this worktree's freshly compiled
`tcl-regex` library and compared it with those same shells. R13 records the
observed false no-match and approximate capture. This was direct engine
evaluation, not a full VM/WASM differential campaign.

An actual `HookHost`/folder-thunk probe also confirmed that a global counter
survives successive identical invocations of a `const_fold` hook. R7 records
the calling convention and the two observed results.

The adversarial follow-up added fixed Tcl 9.0.4/8.6.17 probes for ordered
stateful expression substitutions, short-circuiting, writes preceding an
error, and correlated loop bindings. A3, A4 and A11 record their results.
The hostile-pack, contradictory-descriptor, cyclic-query and budget-churn
scenarios are design tests to implement, not executed vulnerability claims.

The partial-reduction follow-up directly called this worktree's compiled
expression simplifier with an empty, present type-proof context and compared
the produced reassociation with Tcl 9.0.4 and 8.6.17. R15 records the
observed difference. The Tcl probes also checked closed-subtree reduction,
numeric-coercion preservation, and mixed constant/dynamic string assembly.

The proposed APIs, cache changes, transfers, and new branch facts do not
exist yet, so this review makes no claim that their test matrix has passed.
The coverage ledger was mechanically checked against all central enum
entries and the separately authored numbered XC/BPF/TLS codes, with no
missing, duplicate or extra numbered rows. Chain kinds and open code
families were separately inspected. EDA/BPF conclusions use repository
source and contracts; no proprietary EDA engine or privileged kernel load
was exercised. Authoring sketches are explicitly proposed, not executable
fixtures or loader compatibility claims.
Gate results for publishing this document are recorded with the review
delivery; those checks validate the branch, not the unimplemented design.
