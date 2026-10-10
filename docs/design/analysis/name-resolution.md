# Name resolution — the model

This is the design reference for how the stack answers
"which *thing* does this name denote?" for the four name kinds — **command**,
**variable**, **class/method**, and **expr function** — across the analyser,
the LSP providers, the bytecode VM, and the WASM runtime.

The selected input determines which naming owner applies. Original C and Jim
inputs retain separate purpose-specific byte recipes; hosted F5 source inputs
retain their actual policy and ContextRegistry. Explicit Logical reporting uses
the representable namespace/catalogue algorithms described below. Reporting
names, whole-file headers and source geometry cannot establish an Original
callable, entered frame, completed world or native capability.

Three documents cover this surface, and they do not overlap:

| Document | Answers |
|---|---|
| [contracts/command-resolution.md](../contracts/command-resolution.md) | *The owners.* C candidate order, Original source/table receipts, independent Jim/hosted inputs, reporting compatibility and bounded conformance gates. |
| [name-resolution-c-conformance.md](name-resolution-c-conformance.md) | *The ground truth.* The C Tcl algorithm and version-specific rules for 8.4–9.1, pinned to source permalinks. |
| **This document** | *The model.* What we build on top of the rule: written-name parsing, the link graph, workspace and library tiers, value provenance, TclOO dispatch, interpreter domains, and every place we deliberately abstain. |

The governing bias throughout: **a confident wrong answer is strictly worse
than no answer.** A missed reference under-delivers; a wrong reference
silently corrupts code on rename. Every mechanism below is built to abstain
when it cannot prove its answer, and each abstention is a recorded fact
consumers read rather than a guess each consumer re-derives.

---

## 1. One resolver, everywhere

Resolution splits into two halves, and both are centralised:

- **Enumeration** — given a definition, find every call site that reaches it.
  Original enumeration compares retained declaration allocations and authentic
  call edges under each current source/URI/configuration/Registry owner.
  Unknown relevant documents or call coverage cannot become an empty inventory.
  Reporting invocation facts and the workspace name oracle supply only the
  independently selected Logical branch.
- **Selection** — given a cursor on a call site, decide which retained
  declaration or implementation it names. Native source consumers first match
  the complete original source image, input channel, full lexer configuration
  and Registry generation. They then consume the command-binding owner's
  original name input, selected lookup scope and declaration allocation.
  `tcl_lsp_core::definition` shares this selection across providers. Logical
  compatibility selection uses the namespace-aware syntax resolver separately.

C Native command identity is a counted byte name plus its typed namespace slot;
reporting names and the public `all_procs` / `all_classes` maps cannot replace
that identity. A bare global call does not select a same-tailed declaration in
an unrelated namespace. A genuine retained namespace path can select it.
Jim retains its selected flat command-name recipe without borrowing C namespace
components. Hosted source coordinates retain their source-only applicability
and cannot manufacture either native recipe.

Readonly declaration grammar is another purpose. Original body layouts can
supply Registry header assistance before operands run, without proving that the
body was entered or the command completed. Hosted F5 metadata retains its
actual `ContextRegistry`, selected vendor context and complete original word
vector. Authored source roles remain separate from C/Jim parameter acceptance,
lookup, successful stores and compiler admission.

Providers — rename, references, call hierarchy, linked editing,
document highlight, go-to-implementation, signature help, inlay hints,
hover, type hierarchy, type definition, workspace symbols, minify, and the
MCP docstring tool — consume the shared owner for their actual purpose. Original
identity uses positioned lookup and allocation receipts. Readonly syntax uses
Compiler `OriginalRegistryWords` under the complete retained ContextRegistry;
selected, conditional, hosted and authored-transition schemas keep separate
obligations. Edit geometry, rewrite equivalence and native admission require
their own receipts. A namespace-blind name scan cannot replace any of them.

Two rules keep it that way:

1. **Adding a resolution behaviour = adding a conformance vector**
   (`tcl-syntax/tests/data/command_resolution_vectors.txt`, executed against
   a real `tclsh`). Drift becomes a test failure in every implementation.
2. `cargo xtask resolution-drift` (in `make xtask-check`) flags any new
   `.name ==` compare in the lexical window of an `all_procs` / `all_classes`
   mention outside `tcl_syntax::naming` and the sanctioned `definition.rs`
   helpers. Reviewed exceptions carry a `// drift-ok: <reason>` comment.
   The gate enforces this shared-owner rule across consumers.

A selected C class command uses the C command lookup order; proving that its
allocation is a class requires the separate original class owner. A reporting
class label or authored declaration form supplies no runtime metaclass role.

---

## 2. Written names: colon runs and addressability

For a selected C name purpose, the shared namespace geometry follows
`TclGetNamespaceForQualName` — invariant from 8.4 to 9.1 for the source forms
below. The original name protocol first selects the reached input extent;
CString and counted purposes cannot borrow each other’s bytes. Jim and hosted
source inputs do not acquire this C geometry from their labels.

- A run of **two or more** colons is one separator, wholly consumed:
  `a:::b` names `a::b`.
- A **lone `:`** is an ordinary name character. `proc :`, `proc a:b`, and
  `rename src :2` all define real commands.
- A name **ending** in a separator run has an **empty simple name** — the
  `{}` command in that namespace. `proc x:: {} {}` defines it; `x::`,
  `x:::`, and `::x::` all call it.
- An **all-colon word** (`::`, `:::`, …) is the global namespace's `{}`
  command.
- `$`-substitution scans a varname as alphanumerics/underscore plus colon
  runs **started by a pair**: `$:` does not substitute, `$a:b` reads `$a`,
  and `$a:::b` reads the variable `a:::b`.

### Addressability, and W314

Because a written run collapses, some legal definitions have **no absolute
spelling**:

| Definition | Reachable | Absolute form |
|---|---|---|
| `proc :` (any all-colon simple name) | bare/relative lookup only | **none** — `:::` is the `{}` proc |
| `namespace eval :` and everything in it | relative only (`namespace inscope : :`) | **none** — `:::inner` is `::inner` |
| `proc {}` / `proc x::` (empty simple name) | yes | `::` / `::x::` |
| `proc a:b` (interior lone colon) | yes | `::a:b` |

C itself trips over this: `namespace which :` renders the unresolvable
`:::`, and a namespace ensemble cannot dispatch an exported `:`. The
analyser flags exactly these shapes as **W314** (at the definition site for
an all-colon simple name, at the `namespace eval` site for an all-colon
segment), without cascading onto addressable definitions nested inside a
flagged namespace. The unaddressable definition is still **reachable by
relative dispatch**, which is precisely what W314's wording says.

### The constructed-key discipline

