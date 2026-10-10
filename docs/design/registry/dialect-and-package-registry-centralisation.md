# Registry, dialect and package query contracts

The registry describes available command surfaces and their semantic contracts.
The source command owner separately retains the implementation selected at an
actual invocation. Availability, package provision, compiler admission, runtime
identity and source-edit eligibility are independent facts.

Use the [resolved semantic query contract](../contracts/resolved-semantic-queries.md)
for positioned consumers and the
[implementer guide](../compiler/name-resolution-implementer-guide.md) for API
selection and uncertainty boundaries.

## 1. Registration and resolution owners

### 1.1 Registration inputs

`tcl_spectcl::evaluate_pack`, with `evaluate_pack_cached` for cache reuse, loads
`.tclspec` sources. Native Rust specifications and loaded pack declarations feed
the registry model. Inline stubs and sidecar `.stubs` enter through
`model::declaration::DocumentCommandSurface`; they are document declarations,
rather than attested native implementations.

| Model module | Owned contract |
| --- | --- |
| `model::surface` | `SurfaceDeclaration` and translation of compiled specifications |
| `model::context` | `ResolvedContext`, keyed version ranges, floors and assistance queries |
| `model::assembly` | `ContextRegistry`, provider-filtered immutable registry generations |
| `model::binding` | `BindingKnowledge`, `PackageStateMap` and package transitions |
| `model::registration` | Environment registration and declaration trust |
| `model::ingress` | Environment-name resolution and actual document environment inputs |
| `model::semantic` | Generation-bound semantic context |
| `model::declaration` | Document/workspace declarations and their provenance |

Declarations retain provider, version axis, applicability and provenance.
Document declarations use `Provider::Document` and the document version axis;
sidecars retain workspace-untrusted provenance. Their role assistance is combined
with the catalogue's answer. An untrusted addition cannot weaken a shipped
analysis or security fact or establish a native compiler hook.

A `ContextRegistry` generation retains its source specifications and provider
selection. Consumers use the generation's registry instead of reconstructing a
command table from a dialect label. Cache equality includes the semantic
profile, registry snapshot and overlay; a display name or matching fingerprint
is insufficient.

### 1.2 Resolution questions

| Question | Owner | Required distinction |
| --- | --- | --- |
| Written environment name or editor language ID | `model::ingress::resolve_environment`; strict validation via `resolve_known_environment` | A lenient default is not proof that an unknown name is valid |
| Document analysis environment | `DocumentEnvironment`, `ResolvedContext`, actual profile and registry inputs | Authoring surface, lexer grammar, native runtime and embedding policy remain separate axes |
| Written command lookup order | `tcl_syntax::naming::command_resolution_candidates` | Actual namespace/path and slot existence; candidates alone prove no binding |
| Positioned command identity | `SourceCommandBindings::invocation_at_source`, `CommandBindingRealm` | Called slot, terminal implementation, pre-argv compiler selection and post-argv handler differ |
| Package version acceptance | Axis-typed `VersionSet` algebra and the selected package policy | Tcl release and package version axes cannot be interchanged |
| Package index or autoload metadata | `PackageResolver` | An advertisement does not establish that a loader executed or provided the package |
| Runtime package state | `PackageStateMap` and package transitions | Available, advertised, required and provided states differ |
| Registry semantic hook | Selected declaration plus purpose-specific retained binding adapter | Catalogue presence supplies assistance; execution and edits require additional proof |

Assistance queries can enumerate possible declarations under the authoring
context and floors. A diagnostic that asserts identity, an executable rewrite,
or code generation requires the corresponding positioned semantic receipt.
`Absent`, `May` and `Unknown` cannot be treated as a unique live implementation.

| Query | Owner | Required distinction |
| --- | --- | --- |
| Written environment name | `model::ingress::resolve_environment` | Canonical names, aliases and editor identities share one ingress. |
| Document context | Detection facts, overlays and targets | Retain environment generation, overlay and target axes. |
| Call-site binding | `Analyser::command_existence_oracle`, realm `knowledge_at` | `Absent`, `Must`, `May` and `Unknown` remain positioned verdicts. |
| Package requirements | `VersionSet`, floor engine and `PackageResolver` | Assistance candidates do not establish installed packages. |
| Semantic hook | `ResolvedContext::resolve_spec` and retained invocation owner | Availability alone does not prove command binding or trust. |

## 2. Consumer purposes

### 2.1 Front end

