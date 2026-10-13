# Shared name handling

This is the entry point for consumers of Tcl names across the analyser,
compiler, LSP, middleware, runtimes, code generation, tooling and AI surfaces.
Use the [implementer guide](../compiler/name-resolution-implementer-guide.md)
for purpose-specific entry points, the
[consumer matrix](naming-consumer-matrix.md) for actual integrations and
[shared-utility-contracts-rust.md](shared-utility-contracts-rust.md) for ownership.
The [proof index](../analysis/name-resolution-proofs/README.md) owns semantic
questions and their provider-specific evidence. An experiment capture does not
establish an unexecuted implementation result.

Original C and Jim inputs, hosted F5 inputs and explicit Logical reporting have
different owners. Original queries retain the complete image/channel, lexer
configuration, Registry, actual input purpose and positioned binding receipts.
Missing ownership is terminal for definite identity or mutation. Logical
candidate helpers provide bounded assistance; their strings cannot manufacture
native objects, command entries, completed worlds or editable occurrences.

## Representations

A name's spelling, identity and presentation are different values:

| Value | Meaning | Rule |
| --- | --- | --- |
| Written name | Source word or an evaluated Tcl value used for lookup | Parse with the selected dialect and the operation's grammar at ingress. Preserve the source span separately. |
| Namespace identity | A namespace in an interpreter, or a known static path | Retain its identity and lifetime. Its rendered name cannot identify a recreated namespace. |
| Command slot | Namespace identity plus the exact simple name | `CommandSlot` is the runtime representation; `StaticCommandSlot` expresses a statically known slot. A slot's occupant can change. |
| Command identity | A live command object, distinct from its table slot | Use `CommandId` and the runtime's generation/lifetime checks. Imports and traces can retain identity across renames. |
| Variable place/cell | Frame or namespace plus scalar/array identity and links | Resolve under the actual frame context; follow links through the variable owner. |
| Display name | Tcl-facing or user-facing rendered text | Render at output. Never parse it back into identity. |
| Compatibility key | Encoding for an explicitly selected reporting adapter | Keep it separate from Original bytes and allocations; do not expose it as Tcl text or infer a native identity from it. |

Distinct slots can render as the same FQN: namespace segment `a:` with
simple name `p`, and segment `a` with simple name `:p`, both render
`::a:::p`. No string normaliser can recover which slot produced that text.
The same applies to caches, workspace aggregation and editing identities.

`NameBytes`, `ByteNamespacePath` and `ByteCommandSlot` retain exact bytes and
constructed geometry. They do not identify an interpreter allocation. Native
objects additionally retain their issuer, selected purpose, physical storage,
cache and lifetime. Static Original consumers use authentic source allocations
and lookup receipts. Flat strings remain explicit reporting projections; the
existence of a structured type does not establish a consumer's ownership.

## Choose the semantic owner