Written source remains in its original image and channel. A runtime
command-table identity is `tcl_core_types::CommandSlot<S>`: a stable `NsId`
plus a counted simple name. This is necessary because `a:` / `p` and `a` / `:p` are distinct live
slots that both render as `::a:::p`. The native VM and the structural WASM
runtime resolve written names at ingress, retain the structured slot through
rename/import/trace/deletion, and render an FQN only at Tcl-facing boundaries.

The Native source analyser retains `SourceCommandKey::Slot` with the actual
namespace holder and counted name bytes. Logical compatibility keys may use
rooted strings derived from an authoritative `StaticCommandSlot`; VM indexes
that cannot store a slot use private injective storage keys selected by their
`CommandSlot`. These compatibility encodings cannot replace an original Native
name producer. Two rules govern construction and rendering:

1. **Select the actual input and purpose before constructing identity.**
   `NativeNameProtocol` owns lookup, publication, rename source/destination and
   namespace-pattern recipes over original bytes. Those purposes may use
   different extents. Authentic source occurrences retain the original image,
   full grammar and producer; the same display string cannot reconstruct them.
   `canonical_written_command`, `normalise_qualified_name` and
   `qualifier_segments` remain representable C/Logical helpers only where the
   caller already owns their input and selected geometry.
2. **Never use a rendered FQN as table identity.** A display may legitimately
   contain a lone-colon segment (`":::"` is the proc named `:`), so joins
   concatenate with exactly one `::` for presentation. Runtime consumers carry
   `CommandSlot`; static consumers carry `StaticCommandSlot`. Construction-
   inverse helpers such as `naming::key_tail`, `key_holder_and_tail`, and
   `key_segments` apply only where the input is known source spelling or an
   injective compatibility encoding. An `rsplit("::")`, a colon trim, or a
   re-`normalise` of a display can collapse `proc :` into the `{}` name. Display FQNs are output-only and are never parsed back into semantic
   identity.

### The 9.0 `namespace code` intrep round-trip

`namespace code :` evaluated inside `:` generates `::namespace inscope :::
:`. The *text* `:::` resolves identically on 8.6 and 9.0 (both land on the
global namespace, so the dispatched `:` misses), but Tcl 9.0's **generated
word** carries a cached `nsName` intrep — a reference to the namespace
captured at generation time — which `namespace inscope` consumes directly,
reaching the unaddressable namespace by identity rather than by name. Any
string shimmer restores textual semantics. Textual Document lookup follows those textual semantics. An independently
issued original namespace-object/cache receipt has a different purpose and
cannot be inferred from the rendered `:::` word. W314 concerns the textual
name's unaddressability; it does not deny a separately retained object identity
or establish a cache, shimmer or execution permission.

---

## 3. Command names

### 3.1 Resolution tiers

A source query selects its own purpose before consulting provider tiers:

1. **Document** — Native source selection consumes the original call input,
   positioned lookup and retained declaration or implementation allocation.
   Conditional declaration advice supplies only its own readonly grammar.
   Whole-file reporting maps cannot donate a later command, body entry or
   completed world to an earlier call. Logical compatibility uses
   `command_resolution_candidates(ns, path, name)` over its document catalogue.
2. **Workspace** — Original declarations and invocation edges retain each
   owning document’s URI, current complete source/configuration/Registry and
   canonical allocation. A missing loaded-provider or selected-slot receipt
   remains unavailable; the workspace reporting set cannot repair it.
   Explicit Logical compatibility uses `workspace_command_exists` and the first
   recorded representable candidate in `defined_command_names`.
   Server reporting settlement, callable arity and W123 refinement require
   `AnalysisResult::allows_lexical_declaration_advice`, including empty hosted
   inputs whose actual policy/context remains Original. See the
   [workspace diagnostic contract](name-resolution-proofs/original-workspace-diagnostic-refinement.md).
3. **Library / autoload** — `PackageResolver::resolve_auto_command` locates
   the defining file for a `package require`d or autoloaded name (honouring
   `TCL_LIBRARY`, `TCLLIBPATH`, `tclLsp.libraryPaths`, and `.tcl-lsp.ini`,
   mirroring `tclPkgUnknown` / `auto_load`). `ensure_autoload_indexed`
   analyses **only that file** and merges it into the shared
   `WorkspaceIndex`, so definition, references, and rename all answer from
   one index. It is idempotent, a real workspace definition always beats a
   same-named library one, and merged URIs are dropped when the package
   database is rebuilt so a `libraryPaths` change cannot strand stale
   definitions. Indexing provides current source candidates. Package provision,
   loader completion and an actually loaded callable remain independent
   receipts; an indexed filename or matching name does not establish dispatch.

The reporting path tier is **dialect-gated at the recording site**: under a
pre-8.5 dialect `handle_namespace_path_command` records no path entry, so
every consumer skips the tier without threading a dialect through each. The
VM gates at *resolution* time instead (`runtime_version < V8_5`), because its
version knob is mutable mid-life where a document's dialect is not.
Original source lookup instead retains its actual point, policy and namespace
geometry; a missing path cannot use the final reporting path as a substitute.

### 3.2 Source-site namespace propagation

`source` evaluates its file in the caller's current namespace, so a bare
`proc helper` in a file sourced inside `namespace eval ::x` is `::x::helper`.
This is modelled as **seeded re-analysis**, not a string-prefix rewrite:
`Analyser::analyse_with_source_namespace(src, dialect, ns_key)` walks the
whole file inside the source-site namespace — exactly `namespace eval <ns> {
<file> }` — so relative definitions re-home, absolute ones stay put, nested
`namespace eval` composes, and bare call sites gain the seeded tier in their
candidate lists. All the C semantics fall out of the ordinary scope
machinery.

The analyser records the resolution namespace at every `source` site
(`SignatureSource::site_namespace`); the index exposes `source_seed_map`
(per sourced document, the set of namespaces it is sourced under); the
server's `refresh_source_rehoming` reconciles lazily before every
cross-document query, bounded-fixpoint because a seeded parent records
*composed* namespaces for its own nested `source` calls. A non-literal path
is routed through `auto_path_eval::evaluate_auto_path_expr` with the
sourcing file standing in for `[info script]`, so `source [file join [file
dirname [info script]] b.tcl]` re-homes like a literal — and anything the
folder cannot prove abstains rather than guessing.

**One document, several source views.** A document sourced under several
namespaces is one physical syntax with **one source view per seed**
(`namespace eval ::x {source b.tcl}` alongside `namespace eval ::y {source
b.tcl}` creates both `::x::helper` and `::y::helper`). Declaration-side
queries return the **full identity set** (`seed_mapped_symbols`), never an
arbitrary first seed; references union every view's callers; definition
dedupes to the physical site; and rename is an explicit **multi-symbol
edit** where one refusal aborts all of them.
The seed is source advice, not proof that either source call executed. Original
source/file queries retain authentic handler and operand geometry from the
shared source schema; current URI/source/configuration/Registry and actual
loading/namespace receipts remain independent. Logical path-expression folding
is a separately selected reporting branch.