LSP environment ingress uses `model::ingress`. Availability, option and
subcommand queries use the resolved document context. The document's retained
profile and registry are the semantic inputs; a dialect display label does not
recreate them.

Tk illustrates three separate questions: `ambient_package("Tk")` records an
ambient placement, `can_host_package("Tk")` records placement capability, and
`package_active("Tk")` answers availability under the selected policy. None
proves that an arbitrary native Tk command or package loader is installed at a
source invocation.

Navigation, references and edits consume exact source/allocation receipts.
Final workspace or document name maps provide declaration assistance and cannot
replace temporal identity after rename, deletion or redefinition. Foreign source
identity does not supply an editable span in the current document.

### 2.2 Compiler and analyser

Compiler/analyser environment entry points use `environment_ingress` and retain
the actual selected profile, lexer and registry. `ResolvedContext` answers
catalogue availability and version questions. Command transfer consumes original
source bindings and the registry's typed roles, effects, completion and body
contracts.

Normal-result analysis, possible effects and executable erasure have different
projections. A normal handler can support result or footprint analysis without
admitting a compiler opcode or proving observer absence. Side-effect assistance
retains its authored hint projection rather than assuming the visible winning
specification owns every possible hint.

### 2.3 Runtime and backends

Both engines (`tcl-vm`, `runtime/rust`), the engine adapter
(`tcl-engine-tclvm`), the two VM-driving hosts (`tcl-vm-cli`,
`tcl-debugger`) and the `xtask` sweeps and generators that exercise them
resolve every dialect **name** through the seam and reach the registry
as per-environment `ContextRegistry` generations, through one small
`environment` module each: `profile_for_dialect`
(`resolve_environment(…).unit_profile()`), `store_for_profile` /
`store_for_dialect` where provided (the generation's command store), and
`surface_point` in the engines (the resolved environment's document
`SurfaceQuery`). Both engines
resolve the point *and* the store once per profile pin and cache them on
the interpreter, because the builtin-surface gate is consulted on every
command resolution. `codegen_abi`'s three raw name ingresses are
`resolve_known_environment`, keeping the fail-closed decline. Release-based
constructors use the closed release set `TclVersion::dialect_profile_name`
spells and the fixed `f5-irules`/`tk`/`expect`/`f5-iapps` projection targets.
Both engines also resolve explicit `RuntimeContext` pins through
`environment::pin_context`, including registered environments and overlays;
an overlay that has not been installed is an error.


VM and standalone runtime use live command/package tables under their actual
native environment. Registry visibility is a surface gate; it cannot replace
live command lookup. `codegen_abi` uses strict known-environment resolution for
native admission. Missing runtime policies or dependencies decline at the host
capability boundary rather than becoming Tcl errors or synthetic values.

Native, WASM and BPF selection retain their own supported-subset and dependency
contracts. A registry command that is visible in one native family is not
automatically an execution obligation for every backend. Safe-interpreter
command filtering uses `safe_interp_hidden_commands` narrowed by the commands
that the interpreter actually carries.

### 2.4 Tooling and generated views

The two CLIs (`tcl-cli`, through the shared `tcl-cli-support`), the MCP
server (`tcl-mcp`), the spec studio (`tcl-spec-studio`) and the pack
loader's name ingress (`tcl-spectcl`) resolve every dialect **name**
through the seam and answer availability from the resolved environment's
`ResolvedContext`. Their small `environment` modules expose the ingress
forms each consumer needs. `tcl-cli-support` supplies
`profile_for_dialect`, `known_profile_for_dialect` (the validator),
`context_for_dialect` (the assistance view), and
`analyser_profile_for_dialect`. The pack loader supplies
`profile_for_dialect`, `catalogue_profile_for_dialect`, and `lenient_store`.
The CLI's analyser-profile form and the pack loader's profile form use
`DocumentEnvironment::analyser_profile`: the KCS help filter and the
pack-carrying registry cache key deliberately sink `tk` to the permissive
fallback rather than promoting it. 

CLI, MCP, SpecTcl and Studio environment ingress uses `model::ingress`.
`ResolvedContext` supplies command/option/keyed-range assistance. Selectable
labels come from the environment registry; a catalogue identity enumeration is
a different query. The studio's catalogue browsing projection includes only
environments with a catalogue profile.

Generated command, callback and editor views expose registry declarations.
They do not attest live native registrations, provider installation or execution.
Their drift checks compare the rendered view with its owning input rather than
making the rendered table a second semantic authority.

## 3. Declaration trust and uncertainty

Stubs, catalogue declarations, package advertisements and native execution
receipts have different provenance. Only an independently retained native or
provider contract can establish compiler/runtime prerequisites. Missing loader,
worker, source-origin or observer evidence stays unknown.

`RegistrySnapshot::semantic_key` retains immutable structural identity without
reader memo caches. Allocation/source carriers retain this semantic key; actual
queries keep the snapshot reader separately. A fingerprint selects a candidate
cache bucket and never proves structural equality.

## 4. Relations and security

`tcl_registry::relation` owns requires/conflicts evaluation: `Relation<T>`,
`RelationTermKind`, `RelationFactSource`, `evaluate` and `closure_over`.
`OptionTerm`/`OptionFacts` and `ProfileTerm`/`ProfileFacts` are separate domains.
`RelationMode` distinguishes an asserted prerequisite from an inferred parent;
for example, a configured HTTP profile can infer TCP rather than diagnose its
absence as an explicit configuration error.

`tcl_registry::security_floor` prevents an override from deleting a shipped
security relation or clearing a shipped `TAINT_SINK` fact. F5 contracts retain
`BigIpExecutionContext` and cited `EmbeddedRuntimeEvidence`. The committed F5
corpus asserts those authored contracts; it is not itself a live appliance or a
replacement for an independently attributed observation.

## 5. Contract checks

| Check | Contract exercised |
| --- | --- |
| Registry/model parity tests | Context-filtered declarations agree with the compiled catalogue |
| Document grammar agreement and `dialect-drift` | Actual document ingress paths select the same grammar |
| SpecTcl golden snapshots | Shipped packs produce their declared registry snapshots |
| SpecTcl static-fast-path parity | The shortcut and interpreted loader produce equal snapshots |
| `tcl spec upgrade --verify` | Original and rewritten packs have equal registry and environment effects |
| Security-floor tests | Overrides preserve shipped security facts |
| F5 corpus tests | Authored F5 contracts agree with their cited observations |

These checks cover their declared inputs. They do not establish every editor
consumer, live object hook or backend dependency; each consumer still requires
its own positive and withdrawal controls.

## 6. `tcl spec upgrade`: source compatibility

The loader accepts published 1.x packs; this tool rewrites supported 1.x
source vocabulary to 2.0.
The rewriter is `tcl-spectcl/src/upgrade.rs`, driven by `tcl spec upgrade`
with `--from` / `--to` / `--check` / `--verify` / `--restyle`; the 2.0 word
set it targets is `tcl-spectcl/src/loader/available.rs`.

1. **It is a source rewriter, never a load-render round-trip.** Edits are
   content-range replacements located by the loader's own lexer (the
   `speclib_version_span` discipline: same lexing, same BOM handling,
   applied back-to-front, never reformatting), so author layout, comments,
   and delimiters survive and diffs are reviewable. `hook` bodies are never
   descended into.