| Operation | Existing entry point / owner |
| --- | --- |
| Scan a source variable extent | `tcl_core_types::naming::scan_var_name_end_with`, selected by the dialect grammar |
| Parse colon runs and written command names | `tcl_syntax::naming::{qualifier_segments, canonical_written_command, written_command_tail}` |
| UTF-8 compatibility qualification | `naming::qualify`; only with its documented constructed-prefix precondition |
| Namespace-name canonicalisation | `naming::normalise_qualified_name`; unsuitable as a universal command/variable identity normaliser |
| Command lookup base order | `naming::command_lookup_bases` supplies current/path/root coordinates; `NativeCommandSlotProjection::{lookup_bases, lookup_uses_namespace_path}` selects original C extent and path availability. Logical candidate renderers, authored source geometry, native compilation snapshots and live runtime adapters reuse that order; source consumers retain separate path alternatives and stop before later uncertainty when an earlier binding settles lookup. Coordinates grant no live namespace, command binding or closed-world evidence. |
| Variable namespace geometry and query order | `NativeNameProjection::variable_namespace_projection` retains the selected C `VariableRoot` or `NamespaceVariableQuery` components; `NativeVariableNamespaceProjection::namespace_query_bases` admits only the namespace query purpose and selects its C8.4–8.6 global alternate. `variable_namespace_lookup_bases` supplies the same coordinate sequence under an explicit Logical fallback policy. Ordinary scalar/element operation flags, frames, aliases, cell presence and lifetimes remain adapter-owned; Jim query canonicalization is independent. |
| Empty namespace address | `NativeNameProtocol::namespace_address_path` distinguishes `EmptyNamespaceOutsideRoot` from unsupported recipes. Lookup treats that selected invalid address as missing; `report_native_namespace_creation_error` supplies the C creation diagnostic and release-selected error-code update. This supplies no native String allocation or entered namespace. |
| Shared static slot projection | `tcl_runtime_api::command_identity::{StaticCommandSlot, display_namespace, display_command, encode_command_slot}` |
| Original name recipes | `NativeNameProtocol` and the selected operation's checked object/input facade; C, Jim and hosted issuers remain independent |
| Native source text materialisation | `tcl_syntax::backslash::native_arena_text`, over retained `ExecutablePartArena` source/channel/config; runtime admission and operation currency belong to `Interp`, not decoded lexical bytes |
| Original braced value correspondence | `backslash::{native_source_braced_word_map, native_source_braced_word_extent}` and `NativeBracedWordSourceMap` compose the existing continuation scanner with native Document ingress. Exact original/native boundaries round-trip; folded-continuation crossings and partial units decline. Registry word extents reuse this read-only map; it grants no edit or execution authority. |
| Native character units | `tcl_syntax::native_tcl_utf::NativeTclUtf::{decode_unit, decode_c86_ucs4}`; pure selected byte-unit decoding, without object/cache or execution authority |
| C86 split planning and physical results | `tcl_cmd_core::list::plan_c86_split` returns original byte ranges/reuse keys; `tcl_syntax::value::ValueOps::native_split` selects the actual physical adapter |
| Runtime namespace and command lookup | Checked `tcl_runtime_api::Namespaces` interfaces through actual runtime object/table/lifetime adapters; reporting lookup cannot replace original-object lookup |
| Namespace command semantics | `tcl_cmd_core::namespace`: which, origin, import patterns, variable FQNs, qualifiers/tail, parent/children/exists |
| Binding facts and safe specialisation | Compiler `command_binding`, registry `resolved_invocation`, dispatch proofs and site claims |
| Internal single-document target selection | `tcl_lsp_core::definition::{resolve_called_proc, resolve_proc_target_at, resolve_class_target_at}` (`pub(crate)` provider helpers; external crates use public providers/analysis facts) |
| Workspace settlement and import lifecycle | `workspace_index`, server `settle_call_against_workspace`, `namespace_import`, analyser `indirection` |
| Variable source syntax | Dialect-aware helpers in `naming`; array element splitting and word-substitution owners |
| Variable storage context | Compiler `var_resolve`/`var_scoping` and place/alias owners; runtime frame/cell owners |