### 3.3 The command link graph

The following flat link graph describes reporting compatibility. Actual import,
alias and rename operations have distinct original token, target-lookup and
publication purposes; readonly source transition advice does not execute them.
For representable reporting names the workspace index lifts each into a flat
`WorkspaceCommandLink` (`linked_qname → target_qname`, plus the defining-side
span). `linked_invocations_of` widens the existence oracle to admit linked
names and chases the winning candidate to its ultimate target;
`resolve_command_target` resolves a cursor on such a call to the command it
really names, so references gather from either side. `follow_links` walks
chains transitively with cycle detection — an alias of an alias of an alias
resolves to the source, and a malformed cycle stops.

The **follow vs rewrite** policy is the load-bearing distinction:

| Surface | Reference? | Rewritten by rename? |
|---|---|---|
| `namespace import` pattern word | yes | **yes** — `link_target_spans` |
| `rename OLD NEW` — the `OLD` word | yes | **yes** (`rename_target_spans`) |
| `interp alias {} a {} TARGET` — `TARGET` | yes | **yes** (an ordinary invocation; the registry marks it a command prefix) |
| `forward m TARGET` — `TARGET` | yes | **yes** (recorded as an invocation during the class-body walk, resolved in the class's namespace) |
| a call reaching the command *through* an import/alias/rename | yes | **no** — the local name is unchanged by the source rename; the VM's token follows it |
| a glob `namespace import ::mod::*` | no link | — (it names no single command) |

### 3.4 Command names held as data

A value becomes an Original reference only through its authentic input,
selected lookup/allocation and source correspondence. Known text alone cannot
supply identity or writable coverage. The following String/provenance mechanisms
describe the reporting model; Original rename plans separately require complete
call coverage, genuine source edit containers and their semantic permission.

**Constant `$cmd` dispatch — flow-sensitive provenance.** A `$cmd` head is
recorded as a pending `ConstDispatchSite` (variable, head span, resolution
namespace, head-expansion flag) and resolved by CFG/SSA analysis against the
compiler's value model — never a lexical last-write-wins constant map, whose
view collapses `if` / loop joins into the lexically last assignment.
`value_provenance::const_contributors` answers, for a variable use at a
program point, the finite set of written constants that can reach it: it
walks the SSA use-version's reaching definitions through φ-joins (`if`,
`switch`, loops, `try`) and pure single-`$var` copy chains, bottoming out at
literal assignments. Settlement then emits, per resolved target:

- one **indirect** invocation at the `$cmd` head — navigation only, never
  rewritten; and
- one **writable literal-anchored** invocation at each contributing
  definition — the edit that keeps the dispatch alive, so renaming `target`
  rewrites `set cmd target` and the renamed output still executes.

A branch join keeps **every** may-target: `set cmd foo; if {…} {set cmd bar};
$cmd` references both, each with its own writable literal, and renaming one
rewrites only its own contributors. `Statement::AssignConst` carries
`value_span: Option<Span>` — `None` means the constant has *no exact source
representation* (folded or desugared), and every provenance consumer must
refuse to write through it. An expanded `{*}$cmd` head narrows the writable
span to the value's first list element. Unprovable shapes — computed values,
proc parameters, `upvar`/trace-reachable writes, opaque `catch` bodies —
abstain, and a contributor with no writable span sets
`SignatureCommandInvocation::rename_safe = false`, which makes every rename
provider refuse the **whole symbol** rather than emit an edit set that leaves
the dispatch running the old name.

**Dispatch tables.** `harvest_table_command_value_spans` recovers
`(table, value, value-span)` triples from constructors whose literal text is
recoverable — `set arr(k) v`, `array set arr {k v …}`, `dict set d k v`. A
value becomes a command reference only when (a) its table is **consumed** by
a `$table(...)` or `[dict get $table …]` dispatch site — so an unconsumed
config array gains no phantom references — and (b) it resolves to a known
user command in the constructor site's namespace. These anchor at the
literal, so **rename rewrites the table entry alongside the proc**. Values
reachable only through folding (`string map`, computed keys) have no span
and abstain.

**Typed reference roles.** `ArgRole::CommandName` marks a whole word that is
a bare command name held as data — introspected, not invoked, so it carries
no arity. It is applied to `info args` / `info body` / `info default PROC`,
`namespace origin NAME`, and the traced command of `trace add`/`remove
command`/`execution` (leaving the trailing callback its separate
`CommandPrefix`). `ArgRole::CommandNameProbe` is the same reference with a
different *existence* policy: `namespace which -command NAME` and an exact,
pattern-free `info commands NAME` navigate and rename like any reference,
while the recorded `existence_probe` fact keeps the site out of the W123
unresolved-command pass — so a legitimate `[namespace which -command foo] eq
""` check is never flagged. Reference identity and existence assertion are
orthogonal by construction. A glob pattern names no single command and
abstains. `tailcall` (arg 0) and `coroutine` (arg 1) are declared
`command_prefixes`, and `namespace ensemble create`'s `-map` targets and
`-subcommands` names are recorded as references resolved in the ensemble's
namespace, with dynamic elements skipped.

### 3.5 Nested definitions and body namespaces

`Analyser::command_resolution_namespace`, built on the shared
`advance_command_resolution_namespace` per-scope-kind rule, is the *single*
answer to "which namespace is current here?" for every analyser site, so a
call site cannot pick a different one. A definition made inside
`proc ::x::mk {…}` homes to `::x`, not `::`, where it would collide with a
real global under the same `all_procs` key. The rule
covers `proc`, `oo::class create`, `oo::define`, `namespace ensemble
create|configure`, snit `type`/`widget`, itcl `class`, `namespace
import`/`export`, package import aliases, alias resolution, registry-definer
qualification, and deferred method bodies.

Two deliberate exceptions:

- **`apply`'s namespace element is never relative.** `doc/apply.n` and
  `TclNRApplyObjCmd` both `::`-prefix the word before lookup, so
  `handle_apply_command` qualifies it against the global namespace
  unconditionally (tclsh 9.0.4-verified from three different calling
  contexts).
- **TclOO reporting bodies use a global approximation**
  (`Scope::oo_global_resolution`) when the actual object namespace is
  unavailable. Original body selection retains that unknown frame and namespace;
  the reporting `::` does not establish either one. In this compatibility model
  a `namespace import` written inside a method is attributed to `::`, and
  `namespace inscope NS SCRIPT` is **not** one of these cases — it shares the
  `namespace eval` hook, so its body walks in `NS`. `namespace code SCRIPT`
  carries an `ArgRole::Body` and is analysed in the *current* namespace,
  which is where its captured script runs when the callback fires.

---

## 4. Variable names