2. **The 1.x dialect vocabulary is closed**: 13 dialect names + `all-tcl`
   + `tcl8.x` + five `tclX.Y+` forms — 21 tokens. The translation table is
   total; a word outside it refuses the *file* rather than being carried
   through unread.
3. **The loader's per-site vocabulary log inverts**: the machinery that
   notices "this word is newer than your declaration" also tells the
   upgrader which sites force 2.0 and proves the rewritten pack needs
   nothing newer than it declares.

Supported operations:

- `--check`, skip-on-no-`speclib`, refuse-on-non-vocabulary-word,
  reload-after-write proof, exit 1 on remaining work.
- The version word moves to `2.0` only when the body rewrite completed on
  that file; a file with any row left keeps its 1.x header and reports
  *partially upgraded*.
- `dialects` / `-dialects` → `available` at every loader site, through
  the total table: `tclX.Y` → single point, `tclX.Y+` → open range,
  `all-tcl` → `{tcl 8.4-}`, `tcl8.x` → `{tcl 8.4-9.0}` (exclusive maximum
  stated), `f5-irules` → the core family, `tk` → `{package Tk}` on Tk's
  own axis, `f5-bigip` → error (it leaves the Tcl axis). Output reuses the
  renderer's shorthand logic so upgraded packs read hand-written.