Runtime lookup currently has separate structural implementations over token
arenas and byte-valued namespace trees. The shared string candidate helper
is not their direct implementation. Preserve storage adapters and identity
when consolidating policy; flattening a tree into display FQNs loses semantics.
Namespace deletion uses the shared `NamespaceDeleteBackend::delete_original`
two-pass operation over original operands. Fresh written lookup and a retained
namespace object's cached live token have distinct semantics during teardown.
The [original command-argument contract](../runtime/original-command-arguments.md#native-c86-source-text-and-split)
describes selected C86 source materialisation and split's storage/lifetime boundary,
including the Runtime adapter and the VM's unavailable-worker refusal.

## Context and observable semantics

Command lookup needs the current namespace, ordered namespace path, selected
release/dialect, live command existence and interpreter domain. Absolute
names search globally. Relative names, including `inner::p`, use
current/path/global candidate order, with command existence deciding the
winner. Native adapters pass retained namespace tokens to the shared base
iterator and resolve the global token only when reached. They keep their own
physical child traversal, liveness and generation checks; equal displayed paths
do not merge tokens. Snapshot lookup validates later namespace rows only after
an earlier binding is absent. There is no implicit ancestor search. Tcl 8.4
excludes namespace-path entries under its original naming protocol.
See [command-resolution.md](command-resolution.md).

Registry descriptor adapters remove one optional root marker through
`naming::unroot_rooted_key` and preserve the remaining exact bytes. Their labels
identify selected metadata; original binding, compiler hooks, generations and
operation currency remain independent prerequisites. Math-function reporting
labels use the shared constructed-key decomposition; the resulting simple name
cannot establish an interpreter allocation. Private implementation-namespace
source diagnostics use shared written component extents and registry package
exclusions, and decline Jim's absent C implementation namespace. The dialect
scanner uses shared written-name geometry only to produce a bounded source hint.

Simulator mock publication uses Registry's `SimulatorMockName`, an authored
ASCII label convention independent of measured F5 lookup. The generator and
pure-function registration share its suffix/category/action projection; Tcl
registration consumes the generated exact-label table. One optional root marker,
first namespace and final action select the authored mock. Nested labels can
share a mock suffix intentionally; original registration labels remain distinct.
Missing, opaque, Unicode, noncanonical separator or empty-component labels have
no simulator recipe. Derived suffixes cannot begin with a colon that would alter
the owned publication namespace; registration presence uses exact command lookup.
The hand-maintained simulation roster remains distinct from environment
availability and provides no appliance binding or frame authority.

SpecTcl native argument-role hook IDs resolve through
`CommandRegistry::native_argument_role_hook`. Exact qualified command descriptors
precede command/member interpretations, even when the selected command lacks
that hook. This descriptor grammar preserves catalogue labels and provides no
written lookup, original invocation or evaluated operand authority.

Native registration metadata caches retain the exact descriptor query label and
selected dialect. A permitted one-marker fallback compares exact descriptor
labels without applying a second rooted lookup; extra root markers are not a
request to canonicalise written Tcl syntax. Selected registration and operand
layout remain independent of this metadata result.

Registry binding rosters preserve exact descriptor bytes when adding an optional
root marker. Namespace normalisation cannot fold descriptor separator runs or
discard an empty final command component. Jim stock registration metadata uses
the same exact descriptor projection against its measured roster; this supplies
no written lookup or live registration authority.

Original procedure-read flow validates the declaration header, complete current
input, original body/formal correspondence and unanimous source alternatives.
Its immutable source graph uses registry roles and successor-layout barriers;
whole read spans must round-trip through the original correspondence map.
The source graph supplies conditional diagnostics. Executable body entry,
physical variable absence, publication and editing retain separate owners.

Conditional member-body catalogue advice retains its actual definition-body
issuer, source/configuration and applicability. Its absolute command-prefix
projection can expose source metadata for an alias without representing an
executed transition or native frame. Core, DB and server consume the same source
channel and retain its explicit obligation.

Variable lookup needs the frame kind, selected release, lookup operation,
namespace, array syntax and alias links. `n::x` is qualified even without a
leading `::`; an unqualified variable in a proc is ordinarily frame-local.
Tcl 8.x namespace-scope fallback differs from Tcl 9.x. Reads, writes,
creation, introspection and declaration operations must not be conflated.
See [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
[compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md).

Renames, imports, aliases, ensembles, traces, unknown handlers and namespace
teardown modify bindings or lookup context. They use the same lookup policy,
with operation-specific identity and lifetime rules. `interp alias` retains
a target name and re-resolves it; an import retains command identity.
See [command-binding-and-aliasing.md](command-binding-and-aliasing.md).

Static consumers publish a resolved identity or a justified abstention with
its context. A display-tail match cannot turn an unresolved call into a
proved reference. Rename and refactoring require source spans and identity
proofs; source colon-run widths cannot be reconstructed from display names.
Per-command roles and semantics come from the registry, including overlays
and stubs, rather than spelling tests in consumers.

## Dynamic code and familiar naming patterns

Tcl naming is executable behaviour. Definitions, command prefixes, aliases,
imports, variable links and object commands can be assembled, changed and
invoked at runtime. Static analysis must support familiar metaprogramming
patterns and demur where their meaning cannot be established. A dynamic source
shape alone is insufficient reason to stop analysis: constant propagation,
finite alternatives, declared effects and frame context may recover its meaning.

Keep the evidence and its permitted use explicit:

| Evidence | Consumer behaviour |
| --- | --- |
| Proven identity in the modelled context | Publish the target/place and its provenance. A rewrite still needs editable spans, completeness and binding/effect proofs. |
| Conditional identity | Retain the assumptions and context that make the result hold. Consumers must verify them or use guarded runtime dispatch; do not silently treat them as unconditional. |
| Plausible candidates from a familiar pattern | Offer ranked completion or an explicit candidate list with supporting evidence. Do not present a candidate as a definite reference or use it to authorise mutation or optimisation. |
| Unknown or ambiguous | Preserve the reason and any bounded candidate set. Demur on definite target selection, unsafe edits and unsupported diagnostic conclusions. |

This is a consumer contract, not a claim that every current result type already
records these distinctions. Extend existing analysis/binding/provenance facts
to carry them; do not introduce a second resolver per provider.

Recognise common idioms through the existing semantic owners:

- Names assembled with constants, `list`, `format`, string concatenation and
  `${ns}::name`; flow-sensitive constant command heads and finite alternatives.
- List-built callbacks, `{*}` command prefixes, `namespace code` and `inscope`,
  with their captured namespace and actual invocation frame.
- `rename`, `interp alias`, export/import/forget and ensemble maps, including
  wrapper commands with declared prefix arguments and binding lifecycle.
- `global`, `variable`, `upvar`, `namespace upvar`, array-element names and
  frame-forwarding wrappers with known argument/effect contracts.
- Package/autoload/source conventions with indexed files, declared overlays
  or stubs and proved load order; repeated procedure factories over finite
  literal inputs and recognisable definition-body grammars.
- TclOO object-handle provenance, `my`/`self`/forwards/mixins, supported framework
  definers, and shared execution scopes such as Tcltest setup/body/cleanup.
- F5 `call` and rule namespaces where rule/profile context is known, keeping
  protocol command prefixes and BIG-IP object paths distinct.

Knowing the bytes of a computed name does not prove which command occupies its
slot when invoked. Knowing a callback prefix does not prove when or in which
frame it executes. Establish these separately. Pattern recognition consumes
registry roles, binding facts, value provenance and shared name/frame policy;
framework knowledge belongs in registry declarations or supported summaries.

Each heuristic needs concrete preconditions, evidence, mutation/invalidation
rules, a bounded evaluation budget and an abstention reason. Unknown writes,
traces, eval/source/uplevel, dynamic factories and external package effects
widen or invalidate affected facts according to their proved effects. Preserve
facts outside that affected scope. A single candidate in an incomplete workspace
does not prove uniqueness; a variable read in an opaque frame does not prove it
is unset. Avoid blanket diagnostic suppression and confident false positives.

Measure useful coverage as well as correctness. Test ordinary developer idioms
that should resolve, near-matching patterns that must not resolve, shadowed
builders, intervening mutation, conditional definitions and cross-file order.
Retain real-oracle programs and outcomes for semantic claims. Review rejected
patterns to improve coverage rather than relaxing the proof required for edits
or optimisations. See [the pattern acceptance matrix](../../../experiments/name-handling/case-catalogue.md#dynamic-pattern-acceptance).
The [wild-pattern catalogue](../../../experiments/name-handling/patterns/README.md)
retains inspected examples, experiment sources and per-oracle evidence. Maintain
it alongside new naming findings; a source example or another provider's result
cannot answer an unexecuted semantic question.

Discover and prioritise patterns using the maintained corpus in
[docs/CORPUS.md](../../CORPUS.md), its authoritative pinned manifest/fetcher,
and the Tcl Wiki. Retain concrete source locations and revisions, check runtime
claims against real oracles, and evaluate coverage and wrong targets on held-out
projects. The [source and investigation protocol](../../../experiments/name-handling/pattern-sources.md)
defines the evidence record. Corpus frequency and Wiki explanations support
pattern discovery; they do not establish an individual call's identity.

## Surface boundaries

LSP providers, server middleware, CLI tools and MCP tools consume common
analysis/lookup facts and adapt their presentation. Native/WASI/browser hosts
and editor clients must preserve those facts and their invalidation context.
AI prompt templates describe the same API contract; generated copies follow
their generator.

Compiler analyses and codegen consume binding and variable-place facts.
Direct/native specialisation needs a binding proof and guards; generic runtime
invocation preserves call-time lookup. WASM procedure definitions transport the
existing evaluated original argv to the shared Runtime procedure owner; the
internal compiled-body key supplies no Tcl name or namespace. See
[original command arguments](../runtime/original-command-arguments.md#compiled-procedure-definitions).
Procedure maps and compiled-entry tables still use reporting FQNs in parts of
the compiler; these are not an injective replacement for structured slots.
Audit all registered backends before claiming that every target delegates
resolution. A restricted DSL such as BPF-Tcl reports unsupported
namespace/procedure features at its boundary.

F5 protocol command prefixes, iRule procedure namespaces and BIG-IP object
paths are distinct concepts. Preserve their registry/context rules. Appliance
`tclsh`, TMM iRules, tmsh CLI scripts and iApps are separate oracle contexts.
See [namespace-model.md](namespace-model.md).

The private `tcl::clock::ParseFormatArgs`, `tcl::clock::getenv` and
`tcl::clock::GetDateFields` workers have selected native C8.6 registry admission,
shipped runtime backing and absent native compiler hooks. The parser and date
worker consume original objects through `tcl-cmd-core::clock`; the environment
getter reads an optional current-host byte capability, preserving raw Unix
keys/values and operational refusal on unsupported adapters.

`GetDateFields` requires the actual native C8.6 protocol and inherited operation
currency. It converts original seconds with the central Wide getter, then
changeover with the Int getter, rejects an actual bignum seconds primary, and
converts the timezone table. It reads the first transition before the binary
search, selects the last transition at or before seconds (row zero before the
first transition), then reads only the selected offset and timezone-name child.
DST and extra cells supply no inputs. Reached List conversions retain original
storage and backing receipts; allocation leases do not make retired or replaced
objects current. Checks around getters and Dict publication preserve the first
typed host refusal across context effects and cleanup.

The result is a genuine fresh Dict with the original fourteen field keys in
worker order, shared CE/BCE literals, fresh native numeric fields and the same
timezone-name child. Each of the three genuine object-command registrations
owns one shared literal-pool reference, released by its once-only delete
callback; an active date-worker call retains the pool across worker deletion.
Returned containers retain their own child references. Empty timezone data
explicitly refuses unavailable native C localtime conversion; the ordinary
clock offset capability supplies no substitute timezone-name or conversion
evidence. Unsafe unmeasured numeric and cell extents also refuse.

`Oldscan`, `ConvertLocalToUTC`, `GetJulianDayFromEraYearMonthDay` and
`GetJulianDayFromEraYearWeekDay`, guest `env` write synchronisation and the full
seven-worker pool remain unsupported. The
[clock evidence index](../../../experiments/name-handling/results/runtime-c86-real-suites/README.md)
separates original C observations, exact Runtime controls, pure calendar checks
and Registry checks; none establishes complete upstream-suite conformance.

## Evidence and enforcement

Shared command, variable and namespace-operation vectors live in
`rust/tcl-syntax/tests/data/`. Their C Tcl runners use `tcl-test-support` and
the release manifest in `tcl-dialect`. Consumer tests must derive expectations
from retained real-oracle observations for the supported release/context.
C Tcl, Jim and F5 can differ; expectations carry that axis explicitly.
Upstream C Tcl `tcltest` cases and Jim's equivalent suite are valid evidence
when executed on the identified real reference interpreter. Retain suite/case
provenance, dependencies, constraints and actual execution results; skipped
cases supply no oracle answer. Harness execution on our runtimes validates
consumers against reference evidence.
The experiment protocol and executable observations are retained under
[experiments/name-handling](../../../experiments/name-handling/README.md).

`cargo xtask owner-resolution` checks owner/API/contract wiring.
`cargo xtask resolution-drift` checks a narrow class of namespace-blind
proc/class scans. Neither gate proves that every consumer has adopted the
owners; representation-aware consumer tests and a complete inventory are
also necessary.