Original variable consumers select their own input, receiver form, actual frame,
namespace/cell generation and read/write purpose. `var_resolve` and retained SSA
views keep those receipts separate from declaration advice and display maps.
The link-name, scope and candidate machinery below describes reporting
compatibility where an actual storage or activation receipt is not supplied;
it cannot manufacture a cell, read, successful write or entered alias.

The C mechanism is `VAR_LINK`: `upvar`, `global`, `variable`, and `namespace
upvar` each install a `Var` whose `value.linkPtr` points at the target, and
every lookup transparently follows the chain. **The alias and the target are
one storage cell** — one variable with two names. Identical 8.4 → 9.1; only
`namespace upvar` (8.5, TIP 250) is newer than the rest.

### 4.1 The analyser's link model

`VarDef::link_target` holds the qualified cell name, mirroring `VAR_LINK`.
It is populated by `apply_state_transitions`, from the
`VariableCellAliasTransition` each invocation states — `global` (`::v`),
`variable` (`<current-ns>::v`), `namespace upvar` (`<ns>::otherVar`), and a
pack command's alias facts alike — each keeping the **full qualified
path**, so a relative `variable child::v` targets `<ns>::child::v` and
`namespace upvar ::a b::c local` targets `::a::b::c`.
`definition::linked_var_reference_spans` walks the scope tree and unions the
uses of every `VarDef` sharing one cell, wired into references, rename, and
document highlight, and surviving the incremental graft. Two same-named
`variable count` declarations in unrelated namespaces stay distinct.

Three contexts abstain rather than link:

- **`upvar ?level? otherVar local`** defines the local alias with **no**
  `link_target`: the caller frame is statically unknown, so it never
  mis-links.
- **Non-`#0` `uplevel N {…}`** opens an isolated `Uplevel` child scope tagged
  with the level word. `uplevel_hides_scope` resolves `#0` outward to the
  global frame, but a non-`#0` level resolves **only within the body**,
  abstaining on both the enclosing proc and the global. Completion abstains
  the same way — the enclosing proc's locals are genuinely not in scope
  inside `uplevel 1`, so offering them would suggest out-of-scope variables.
- **Dynamic declaration names.** `variable $dyn` records nothing; every
  declaration handler is gated on `naming::is_dynamic_word`, the same `$`/`[`
  test `rename` and `proc` use.

`dict with` binds keys only for a literal dict word; `dict update` binds its
alias variables with no key link. Both are honest misses, not wrong edits.

### 4.2 TclOO instance variables

At actual method entry, independently selected class/object variable links can
address the same cell across method frames. The reporting OO model separately
seeds declared variable advice in method bodies. `collect_var_decl_spans`
(`analyser/oo.rs`) retains each written `variable` declaration span through
`walk_method_body` and `DeferredBody`; an unavailable declaration uses a
zero-width advisory span. Neither the method body range nor an advisory seed
is an editable variable declaration. Original variable rename requires its
own current receiver, source spelling and reference-coverage receipts.

### 4.3 The 8.x namespace-scope global fallback

For the represented unqualified variable lookup at namespace scope,
8.4/8.5/8.6 can fall back to the global cell when the namespace variable is
undefined; 9.0/9.1 omit that fallback and raise "no such variable".
Original receiver extent, namespace allocation, alias and observer purposes
remain independently selected; this comparison does not describe every
cross-version naming behavior. One registry knob is the single source of truth:
`DialectProfile::namespace_var_global_fallback` derives the behaviour from the
dialect's *runtime base version*, so `f5-irules` follows its embedded 8.4 and
EDA shells follow their embedded cores; an unknown base takes the stricter
9.0 reading. No dialect strings appear anywhere else in the stack.

Three layers honour it:

- **VM** — a `RuntimeVersion` knob (default `V9_0`, inherited by
  `fork_child`, exposed as `tclvm --tcl-version`) gates variable-place resolution for
  reads, writes, `unset`, `incr`, and `info exists`.
- **Runtime** — `Namespaces.ns_var_global_fallback` gates `ns_scope_fallback`
  in `classify`. A declared-but-unset `variable` installs a **self-link
  marker** — C's undefined-`Var` stand-in — which blocks the fallback exactly
  as 8.6 does.
- **Analyser** — `record_var_read` attaches a bare namespace-frame read to
  the *global* cell under 8.x only, never to an intermediate namespace;
  `lookup_var_in_scope_chain` applies C's two-table rule at namespace frames
  (own table, then global iff 8.x); completion only forces `$::g`
  qualification at namespace scope under 9.0+ semantics. Proc, method, and
  uplevel contexts keep the documented lenient outward walk — a navigation
  choice that predates versioning and is unchanged.

Pinned by vectors run through the VM at both `V8_6` and `V9_0` *and*
executed under real `tclsh8.6` / `tclsh9.0`, so the table cannot drift.

### 4.4 Four data models, one rule set — deliberately not unified

Variable resolution has four implementations, and that is the decision, not a
defect:

| Layer | Shape | Consumer |
|---|---|---|
| Analyser `VarDef` | span/position-oriented | LSP navigation: declare/use sites, alias unification |
| Compiler place layer (`var_resolve.rs`) | `Place`/overlap-oriented | static-analysis soundness, over-approximating dynamic names for SSA/SCCP |
| Bytecode VM | flat per-frame value map with `VAR_LINK` | execution |
| WASM runtime | arena of cells, `NsId` tree walk | execution |

Unifying the *structs* would couple navigation to execution storage for no
benefit. What they correctly share is the alias **semantics**: `naming.rs`
candidate resolution, `var_refs.rs` scanning, and the `VAR_LINK` rule. "One
alias model" means one shared rule set expressed once and viewed four ways —
not one shared type.

---

## 5. Classes and methods

Original class and method selection uses canonical declaration and receiver
owners, including `receiver_identity`, `method_symbol` and the independent
source member roster. Complete source/configuration/Registry and exact
class/object allocation, side, method entry and selector purpose remain separate
premises. Known original inputs do not enter the String hierarchy’s tail
fallback. The hierarchy, type-flow and factory catalogues below describe
reporting/source assistance; they supply neither runtime object identity nor a
Native constructor, member dispatch or complete rename permission. Authored
member bodies retain their genuine whole worker words and selected source
dialect without issuing body activation.

### 5.1 Class-name resolution is the one-hop rule

C resolves a bare `superclass` / `mixin` name relative to the `oo::define`
**call-site** namespace, in two scopes only — current, then global, plus
`namespace path` from 8.5 — via `GetClassInOuterContext`. There is no
ancestor walk. A bare `superclass Base` inside `::a::b::Sub` where `Base`
exists only at `::a::Base` genuinely errors at class-definition time in real
Tcl, so the analyser must not link it.