- Role discrimination against the live environment registry: a membership
  token whose environment declares exactly one ambient package translates
  to that provider (`f5-iapps` → `{package f5-iapps-cmds}`, `f5-tmsh` →
  `{package f5-tmsh-cmds}`, `expect` → `{package Expect}`), and the
  loader's `available` reader carries the exact inverse, so the translated
  spec is byte-equal. `spectcl` and `bpf` stay markers: their environments
  declare no ambient provider (their surfaces are compiled), so no
  `available` row can carry the claim, and the marker names that reason.
- `ambient_package NAME VERSION` rehomes into
  `environment OWNER -extend { ambient NAME VERSION }`, where OWNER is
  the pack's sole declared `environment` block, else its sole membership
  token across `dialects` rows; an ambiguous pack (or a non-plain version)
  keeps a `# TODO(spectcl 2.0):` marker and reports partial. Never guess.
- `file_extension … -dialect D` moves its detection into
  `environment D -extend { … }` (the flag dropped, everything else
  verbatim); an unresolvable `D` keeps the marker.
- `--infer-provides` (off by default) hoists a uniform `required_package`
  (the pack-level default, else one identical row in every command) to a
  pack-level `provides`, whose loader semantics keep the snapshot
  byte-equal.
- Post-rewrite proof through the vocabulary log: the rewritten file is
  re-loaded and every site needing a vocabulary above its own declaration
  is reported.
- `--verify` compares `command_entry_json` snapshots of the original and
  the rewritten pack across every dialect a 1.x row can gate on — the Tcl
  ladder, `f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`, `spectcl`, `bpf` —
  plus the **environment-effect snapshot**
  (`upgrade::environment_effect_snapshot`): the scoped detection and
  placement rows both forms load to, which is what licenses moving a
  row's home while proving the registry effect stayed put.
- Explicit `--from` / `--to`; downgrades are refused (an unsupported major
  fails closed, so a 2.0 → 1.x rewrite would be a silent capability loss).
- `--restyle`: after the row rewrite, the pack is re-emitted through
  `export_pack` in canonical form — straight-line registration calls at
  the house layout, comments and author layout dropped — and `--verify`
  proves the restyled snapshot as it proves the plain one. A
  **programmed** pack is refused whole (`available?` at registration, or a
  top-level statement that is not one of the recorded registration
  calls); a partial upgrade keeps its TODO markers and is not restyled.

`sdc_base.tclspec` stays at `speclib … 1.1` as the live 1.x corpus the
tool and the loader are exercised on; the six EDA packs and `upf` declare
2.x.

## 7. Shared name-resolution vectors

### 7.1 Reference interpreters

`tcl-test-support` discovers the requested exact C Tcl releases and current
pinned Jim. Explicit `TCL_LSP_TCLSH84`, `85`, `86`, `90`, `91` and Jim paths retain
interpreter provenance. Required discovery validates the reported release and
fails on a missing, malformed or stale interpreter. An optional local run is
explicitly partial; missing capability is different from missing interpreter.

`ensure-test-deps.sh`'s `ensure_tclsh` provisions the five pinned
interpreters declared in `rust/tcl-dialect/data/reference-toolchains.tsv`.
`scripts/dev/tcl-reference-toolchains.sh` validates each source/build tree
and publishes wrappers that execute its `unix/tclsh` with the matching
library. `TCL_LSP_TCL_BIN_DIR` selects the wrapper directory; otherwise the
helper chooses `~/.local/bin` or `/usr/local/bin` according to the
available writable PATH location. An existing build is reused only when
its reported `info patchlevel` matches the pinned patchlevel.

`tcl_test_support::locate_tclsh(TclVersion)` validates the interpreter's
exact pinned patchlevel. The `tcltest_sweep` instead binds the pinned Tcl
9.0 source tree through `source_tree_for_sweep` and
`tcl_test_support::locate_source_tree`, then obtains its interpreter with
`tclsh_from_source_tree`. This keeps the test sources and executable from
the same validated tree.

`audit_option_dialects` calls
`require_all_tclsh_built` before probing and fails immediately listing
any missing binary (`AUDIT_ALLOW_MISSING_TCLSH=1` is the documented escape
hatch for a deliberately partial run). `fetch_tcl_source.sh` fetches the
Tk trees (`tk84` … `tk91`, `tkall`) beside the Tcl ones, and
`session-start.sh` runs `tkall`.

### 7.2 Vector files and expectation columns