`class_hierarchy::resolve_class_name` uses
`naming::bareword_resolution_candidates` and abstains on that shape, keeping
a sound-by-abstention unique-tail fallback for the cross-file `namespace
import` idiom. `resolve_written_class_name` (exact → canonical global
spelling → colon-run rule → unique tail, abstaining on ambiguity) is the
shared call-site resolver that W308's `canonicalise_class_name`, the
semantic-tokens definer head, and `resolve_user_class` all delegate to.

### 5.2 MRO is a graph walk, not a lattice

TclOO's linearisation is **DFS with late placement**, not C3: a two-pass walk
(`BUILDING_MIXINS`, then non-mixin) where a re-encountered method is *copied
down* to the latest position, so methods come as late in the chain as
possible. It resolves diamonds deterministically where C3 would raise.
`tcl_syntax::mro::tcloo_linearise` is the canonical implementation, asserted
against real tclsh `info class call`. `class_hierarchy::build_class_hierarchy`
consumes the `ClassDef` index — `superclasses` and `mixins` come straight off
`ClassDef`, populated uniformly for TclOO **and** snit by the definer
registry — and precomputes `mro_map`, `method_providers`, and the subclass
closures. Because it reads only `ClassDef` and the definer family, it
generalises to snit and itcl with no command-name matching. `next` /
`nextto` walk the receiver's MRO past the current provider; `nextto C m`
restarts at `C`.

### 5.3 Dispatch: visibility is registry data

The registry owns the visibility semantics.
`DefinitionBodyGrammar::member_default_exported` is C's `PUBLIC_PATTERN`
rule — exported iff the first character is an ASCII lowercase letter — and
the analyser applies it at member (re)definition, layering explicit `export`
/ `unexport` with last-writer-wins (a re-`method` resets to the default,
tclsh-pinned). The workspace index carries a typed method table
(`WorkspaceMethod`: name, receiver kind, effective export state, `private`
flag) plus each record's explicit export/unexport deltas.

`WorkspaceIndex::method_dispatch_chain` computes the C-faithful chain for a
receiver class: the canonical linearisation filtered by `MethodAccess`.
`External` (`$obj m`) sees exported implementations only; `Internal` (`my m`,
declaration-side cursors) reaches unexported ones, and `private` definitions
only in the receiver's own class. **Go-to-definition returns the chain
head** — the implementation the call actually enters — never the override
family, and an externally-uncallable method resolves to nothing, mirroring
C's `unknown method`. The in-document provider applies the same rule through
the analyser's `mro_map`.

Rename and references deliberately keep the **ancestry-closed override
family** policy: a polymorphic name is renamed across the family. `next` /
`nextto` sites are references but are never rewritten (they carry no name),
so they fold into the read-only method paths and never into the set that
drives rename.

The four dispatch spellings differ in *where the class comes from* and *what
they can reach*, and both are recorded facts rather than re-derived guesses
(`DispatchReceiver`, `CmdCommandSite`):

| written | class evidence | reach |
|---|---|---|
| `$obj method` | SSA type lattice / constructor harvest | object command — exported only |
| `[Dog new] method` | the substitution's return type | object command — exported only |
| `objcmd method` (from `CLASS create objcmd`) | `instance_classes`, gated on `created_instance_commands` | object command — exported only |
| `my method` | the **enclosing** class body containing the offset | self-dispatch — exported *and* unexported |

`[self] method` and `[self object] method` name the same receiver `my` does
but resolve as the *object command* row, because the substitution yields the
object's own command, which filters exports — tclsh 8.6.16 and 9.0.4 agree
(`my varname v` works where `[self] varname v` is `unknown method`). The
self-dispatch keyword is never spelled in the walker:
`CommandRegistry::method_dispatch_keyword` answering
`MethodDispatchKind::SelfDispatch` identifies it, so a dialect that gains or
loses one propagates through the registry. `next`/`nextto` (`NextChain`) and
`self` (`Introspection`) are deliberately *not* dispatch sites — neither
names a method in any of its words.

### 5.4 Per-object methods

`oo::objdefine` shares the `oo::define` member grammar, so its body and
inline forms parse with the same helpers into a *throwaway* `ClassDef` —
deliberately **not** registered in `all_classes` (a per-object extension is
not a class and must never leak into class listings, hover, rename, or
completion), homed under a private synthetic `@objdefine@…` name so the
duplicate detector never confuses a per-object `greet` with the class's own.
Each method body is walked into the scope tree, so in-body diagnostics and
resolution light up exactly as inside `oo::define`, and the declarations land
in `AnalysisResult.object_methods`, which go-to-definition consults **ahead**
of the class — matching TclOO's per-object layering.

Records key by the receiver's **binding identity** — each carries its
`oo::objdefine` site offset, and lookup matches call sites whose receiver
resolves to the same variable binding (the innermost proc or method body
declaring the name) — so two unrelated locals both named `o` in different
procs never collide. Several binding-compatible candidates (an ambiguous
reassignment) abstain to the class chain. The per-item incremental graft
merges and rebases the records so the server path agrees.

### 5.5 Cross-file `oo::define` and class references

A cross-file `oo::define ::C` records a second `::C` index entry with empty
superclasses. `WorkspaceIndex::resolved_parents_of` unions superclasses and
mixins across **every** indexed definition of a class, and both method-family
closures route through it, so no indexing order lets the stub hide the real
hierarchy edge. `ClassDef::via_define` marks an
`oo::define` on a locally-uncreated class as an extension stub, and
cross-document go-to-definition prefers the `oo::class create` site, falling
back to all sites only when a class is defined solely by `oo::define`.

Class references from `superclass`, `mixin` and itcl `inherit` use
`record_member_command_references` and Registry member descriptors:
`MemberSpec::all_args_ref == MemberRefKind::Class` and
`ArgRole::CommandName` positions also retain forward target references.
These reporting references carry no fixed call arity and do not invoke a
superclass, mixin or forward target. Original source consumers independently
retain the selected member/body context, authentic operand geometry and current
class identity; reporting names cannot replace those owners. References and
code-lens counts consume the common reference inventory, while rename additionally
requires its complete original selection, visibility and edit coverage.
`references::class_references` consumes these references for Find All References
and rename, while the canonical declarations retain their original source owners.

### 5.6 Object→class binding is a lattice — and the ⊤ taxonomy

"Which class provides this method?" is a graph walk (§5.2). "What class does
`$o` hold *here*?" is a flow-sensitive dataflow fact that merges at joins and
must be able to say *I don't know* — a lattice. Dispatch is the product of
the two: for each candidate class in the lattice value, index the MRO table;
when the value is ⊤, abstain.

```text
           ⊤   (abstain — tagged with WHY)
          /|\
    Set{A,B,…}      ← control-flow JOIN of concrete bindings
        |
    Concrete(A)     ← set o [A new]
        |
        ⊥   (never seen to hold an object)
```

`join` is the least upper bound: `⊥` is identity, two distinct concretes
widen to a `Set` (**not** ⊤ — a finite class set is still resolvable, and
every member is checked against the MRO table), and ⊤ absorbs. The primary
object-typing signal is read out of the **existing SSA type lattice**
(`FunctionUnit::types`, which already infers `TclType::Object { class_name }`
per SSA version) — there is no second dataflow engine. See
[tcloo-object-typing.md](tcloo-object-typing.md) for that model.

Every abstention is tagged, so a consumer reports *why* rather than guessing:

| reason | trigger |
|---|---|
| `dynamic-assign` | an object binding and a non-object literal reach the same var |
| `factory-return` | bound from a `[proc …]` whose object class is not modelled |
| `runtime-oo-define` | class mutated by a runtime `oo::define` after binding |
| `introspection` | bound from `info object class` / `oo::copy` / `[$cls new]` / `self` |
| `per-object-mixin` | receiver touched by `oo::objdefine` |
| `forward` | bound from a `forward` or alias not modelled |
| `cross-file-miss` | the class *name* is known but has no `ClassDef` in the index |
| `unknown` | bare parameter / global / `upvar` — no local evidence at all |

A bare class name resolves through Tcl's own candidate order and is **never**
matched to a same-tailed class in an unrelated namespace, so cross-file
resolution cannot manufacture a confident false resolution from a namespace
collision.

`ClassValue`, `TopReason` and `analyse_dispatch` in `class_lattice.rs` expose
standalone advisory lattice and dispatch queries. Shipping object diagnostics
consume the SSA type/provenance and class hierarchy owners. A class label or
finite label set supplies no Native receiver allocation, current method table or
entered frame. Source receiver assistance independently requires the genuine
positioned `source_class_instance_words_at` carrier and canonical original class
metadata, including its conditional applicability obligations. Actual dispatch
and diagnostics that require current presence or absence retain their own
original allocation, lookup and completeness premises. Missing ownership remains
unavailable even when a reporting class name matches. The fixed
[receiver corpus measurements](../../../experiments/mro_eval/RESULTS.md)
retain their observed source population and supply no per-site Native identity.

### 5.7 Abstentions on the class *definition* side

Independent of the receiver question: *what does this class contain, and is
it even a class?* Both answers are recorded on the `ClassDef` so every
consumer abstains from one fact.

| flag / state | trigger | consumer effect |
|---|---|---|
| `ClassDef::inheritance_unknown` | manufactured by a user metaclass whose `create` override could not be read, so the spliced superclass list is unknown | W308 abstains: an inherited method is not a missing one |
| `ClassDef::member_set_incomplete` | the body installs members the walk cannot read | W308 abstains: the member tables are a lower bound |
| *no `ClassDef` at all* | `X create Name … Body` where `X` cannot be **proved** a metaclass — a dynamic head, or a name no factory index carries | nothing recorded, nothing diagnosed |

**Reflective member installation.** ticklecharts' `chart3D` builds its whole
public surface by `foreach method {…} { method $method {*}[…] }`. Tcl expands
both shapes at definition time, so the members are entirely real; what the
analyser cannot do is *name* them. Two registry-driven signals set
`member_set_incomplete` (`Analyser::member_declaration_is_opaque`): a
**member** word whose declaration arrives through a `{*}` expansion that
would not splice statically, or with a computed word in a declaring role
(`Name` / `ParamList` / `Body`); and a **non-member** word that either has no
registry spec or declares an `ArgRole::Body` — a script that can install
members out of sight. Neither signal names a command or keyword. What is
readable stays recorded; the flag only says "there may be more", which is
exactly the premise W308 needs. Readonly member declarations can retain authentic conditional installer schema
and whole worker words, including declarations nested in source loops. Such
source cards retain their unknown applicability and never fabricate an installed
`MethodDef`, formal topology, runtime roster or successful declaration from a
literal loop list.

**Cross-file class factories.** A user metaclass is resolved wherever it can
be **proved** to be one. When a class is recorded, the walk asks whether it
is itself a factory (its superclass chain reaches a registry
`IS_OO_METACLASS` command with a `TclOo` grammar) and stores a
`ClassDef::factory`: per manufacturer subcommand, which creation argument is
the new class's name, which is its body, and the prologue it splices — the
last as a *template* (`FactoryWord::{Literal, CallerSplice}`), because
`{*}$superclasses` is only resolvable per call. The template is call-site
independent by construction, which is what lets one derivation serve every
call site. A workspace **class factory index** (`ClassFactoryIndex`) is
published by the salsa graph and consulted when the local class table misses,
riding the same rails `external_call_sites` already uses; an empty index is
stored as `None` so a workspace with no metaclass never moves the input off
its default.

`DefinitionBodyGrammar::manufacturers` and command-level
`manufacturer_methods` carry each method's name, visibility, instance-name
position, definition-body position, and constructor-payload start; consumers
query those descriptors and never match `new`, `create`, or
`createWithNamespace` by name. The distinction is real: C Tcl hides
`createWithNamespace` on ordinary class-command dispatch and hides `new` on
`oo::class` itself, so `exported_manufacturer_method` is the single
external-callability query used by lowering, signature scanning, binding,
type inference, and semantic tokens. An authored class-shaped source line cannot publish a class allocation merely
because its words resemble a constructor. Independent readonly source roles
can describe that syntax with applicability obligations; manufacturer admission,
actual allocation and constructor completion remain separate.

The publish is a **fixpoint**, not a single pass: `item_tree` reads the
published factory index and the index is built out of `item_tree`, so one
publish is exactly one link of the metaclass chain deep.
`sync_workspace_class_factories` iterates — recompute, compare-then-set,
repeat — stopping when a round moves nothing. It is bounded, not merely
expected to converge: a round adds an entry only when some file *proved* it,
so growth is monotone over a set bounded by the project's literal creation
calls; a cyclic declaration proves neither link and settles empty on the
first round; `CLASS_FACTORY_SYNC_ROUNDS` caps the loop and publishes what has
been proved, which is still sound if incomplete; a round cancelled by a
concurrent edit is retried rather than mistaken for the fixpoint; and a
workspace with no metaclass settles in one round that writes nothing. Two
supporting behaviours keep the extra rounds off the critical path: a
`SourceFile` created for an arriving document is **seeded** with the oracle
the rest of the project already carries, and each round's invalidated peers
are rescheduled as that round lands. The oracle travels with deferred
proc/method bodies (`ItemBodyKey::body_env`), so a `Meta create …` inside a
proc body is classified identically by the whole-file and per-item
strategies.