| File | Domain | Renderer / query owner |
| --- | --- | --- |
| `command_resolution_vectors.txt` | Namespace/path command lookup | `tcl_syntax::naming::conformance` |
| `variable_resolution_vectors.txt` | Variable, alias and frame lookup | `tcl_syntax::var_conformance` |
| `namespace_op_vectors.txt` | Namespace operations and native errors | `tcl_syntax::ns_op_conformance` |
| `command_binding_vectors.txt` | Registration, mutation and command identity | `tcl_syntax::execution_conformance` |
| `package_lookup_vectors.txt` | Package advertisements, loaders and provision | `tcl_syntax::execution_conformance` |
| `autoload_vectors.txt` | Autoload lookup and explicit feature absence | `tcl_syntax::execution_conformance` |

Files live under `rust/tcl-syntax/tests/data`. `vector_ops` owns shared pipe-row
and setup-operation syntax. `release_expectations::PerRelease` accepts a common
expectation or a range-tagged expectation for C8.4, C8.5, C8.6, C9.0 and C9.1.
Range entries cover the release set exactly once; semicolons inside an expected
value remain data unless followed by a range token.

Executable vectors have five columns: identifier, required Jim operations,
script, per-C-release observable and independently measured Jim observable.
`ExecutionVector::script` captures completion and result together. A
`!UNSUPPORTED` expectation is checked by invoking the operation; it is not an
instruction to skip an assertion. `JimCapability` measures operation availability
without treating Jim as a C release. C imports retain command-token relationships
through rename; Jim imports can follow later target names, so their columns are
independent.

- `tcl_syntax::release_expectations::PerRelease` parses an expectation
  field that is either one value every release shares or a
  `RANGE=VALUE;RANGE=VALUE` list over the ladder 8.4, 8.5, 8.6, 9.0, 9.1
  (`8.4-8.6=…`, `9.0+=…`). The entries must cover the ladder exactly once,
  and a `;` only separates entries when a range token follows it, so
  expectation values may contain semicolons.
- `tcl_syntax::vector_ops` holds the row syntax the domain files share:
  pipe-split rows and the `kind(argument)` setup mini-ops, split on
  parenthesis depth so an argument may contain commas and nested
  parentheses.
- `rust/tcl-syntax/tests/support/mod.rs` is the five-binary matrix, keyed
  `TCL_LSP_TCLSH84` … `TCL_LSP_TCLSH91` with PATH fallbacks; it uses `tcl_test_support` to verify
  the interpreter's reported patchlevel matches the pinned release,
  and skips a missing release loudly. The three suites
  (`command_resolution_conformance`, `variable_resolution_conformance`,
  `namespace_op_conformance`) run every row against every available
  release and assert that release's own column.
- The variable and namespace-op observable is the two-element list
  `<catch code> <result-or-message>`, with the whole row inside the
  `catch`, so a row that uses a subcommand a release does not have states
  that release's real error text rather than being excluded. The command
  domain keeps its `-` sentinel for `invalid command name` and `!ERROR`
  for a scenario a release cannot set up at all.
- Each file carries a `documented-non-conformance` block naming the
  `knownBug` rows deliberately excluded (`namespace-56.4`, `info-15.8`,
  `interp-27.5–27.8`, `var-3.3`/`var-3.4`'s `testupvar` constraint, and
  `namespace-51.13`'s mid-teardown observation) — so nobody "fixes" the
  model to a behaviour real Tcl does not exhibit.

The same rendered bytes can be consumed by hermetic owners, source analysis,
VM/runtime dispatch and real-interpreter tests. Using a shared fixture does not
prove all those consumers are wired or that a source edit is equivalent. Rewrite
comparisons observe exit status, stdout and primary error text; cache identity,
trace counts, loader effects, error options and other state need explicit
observations when they belong to the contract.

### 7.3 Native library boundaries

`init.tcl`, `package.tcl`, `tm.tcl` and `safe.tcl` are release-specific executable
native-library inputs. Their unknown/autoload ordering, index discovery, module
paths and safe-interpreter behaviour cannot be inferred from a package name or
registry advertisement. A bounded trusted loader recipe supplies only its
retained prerequisites and effects; opaque or unprovided loaders remain unknown.

### 7.4 Evidence scope

Committed deterministic vectors provide reproducible expected inputs. Live
interpreter comparisons establish only their actual binaries, entry protocols
and observed state. Full native-test replay and appliance observations are
separate evidence sources. The BIG-IP corpus cites appliance transcripts through
`EmbeddedRuntimeEvidence`; ordinary repository tests do not contact an appliance.
No generated declaration or simulator run independently establishes F5 semantics.