What still abstains, and must: a **dynamic head** (`$meta create …`); a name
the index does not carry; a name that is only a **tail** match (the lookup
walks Tcl's own candidate order and takes an exact hit only, so a global
`Megawidget create …` never reaches `::tk::Megawidget`, and a locally-written
class of the same qualified name shadows the index as the interpreter would);
and an **unreadable prologue**, which yields `inheritance_unknown` cross-file
exactly as same-file.

---

## 6. Interpreter domains

Original interpreter paths retain genuine list elements, the selected child-key
purpose and independently current interpreter allocation/generation. Runtime
child commands, deletion/recreation and evaluation ownership remain separate.
Synthetic `@interp@` source domains below are reporting-analysis isolation;
their strings cannot supply an actual interpreter token, runtime command table
or frame. Original source body advice preserves an unknown entry rather than
turning domain geometry into successful `interp eval`.

The analyser maintains an interpreter-domain map driven entirely by registry
hooks (`InterpCreate` / `InterpDelete` / `InterpHide` / `InterpExpose` /
`InterpEval`) — no command names in the walker.

- **Identity.** A literal `interp` path (a Tcl list, relative to the current
  interpreter) keys the domain. Paths named inside a child's eval body
  qualify against the enclosing path: `interp create t` inside `interp eval s
  {…}` names `s t`. Evaluation bodies home under the synthetic namespace
  `@interp@<path>` — unrepresentable in Tcl, so a real parent namespace of
  the same name can never collide — and repeated evals into the same live
  interpreter accumulate, as in C.
- **Temporal identity.** `interp delete` bumps the path's **epoch**; a
  re-created interpreter homes under `@interp@<path>#<epoch>` and never
  merges with its predecessor's definitions.
- **Existence.** An `interp eval` into a literal path never created in the
  file draws **W140** (abstaining when any interp operation used a dynamic
  path).
- **Source visibility.** `SourceInterpreterVisibilitySnapshot` retains the
  original complete child creation/evaluation source, C naming policy, full
  editing input, scoped visible slots and distinct hidden-token allocations.
  `Traits::SAFE_INTERP_HIDDEN` selects the provided Registry source initializer.
  Original structured define/move/delete/alias and hide/expose operations update
  this conditional source map. A literal single colon remains name data; scoped
  command keys are projected by the selected C purpose, never by stripping all
  leading colons. Unsupported known lookup/table transitions withdraw advice.
  The source-only C root geometry requires the original complete `interp eval`
  body owner. An unknown nested child cannot borrow its parent's snapshot.
- **Conditional visibility diagnostics.** W129 carries
  `ConditionalInterpreterVisibilitySubject`, including original name/whole
  invocation, scoped slot, full source/configuration/context and explicit
  applicability obligations. `Diagnostic::conditional_interpreter_visibility`
  and the shared `DiagnosticSubjectData` transport expose this data without
  reparsing messages. W129 does not select an actual hidden allocation or
  exclude invocation effects: possible source/package/definition edges remain.
  Direct and nested calls, original list quoting and deferred-body snapshots
  share the same issuer. A reporting ensemble map or handle name alone cannot
  issue an original mapped-target/body receipt. Jim does not acquire the C child
  interpreter surface through this source model.
- **Cross-domain aliases.** `interp alias PATH name TPATH target` records the
  alias under the *source* domain (`::@interp@<path>::name`) targeting the
  *target* domain's command, so child-side calls resolve through the ordinary
  link machinery while definitions stay separated.
- **Multi-word scripts.** `interp eval p w1 w2 …` concatenates at run time and
  commands can span word boundaries, so the words are **consumed without
  walking** — sound isolation. Walking them in the parent scope would merge
  child definitions into the parent namespace. W312 separately flags the
  injection-prone shape.

Command-table mutations inside a child retain separate source and execution
purposes. Source definitions, moves, deletions and aliases use original scoped
operands and source order. Renaming a child command does not mutate the parent's
file-wide reporting maps. Hidden allocations remain independent when a new
visible command occupies the same slot; exposing an allocation under a different
name preserves that distinction. Unknown entry, operand materialization,
observer interference and unmodelled mutation remain explicit source
assumptions, never a completed-world or selected-callable result. Deferred-body
cache keys retain the complete source receipt and immutable editing generation;
a projection that loses that owner uses the whole-file analysis entry.

The implementation assertions and exact CLI observations have separate scope:
[original source visibility advice](name-resolution-proofs/interpreter-original-source-visibility-advice.md)
and [C child visibility source controls](name-resolution-proofs/interpreter-safe-visibility-source-controls.md).

Frame identity inside alias callbacks and cross-interpreter runtime re-entry
are execution concerns, not resolution ones; they live with the VM.

---

## 7. Expr functions

`expr {f(x)}` compiles to the **relative** command `tcl::mathfunc::f`,
re-resolved against the current namespace at run time, so a namespace-local
`proc ::ns::tcl::mathfunc::f` shadows the global one and any TIP 232
`proc ::tcl::mathfunc::f` dispatches exactly like a builtin. Tcl 8.4 is a
different mechanism entirely — a fixed C function table with no command name,
no namespace, and no user override.

Native readonly function selection retains the checked expression AST,
original function extent, full source configuration and independently selected
function dispatch protocol. C 8.4 and Jim fixed-table functions supply selected
Registry metadata; C 8.5+ command-table functions retain their actual original
lookup and declaration reference. The source owner is
`OriginalMathFunctionOccurrence`, queried through
`AnalysisResult::original_math_functions_in_source`; an expression function is
never manufactured as an ordinary command-head word. Missing ownership
withdraws Native navigation before any reporting-map fallback.

Logical compatibility records `ExprNode::function_calls` as mathematical
invocations. Its flags and candidate rules remain separate from the Native
function producer:

- **The mathfunc shape is a recorded flag, not a string sniff.**
  `SignatureCommandInvocation::is_mathfunc_call` is set once, at record time,
  by the single caller that needs it. A mathfunc invocation's resolved name
  always carries the fixed `tcl::mathfunc` dispatch segment regardless of the
  caller's namespace, so the generic settlement pass's one-hop
  `{ns}::{tail}` suffix strip would recover `::tcl::mathfunc` as a bogus
  calling namespace and — whenever *any* unrelated command in the file shared
  the bare tail (`proc sin`, `proc max`) — silently mis-resolve `sin(...)` to
  it. Settlement branches on the flag and uses
  `command_resolution_candidates(&ns, path, "tcl::mathfunc::f")` — the
  **`namespace path`-aware** builder, matching the VM, which routes mathfunc
  lookups through the same resolver as everything else.
- **Dialect gating.** `tcl_syntax::expr::mathfunc::added_in` is the single
  source of truth for which names are expr functions and the release each
  first appeared in (8.4 fixed table; 8.5 TIP 232 `min`/`max`/`isqrt`; 9.0
  TIP 521 `is*`; 9.1 TIP 745 C99). The const-folder declines to fold a
  function newer than the dialect's expr-grammar base version, and the
  analyser emits W002 on the same token — both reading one shared
  `math_func_ceiling_for_dialect`. W123 consults the same table, and
  additionally requires `mathfunc_command_wrappers_available_in_dialect`
  (8.5+) for an *ordinary* call that merely settles to a
  `::tcl::mathfunc::…` shape, since the command wrappers that make such a
  bareword valid at all postdate the expr functions themselves.

---

## 8. Deliberate abstentions

These are the answers we refuse to give, and why refusing is correct.

| Surface | Behaviour |
|---|---|
| **Dynamic command head** (`$cmd` with unprovable provenance, `{*}$cmd`, computed) | Actual target identity remains unavailable. Authentic source vectors and conditional schemas may remain readonly advice with explicit unknowns; reporting constant provenance is covered by §3.4. |
| **`upvar` to a non-`#0` frame** | The target frame is statically unknown. The local alias is defined with no link; the body scope is isolated. |
| **Non-literal `namespace path`** | Original lookup retains an unknown path obligation. The reporting approximation uses an empty candidate-path list, which cannot donate an Original lookup. |
| **`forward` target at call time** | The target is re-resolved per call against the object's namespace, so only the written word is a reference — the callee is correct-by-deferral. |
| **Glob `namespace import ::mod::*`** | Names no single command; introduces no link. |
| **Glob `info commands PATTERN`** | Names no single command; no probe reference. |
| **`load LIB`** | No Tcl-source definition exists to point at. |
| **`thread::send` / `comm send` / Tk `send`** | The script resolves in the *target* interpreter's global namespace. |
| **Custom C resolvers** (`Tcl_SetNamespaceResolvers`) and `namespace unknown` handlers | Out of scope for static resolution; the runtime's fallback fires only after the rule misses. |
| **Values reachable only through folding** (`string map`, computed dict keys) | No source span exists to rewrite, so they contribute no reference. |
| **Members installed reflectively** | `member_set_incomplete`; the member table is a lower bound (§5.7). |
| **Metaclass with a dynamic or unproved head** | No actual class allocation or constructor permission follows. Independently retained source schema can describe syntax without establishing execution. |
| **`::tcl::prefix`, `::tcl::clock` ensemble internals** | Unmodelled by choice — ensemble-backing implementation detail, not documented public surface. |

Two known asymmetries worth naming because they look like bugs and are not:

- **References follow a link; rename does not rewrite the far side's local
  usages.** An imported or aliased *call* names the local command, which
  keeps its own name across a source rename (§3.3).
- **Definition returns one entry; references and rename cover the override
  family.** Go-to-definition answers the implementation the call actually
  enters; rename must cover every polymorphic sibling or it breaks dispatch
  (§5.3).

---

## 9. Where the pieces live

| Area | Home |
|---|---|
| Written-name canonicalisers, key helpers, candidate order, conformance vectors | `rust/tcl-syntax/src/naming.rs` |
| MRO linearisation | `rust/tcl-syntax/src/mro.rs` |
| Expr-function name/version table | `rust/tcl-syntax/src/expr/mathfunc.rs` |
| Scope tree, settlement, `command_resolution_namespace` | `rust/tcl-compiler/src/analyser/scope.rs` |
| Declaration handlers, interp domains, links | `rust/tcl-compiler/src/analyser/handlers.rs` |
| Invocation recording, reference roles, const dispatch | `rust/tcl-compiler/src/analyser/commands.rs` |
| TclOO members, export state, per-object methods | `rust/tcl-compiler/src/analyser/oo.rs` |
| Class hierarchy, MRO map, method providers | `rust/tcl-compiler/src/analyser/class_hierarchy.rs` |
| Object→class lattice, ⊤ taxonomy | `rust/tcl-compiler/src/analyser/class_lattice.rs` |
| Flow-sensitive constant provenance | `rust/tcl-compiler/src/value_provenance.rs` |
| Compiler place/alias layer | `rust/tcl-compiler/src/var_resolve.rs` |
| Target selection for every provider | `rust/tcl-lsp-core/src/definition.rs` |
| Original source/schema selection and complete operand correspondence | `rust/tcl-compiler/src/registry_invocation/source_structure.rs`, `conditional_metadata.rs`, `conditional_vendor_metadata.rs` |
| Current readonly declaration/URI identity | `rust/tcl-lsp-core/src/original_declaration.rs`, `original_indexed_location.rs` |
| Original receiver/member selection | `rust/tcl-lsp-core/src/receiver_identity.rs`, `method_symbol.rs`, `original_member_rename.rs` |
| Workspace oracle, links, method dispatch chain | `rust/tcl-lsp-core/src/workspace_index.rs` |
| Import/export lifecycle decisions | `rust/tcl-lsp-core/src/namespace_import.rs` |
| Source/`package require` run order | `rust/tcl-lsp-core/src/source_graph.rs` |
| Autoload tier, source re-homing, seed identities | `rust/tcl-lsp-server/src/lib.rs` |
| Dialect knobs (`namespace_var_global_fallback`, version gates) | `rust/tcl-registry/src/dialects.rs`, `commands/tcl/` |
| VM dispatch, epoch cache, interpreter tree | `rust/tcl-vm/src/interp.rs` |
| WASM runtime namespace/variable homes | `runtime/rust/src/namespace.rs`, `vars.rs` |
| Drift gate | `rust/xtask/src/resolution_drift.rs` |

## Related

- [contracts/command-resolution.md](../contracts/command-resolution.md) — the
  selected lookup owners, reporting rule and bounded conformance gates.
- [contracts/naming-consumer-matrix.md](../contracts/naming-consumer-matrix.md)
  — current shared APIs, independent authority and remaining consumer limits.
- [compiler/name-resolution-implementer-guide.md](../compiler/name-resolution-implementer-guide.md)
  — Original input, schema, frame, storage and edit receipts with usage examples.
- [name-resolution-c-conformance.md](name-resolution-c-conformance.md) — the
  C algorithm and the 8.4 → 9.1 version matrix.
- [contracts/cross-file-diagnostics.md](../contracts/cross-file-diagnostics.md)
  — the shared cross-document settlement lookup and its abstention gates.
- [import-order-source-graph.md](import-order-source-graph.md) — the run
  order both wildcard-import tiers rank cross-document events with.
- [tcloo-object-typing.md](tcloo-object-typing.md) — the object-handle typing
  model feeding §5.6.
- [contracts/runtime-variable-frame-model.md](../contracts/runtime-variable-frame-model.md)
  — the variable / call-frame contract.
- [workspace-indexing.md](../contracts/workspace-indexing.md) — how the index
  that backs the workspace tier is built.
- KCS: [W314](../../kcs/codes/kcs-diagnostic-w314-no-absolute-name.md),
  [rename](../../kcs/features/kcs-feature-rename.md).
