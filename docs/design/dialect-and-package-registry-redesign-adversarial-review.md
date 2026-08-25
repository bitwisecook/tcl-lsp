# Adversarial review: dialect/package/environment registry redesign ([#1631](https://github.com/bitwisecook/tcl-lsp/issues/1631))

> **Review target:**
> [`dialect-and-package-registry-redesign.md`](dialect-and-package-registry-redesign.md)
> at `1c9c62461813bfa7e635757eba6d22610e390af2`
>
> **Verdict:** request changes. The dialect/package/environment separation is
> the right direction, but P0 is not ready to ratify and P1/P3/P5 must not begin
> from the current data model.

## Executive verdict

The proposal correctly identifies the catalogue's category error: Tk, tcllib,
Expect, EDA libraries, and iApps are not new Tcl lexical grammars merely because
they add commands. It also correctly wants one source of truth for names, a
positive closed-world model for iRules, provider-relative versioning, and a
declarative route out of native per-package Rust specs.

The proposed replacement nevertheless collapses four different things into one
`ResolvedContext`:

1. the source language's syntax and core evaluation semantics;
2. an installation or build's capabilities;
3. an environment's expected package catalogue; and
4. the mutable command and package state of each Tcl interpreter at a point in
   execution.

Those are not interchangeable. The distinction is observable in upstream Tcl,
JimTcl, Tk, tcllib, and picol source, and in the experiments below. The most
serious consequence is that the proposed provider-resolution rule can attach
the wrong arity, taint sink, side effect, lowering, or codegen hook to a command
whose runtime binding was replaced by a package script, import, alias, or
`rename`. For hover this is misleading; for taint analysis and compilation it
is unsound.

The minimum safe redesign is:

```text
ProjectEnvironment
  ├── language core: family + release + build/capability profile
  ├── target VersionSet per version axis
  ├── expected/hosted package catalogue
  ├── editor routing identity (from a fixed contributed set)
  └── policy defaults

AnalysisWorld
  └── InterpreterId → RealmState
        ├── package state: unknown / available / loading / provided(version)
        ├── visible and hidden command bindings
        ├── namespace imports, aliases, and renames
        ├── safe/trusted policy
        └── must/may candidates with provenance

ProviderCatalogue
  └── declarations of possible surfaces, version sets, and provenance
      (evidence for RealmState, never a substitute for RealmState)
```

This is not a demand for perfect dynamic execution. A conservative abstract
state is sufficient: retain a unique spec only when the binding is proved;
otherwise carry a candidate set and abstain from strong semantic claims. Much
of the required transition vocabulary already exists in
[`state_transition.rs`](../../rust/tcl-registry/src/state_transition.rs):
command bindings, interpreter topology, interpreter policy, child safety,
hide/expose, and widening should be integrated into this design rather than
discarded behind one document-global provider floor.

## Method and evidence boundary

This review attempted to falsify the proposal, not merely to find missing prose.
It used immutable upstream revisions, read implementation source rather than
README claims where possible, built two configurations from the same JimTcl
revision, ran Tcl 9.0.4 package/interpreter/binding probes, and reconciled the
proposal with the repository's existing external-library command-shape census.

The following revisions were inspected:

| Project | Revision | Why it is here |
|---|---|---|
| Tcl | `core-9-0-4`, [`c655b477`](https://github.com/tcltk/tcl/tree/c655b4770b1d6d32a8cbffd6cef59db6029fe19e) | package algebra, per-interpreter state, command replacement, safe interpreters |
| Tk | `core-9-0-4`, [`584f8fcf`](https://github.com/tcltk/tk/tree/584f8fcf62c320d7c341e77171188cb4d79c3725) | package names, loader relationship, build flags, Tcl compatibility |
| JimTcl | `0.84`, [`d5243a25`](https://github.com/msteveb/jimtcl/tree/d5243a25c488dfe751ef218828f13516e04ea2ba) | same-release build-dependent syntax/semantics and command surfaces |
| picol | [`5f902e9b`](https://github.com/antirez/picol/tree/5f902e9b211fff21fb6e6abfd116df665e4091d5) | embedded interpreter and mutable command-table stress test |
| tcllib | `tcllib-2-0`, [`2a63bf21`](https://github.com/tcltk/tcllib/tree/2a63bf212cc6423efb7a39958596ae2a2333c266) | runtime-created DSLs, object commands, callbacks, completion codes |
| ticklecharts | `v3.2.8`, [`b49f014c`](https://github.com/nico-robert/ticklecharts/tree/b49f014cb6c2412a7e661e805a37eaebcd625c59) | third-party version axes, foreign code, method-shaped sinks |
| pave | [`875de1f1`](https://github.com/aplsimple/pave/tree/875de1f1539d62e46e48e206f34f82848bca1736) | nested table DSLs and runtime-installed methods |
| SpiceGenTcl | [`e8aa45ce`](https://github.com/georgtree/SpiceGenTcl/tree/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec) | constrained options, TclOO methods, process/file sinks |

The JimTcl branch at
[`3760afa0`](https://github.com/bitwisecook/tcl-lsp/tree/3760afa0696149d2fa8ad9c36ed43b3e01ec4eac)
was treated as prior research to audit, not as ground truth. Its measured
release deltas are valuable. Its build matrix is not complete: the fetch path
builds one default configuration per release, and the
[`probe-jimtcl.sh`](https://github.com/bitwisecook/tcl-lsp/blob/3760afa0696149d2fa8ad9c36ed43b3e01ec4eac/scripts/dev/probe-jimtcl.sh)
script reduces many distinct failures to `ERR`. Finding B1 below is the missing
dimension.

The experiments are deliberately small. They demonstrate that a proposed
invariant is false; they do not claim to enumerate every Tcl implementation or
package behaviour.

## Blocking findings

### B1. `family × release` is not a total language-semantics key

The proposal makes grammar and the full expression surface total functions of
`(Family, Release)`, and says a family/release owns every character and expr
axis. JimTcl's build system falsifies that invariant at one release.

At JimTcl 0.84, [`auto.def`](https://github.com/msteveb/jimtcl/blob/d5243a25c488dfe751ef218828f13516e04ea2ba/auto.def#L18-L54)
defines independent `utf8`, `math`, `minimal`, `with-ext`, and `without-ext`
choices. Its extension table exposes independently selectable `json`,
`namespace`, `regexp`, `tclprefix`, `zlib`, and many other modules
([`auto.def` lines 65–101](https://github.com/msteveb/jimtcl/blob/d5243a25c488dfe751ef218828f13516e04ea2ba/auto.def#L65-L101)).
Without `JIM_UTF8`, Jim explicitly defines one byte as one character
([`utf8.h` lines 28–40](https://github.com/msteveb/jimtcl/blob/d5243a25c488dfe751ef218828f13516e04ea2ba/utf8.h#L28-L40)).

I built the same commit twice, once with `./configure` and once with
`./configure --minimal`:

```text
Jim 0.84 default:
patch 0.84 utf8len 1 json 1 tclprefix 1 zlib 1 sqrt 0 sqrt_result {2.0}

Jim 0.84 --minimal:
patch 0.84 utf8len 2 json 0 tclprefix 0 zlib 0 sqrt 1
sqrt_result {syntax error in expression: "sqrt(4)"}
```

The same family and release therefore has a different character model, expr
function acceptance, and command surface. The optional extensions are not
equivalent to Tcl `package require`: some are compiled in or out before an
interpreter exists.

**Required change.** Add an explicit implementation/build capability profile.
At minimum, make the core semantic key `(family, release, capabilities)` and
separate syntax capabilities from command capabilities. A named build profile
may inherit a measured default, but every non-default axis must be representable
and unknown builds must produce `Unknown`, not silently assume the default.

**Acceptance test.** Build Jim 0.84 default, `--minimal`, `--disable-utf8`, and
`--without-ext=default`; run the same lexer, expr, character, and command probes;
prove the resolved profile predicts every observed result. Re-run at one older
release to ensure the axis is not accidentally hard-coded to 0.84.

### B2. Provider/package state is per interpreter and temporal

The proposal resolves bytes to one context containing one active-provider
interval per document. Tcl stores package records in the interpreter's own
`packageTable`: the upstream type comment says exactly that
([`tclPkg.c` lines 53–65](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclPkg.c#L53-L65)).
`package require` may execute an arbitrary `ifneeded` script to provide the
package
([`tclPkg.c` lines 202–221](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclPkg.c#L202-L221)),
and `package unknown` may execute another script when the package is not known
([`tclPkg.c` lines 489–540](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclPkg.c#L489-L540)).

The Tcl 9.0.4 probe in Appendix A observed:

```text
before_require provide {} command 0 loads 0
require_result 1.0
after_require provide 1.0 command 1 loads 1
child_initial_demo 0 child_initial_package {}
parent_demo loaded parent_package 1.0 child_demo child child_package 2.0
parent_open 1 safe_open 0 safe_hidden 1 parent_tcl 9.0.4 safe_tcl 9.0.4
after_rename_command 0 package_still_loaded 1.0
```

Five distinct facts follow:

- activation changes over time;
- a child interpreter does not inherit the parent's package state;
- two interpreters can provide different versions and bindings;
- safe and parent interpreters can share the same Tcl core release while
  exposing different core commands; and
- package-provided state survives deletion of a command the package created.

The last point is decisive: `package provide Demo 1.0` is not proof that command
`demo` currently exists, much less proof of what it denotes.

**Required change.** Resolve a project environment first, then analyse a graph of
interpreter realms. Each realm needs temporal package and command-binding state.
Whole-file activation may remain an explicitly labelled assistance heuristic,
but compiler, taint, side-effect, and codegen decisions must query the
position-sensitive realm state. Unknown topology or dynamic scripts widen a
binding/package fact to `May` or `Unknown`.

**Acceptance test.** Add e2e fixtures for parent/child package divergence, safe
interpreter hiding, require-before/use-after ordering, command deletion after
provide, and an `interp eval` whose target is not statically known. Strong
semantic hooks must fire only in the realms and program points where their
binding is proved.

### B3. `Lifecycle` is not Tcl's package requirement algebra

The proposal reuses the existing `Lifecycle` as each `Provided.window`, while
also saying contexts carry an interval and `package require` requirements can
be sets. The existing type explicitly represents one introduced/deprecated/
retired history, uses plain dotted releases, and explicitly says package
prerelease labels require a different comparator
([`lifecycle.rs` lines 19–50](../../rust/tcl-registry/src/lifecycle.rs#L19-L50)).

Tcl requirements are not one lifecycle interval. The official
[`package` manual](https://www.tcl-lang.org/man/tcl9.0/TclCmd/package.html)
defines multiple requirements as alternatives, supports open-ended and bounded
ranges, and makes a bounded range's maximum exclusive; the same contract is
version-pinned in [`doc/package.n` lines 300–356](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/doc/package.n#L300-L356).
The local oracle showed:

```text
vsatisfies 8.6.16 8.5-9.0 1
vsatisfies 9.0 8.5-9.0 0
vsatisfies 9.0.4 8.5-9.1 1
vsatisfies 9.0.4 8.5- 1
vsatisfies 9.0.4 8.5 0
vsatisfies 9.0.4 8.5 9.0-9.1 1
```

Thus the prose example “8.5–9.0” is itself ambiguous: mathematical/editor UI
notation usually includes 9.0, while Tcl `8.5-9.0` excludes it. `TargetSpec::Set`
also contradicts the later claim that every provider context is an interval.

**Required change.** Keep two types:

- `Lifecycle` for an item's single-axis history and deprecation metadata; and
- a typed, comparator-aware `VersionSet` for requirements and targets,
  normalised to a union of half-open ranges (plus exact points if the comparator
  requires them).

Every version set must carry its axis/comparator; a Tcl core `Release` ordinal,
a TIP-style package version, a BIG-IP release, and an ECharts release must not
be comparable by accident. User-facing syntax must state bound inclusivity
unambiguously.

**Acceptance test.** Differentially test the Rust normaliser and all set
operations against Tcl's `package vsatisfies`, including disjoint unions,
exclusive maxima, open ends, exact requirements, alpha/beta labels, and
multi-component versions. Property-test `contains`, `intersect`, `subset`, and
normalisation idempotence.

### B4. Provider specificity is not Tcl command resolution

The proposal generalises “fewest dialect bits wins” to “narrowest provider set
wins” so same-name specs resolve deterministically. That can decide which
*declaration the registry author intended to override*. It cannot decide which
command Tcl will call.

Tcl permits command replacement; `Tcl_CreateObjCommand` deletes or replaces an
existing command of the same name in the target namespace
([`tclBasic.c` around command creation](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclBasic.c#L2770-L2815)).
Namespace imports can conflict or be forced, package loader scripts can import
or create commands, and `rename` changes the table again. The experiment in
Appendix A loaded two packages exporting `clash`:

```text
package_order_AB B package_order_BA A
first A conflict 1 message {can't import command "clash": already exists}
forced B origin ::B::clash
```

Loading the same providers in the opposite order changed the binding. An
ordinary import rejected a collision; `namespace import -force` replaced it.
No provider-window width can recover those runtime facts.

**Required change.** Keep pack-tier and provider-specificity precedence solely
as *catalogue authoring precedence*. Runtime resolution must use the shared
command resolver and realm transition state. When two active providers may bind
the same name and order is not proved, preserve both candidates. Completion may
offer the union; hover should show provenance/ambiguity; taint, lowering, and
codegen must take the conservative union of effects or abstain.

**Acceptance test.** Exercise packages that create the same global name, normal
and forced namespace imports, aliases, rename chains, later proc definitions,
and unknown `package ifneeded` scripts. Verify that changing only load order can
change the resolved spec and that an unknown order never selects one by static
provider specificity.

### B5. A package version does not determine one command surface

`provides NAME VERSION` is useful catalogue metadata, but the proposal treats it
as though resolving a package version activates a fixed surface. Tcl loads a
package by evaluating script. That script may inspect platform state, load a C
extension, source other files, import names, define only some commands, or fail
after making partial changes. `package forget` mutates the per-interpreter
database again
([`tclPkg.c` lines 1117–1153](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclPkg.c#L1117-L1153)),
and `package ifneeded` stores executable scripts rather than declarative
surfaces
([`tclPkg.c` lines 1156 onward](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclPkg.c#L1156-L1185)).

Real tcllib packages rely on that flexibility:

- `try` chooses `ifneeded`, immediate provide, and compatibility behaviour from
  the Tcl core version
  ([`modules/try/pkgIndex.tcl`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/try/pkgIndex.tcl));
- `snit` exposes parallel trains selected by the core requirement
  ([`modules/snit/pkgIndex.tcl`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/snit/pkgIndex.tcl)); and
- cryptographic modules conditionally select `tcllibc`, Cryptkit, or Trf
  accelerators at runtime
  ([`sha1v1.tcl` lines 505–520](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/sha1/sha1v1.tcl#L505-L520)).

The package metadata is therefore an expected/candidate surface, not a proof of
the live command table. The same distinction applies to core commands in safe
interpreters and to compile-time Jim extensions.

**Required change.** Name this boundary in the types. A `SurfaceDeclaration`
describes commands a provider may install under predicates; a `RealmBinding`
describes what analysis proves is installed here. Allow platform/build/feature
predicates on surface declarations. Activation evaluates declarative predicates
where possible and widens when the loader is dynamic. Never derive a proved
binding merely from `package provide` or a resolved package floor.

**Acceptance test.** Use a package whose `ifneeded` script conditionally defines
two different commands, one which partially mutates then errors, and one which
loads an optional accelerator. The analyser must distinguish expected, must-be-
present, may-be-present, and absent commands without executing untrusted loader
scripts.

### B6. SpecTcl cannot yet represent the packages P3–P5 promise to migrate

The proposal lists the DSL changes needed to round-trip today's native Tk specs,
then uses a byte-compared native/pack registry dump as the P3/P5 equality gate.
That gate is necessary, but it only proves that the new representation preserves
facts the old representation already had. It cannot prove that either model
describes the upstream library.

The repository's existing
[`external-library command-shape census`](spec-dsl-examples/external/README.md)
found sixteen concrete gaps. Several are structural Rust-model gaps, not merely
missing SpecTcl spellings:

- taint sinks, command forms, and deprecation replacements are command-only,
  while real sinks and form changes occur on object methods (G7/G15);
- directional option requirements are absent despite 189 measured
  `-require` uses in SpiceGenTcl (G2);
- foreign-language source values, nested structured tuple bodies, runtime-
  extensible closed sets, and per-object dynamic methods are not modelled
  (G6/G8/G9/G11);
- library-defined completion code 5 is meaningful inside `struct::tree walk`
  but cannot be scoped to that body (G13); and
- `SubSubCommand` documentation promises more shape than the type carries
  (G16).

The upstream source demonstrates why these are not theoretical. Tcllib's
`oo::dialect` explicitly constructs domain-specific languages, aliases the
current `oo::define` commands, and evaluates definition bodies in a generated
namespace
([`oodialect.tcl` lines 10–24](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/oodialect/oodialect.tcl#L10-L24),
[`48–124`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/oodialect/oodialect.tcl#L48-L124),
and [`183–194`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/oodialect/oodialect.tcl#L183-L194)).
`struct::tree` creates object commands with `interp alias`, discovers its method
set using `info commands`, and uses custom completion code 5
([`tree_tcl.tcl` lines 130–183](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/struct/tree_tcl.tcl#L130-L183),
[`200–220`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/struct/tree_tcl.tcl#L200-L220)).
Its `walk` body writes caller variables and interprets completion code 5 locally
([`WalkCall`](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/struct/tree_tcl.tcl#L2090-L2120)).
`struct::graph` adds a third dispatch level discovered by naming convention
([`graph_tcl.tcl` lines 1562–1590](https://github.com/tcltk/tcllib/blob/2a63bf212cc6423efb7a39958596ae2a2333c266/modules/struct/graph_tcl.tcl#L1562-L1590)).

Independent libraries hit the same model boundary in different ways:

- ticklecharts versions thousands of option declarations against the ECharts
  release axis
  ([`series.tcl` lines 6–30](https://github.com/nico-robert/ticklecharts/blob/b49f014cb6c2412a7e661e805a37eaebcd625c59/series.tcl#L6-L30)),
  stores JavaScript as a non-Tcl value
  ([`jsfunc.tcl` lines 6–46](https://github.com/nico-robert/ticklecharts/blob/b49f014cb6c2412a7e661e805a37eaebcd625c59/jsfunc.tcl#L6-L46)),
  and places a file-write sink on an object method
  ([`chart.tcl` lines 250–288](https://github.com/nico-robert/ticklecharts/blob/b49f014cb6c2412a7e661e805a37eaebcd625c59/chart.tcl#L250-L288));
- pave consumes nested seven-field widget tuples with recursively interpreted
  option/attribute lists
  ([`apavebase.tcl` lines 3476–3550](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L3476-L3550)),
  and installs computed methods on one object at runtime
  ([`apavebase.tcl` lines 2385–2411](https://github.com/aplsimple/pave/blob/875de1f1539d62e46e48e206f34f82848bca1736/apavebase.tcl#L2385-L2411)); and
- SpiceGenTcl requires Tcl 9 plus several packages
  ([`SpiceGenTcl.tcl` lines 17–26](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/SpiceGenTcl.tcl#L17-L26)),
  declares directional option constraints through `argparse`
  ([`specElementsClassesNgspice.tcl` lines 90–107](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/src/ngspice/specElementsClassesNgspice.tcl#L90-L107)),
  and puts process/file sinks inside `runAndRead` methods
  ([`specSimulatorClassesNgspice.tcl` lines 59–86](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/src/ngspice/specSimulatorClassesNgspice.tcl#L59-L86),
  [`108–146`](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/src/ngspice/specSimulatorClassesNgspice.tcl#L108-L146)).

This also refines the dialect/package rule: a package can create a nested DSL
without changing Tcl's outer lexical grammar. Such a surface is a package plus
a command-argument sublanguage descriptor, not a new dialect and not merely a
flat list of commands.

**Required change.** Make closure of the external census's `[STRUCT]` gaps a
prerequisite to claiming complete Tk/tcllib migration. Add an explicit
`DynamicSurface`/`UnknownMembers` escape hatch so a pack can honestly say that a
method set is runtime-extensible without pretending it is closed. Move semantic
fields shared by commands and methods into a common `InvocationSpec` or typed
capability components rather than copying individual fields down one level at a
time.

**Acceptance test.** Replace byte equality with two gates:

1. representation parity: native and pack serialisations agree for already-
   modelled facts; and
2. behavioural parity: fixture calls exercise completion, hover, semantic token
   roles, arity, control flow, taint, side effects, deprecation, and binding
   transitions for representative Tk and tcllib surfaces.

P5 must include `struct::tree`, `struct::graph`, `fileutil::traverse`, and
`oo::dialect` as adversarial acceptance modules, not only packages already
covered by native specs.

### B7. Runtime packs cannot create editor language identities dynamically

The proposal puts `editor_language_id`, filenames, and extensions in dynamic
pack/workspace environments, then says editor generators iterate the environment
registry. Build-time generators can see bundled environments; they cannot see a
future user's workspace pack.

VS Code language IDs, aliases, extensions, filenames, and first-line patterns
are extension-manifest contribution points, declared under
[`contributes.languages`](https://code.visualstudio.com/api/references/contribution-points#contributes.languages).
Zed likewise requires each language to exist in the extension's `languages/`
directory with `config.toml` and a grammar registered in `extension.toml`
([Zed language extension documentation](https://zed.dev/docs/extensions/languages)).
Neither mechanism lets an LSP server materialise arbitrary new editor language
identities from a pack after the extension is installed.

The proposed single resolver can still unify *server-side* names. It cannot make
all dynamic environment names valid editor ingress IDs or give them syntax
grammars and file associations automatically.

**Required change.** Split identity into:

- `EditorLanguageIdentity`: a fixed, build/install-time contributed set,
  generated from compiled and bundled data; and
- `ServerEnvironment`: dynamic and selectable through settings, directives,
  status UI, CLI, or MCP while documents retain a generic contributed Tcl
  language ID.

A pack may request patterns for server-side detection, but an editor adapter
must report whether it can apply them dynamically. Never promise a new language
ID where the host cannot register one.

**Acceptance test.** Install a released VSIX/Zed extension, then add a previously
unknown workspace environment without rebuilding the extension. Verify that the
document remains attached to the Tcl LSP under a contributed language ID, the
server selects the new environment, and UI text does not claim a new native file
type was registered.

### B8. Leaked-static specs make live reload unbounded

The proposal correctly changes dynamic `Environment` values to `Arc`, yet
explicitly leaves loaded `CommandSpec`s leaked-static while moving large Tk and
tcllib surfaces into reloadable packs. The current loader documents the leak
([`loader.rs` lines 48–51](../../rust/tcl-spectcl/src/loader.rs#L48-L51)) and
acknowledges that string interning only reduces growth; it does not end it
([`loader.rs` lines 475–523](../../rust/tcl-spectcl/src/loader.rs#L475-L523)).
Every generation still leaks command objects and their slices
([`loader.rs` lines 3666–3706](../../rust/tcl-spectcl/src/loader.rs#L3666-L3706)).

The registry records a `CommandSpec` size of 1,296 bytes and roughly 2,400
shipped specs
([`registry.rs` lines 958–980](../../rust/tcl-registry/src/registry.rs#L958-L980)).
As a lower bound, one complete dynamic generation is therefore about 3.1 MB
before nested slices and newly changed strings. One hundred Spec Studio edits
to a mass-migrated surface can leak roughly 311 MB of command objects alone.

This contradicts the stated live pack/config reload and Spec Studio workflow.
It becomes worse precisely when P3–P5 move the largest static catalogues into
packs.

**Required change.** Make generation ownership a P2 prerequisite, not later
cleanup. Keep immutable built-ins as true statics, but allocate dynamic pack
specs and all their nested data in an arena or `Arc<RegistryGeneration>`.
Queries return generation-bound handles/IDs rather than public `&'static`
references; dropping the last registry snapshot drops the entire generation.
Salsa keys should carry the generation ID already proposed for environments.

**Acceptance test.** Reload a large generated pack 1,000 times while changing
one detail field, force old snapshots out, and assert resident live generation
bytes plateau within a small constant number of generations. Run under a leak
detector or allocator accounting, not only RSS.

### B9. Nearest-wins workspace data is a security boundary, not just precedence

Today the pack loader discovers `.tclspec` files from the workspace and gives
nearer tiers precedence
([`discovery.rs` lines 19–45](../../rust/tcl-spectcl/src/discovery.rs#L19-L45),
[`pack.rs` lines 19–35](../../rust/tcl-spectcl/src/pack.rs#L19-L35)). A workspace
pack can explicitly replace a shipped same-name command using `-override`; the
loader reports the replacement but permits it
([`pack.rs` lines 651–693](../../rust/tcl-spectcl/src/pack.rs#L651-L693)). The
proposal adds environment policy, provider availability, taint, side effects,
and eventually executable hooks to this precedence system without defining a
trust model.

That lets a repository-controlled `.tcl-lsp/*.tclspec` suppress or weaken the
analysis facts that warn about that repository's code. A malicious pack could
replace a shipped sink spec with a harmless one, open a closed-world
environment, change a target, or alter a hook. A warning in the pack file is not
an adequate boundary when the user is reviewing some other source file.

VS Code's
[`Workspace Trust` guidance](https://code.visualstudio.com/api/extension-guides/workspace-trust)
exists specifically because workspace files and settings may be authored by an
attacker; trust-sensitive extension features must respect restricted mode.

**Required change.** Every declaration and resolved fact needs provenance and a
trust class. At minimum distinguish built-in, signed/bundled, user-trusted,
workspace-trusted, workspace-untrusted, and live Studio override. In an
untrusted workspace:

- additions may improve syntax colouring, completion, and documentation;
- workspace data may not remove or weaken built-in taint, side-effect, safety,
  closed-world, or codegen facts;
- native or Tcl hook execution is disabled;
- overriding a canonical environment or shipped command requires explicit
  trusted opt-in; and
- diagnostics and hover expose the winning fact's provenance.

Security facts should merge monotonically unless the user explicitly trusts the
override. “Nearest wins” may remain the editing model for non-security prose;
it must not be the security lattice.

**Acceptance test.** Open an untrusted fixture whose workspace pack attempts to
remove an `exec`/`open |...` sink, replace a built-in command, and relax iRules
closed-world policy. The same security findings must remain. Trust the fixture
explicitly and verify the UI discloses the changed provenance before allowing
the override.

### B10. Parse-at-maximum plus endpoint detectors is not a sound range proof

For multi-target projects, the proposal parses once under the range maximum,
runs detectors only for axes differing at the endpoints, and compares constant
folding at the endpoints. This is an optimisation presented before the semantic
invariant it must preserve.

There are three independent holes:

1. `TargetSpec::Set` can be non-contiguous, so there may be no meaningful
   interval whose endpoints characterise the selected releases.
2. A grammar axis can change and later change back. Equal endpoint values do
   not prove equal values in the interior. The design calls the axis set
   extensible, so monotonicity cannot be assumed forever.
3. Parsing with the accepting grammar can consume/recover tokens differently
   from an older grammar. A post-lex detector only works if it is proved
   complete for every affected axis and retains all spans/structure needed to
   emulate the other parse. Jim's quote termination, brace continuation,
   variable syntax, and list parsing are concrete axes where the whole word
   structure, not one numeral token, can differ.

This review did not find and does not claim a current Tcl 8.4–9.1 example whose
endpoint values hide an interior A→B→A change. The blocker is the missing proof
obligation: the proposed algorithm does not remain correct as the family ladder
or a set target grows.

**Required change.** Define correctness first: compatibility means the relevant
parse and semantic facts agree for every selected target. Implement it by
evaluating every distinct `GrammarId`/semantic profile represented in the
finite `VersionSet`, deduplicating releases with identical profiles. Preserve a
token-spanned parse per distinct grammar when structure differs. Targeted
detectors may optimise a profile pair only after differential corpus/fuzz tests
prove equivalence to the reference multi-parse for that pair.

Make `primary` explicit for a range/set. “Maximum is usually a superset” is not
a contract, and assistance policy must not silently choose a release whose
grammar rejects or changes the project's intended baseline.

**Acceptance test.** For every family ladder, enumerate every selected release
and compare the optimised compatibility result with the reference result from
per-distinct-profile parsing. Include non-contiguous sets and a synthetic
A→B→A test axis so endpoint-only regressions fail even if current Tcl happens
to be monotone.

### B11. Tk 9's package names are executable compatibility, not a plain alias

The proposal's `alias PACKAGE NAME` example treats `Tk` and Tk 9's lowercase
`tk` as equivalent spellings, and placement examples suggest Tk can
`tracks-base`. Tk 9.0.4's source is more specific:

- its generated `pkgIndex.tcl` registers lowercase `tk` as the package that
  loads the library, then registers uppercase `Tk` with an `ifneeded` script
  which requires that exact lowercase version
  ([`unix/Makefile.in` lines 796–813](https://github.com/tcltk/tk/blob/584f8fcf62c320d7c341e77171188cb4d79c3725/unix/Makefile.in#L796-L813));
- the C initialiser always provides `tk`, but only provides `Tk` when
  `TK_NO_DEPRECATED` is not defined
  ([`tkWindow.c` lines 3457–3469](https://github.com/tcltk/tk/blob/584f8fcf62c320d7c341e77171188cb4d79c3725/generic/tkWindow.c#L3457-L3469)); and
- Tk has its own 9.0.4 version constants and only rejects Tcl major versions
  below 9; it does not derive its patch release from the Tcl interpreter
  ([`configure.ac` lines 26–43](https://github.com/tcltk/tk/blob/584f8fcf62c320d7c341e77171188cb4d79c3725/unix/configure.ac#L26-L43)).

Thus uppercase is a version- and build-sensitive compatibility provide backed
by executable loader logic. It is not merely another static name for one
provider, and Tk's release train is not universally identical to the Tcl core
train.

**Required change.** Model canonical package identities, co-provides, and
loader/dependency aliases separately. A compatibility alias may state “requiring
A requires exact B, and successful load is expected to co-provide A”, with a
build predicate. Keep Tk on its own version axis and express compatibility with
Tcl as a requirement relation, not `TracksBase`, unless an individual host
environment truly guarantees matched versions.

**Acceptance test.** Test Tcl/Tk 8.x and 9.x package indexes, a Tk 9 build with
and without `TK_NO_DEPRECATED`, `package require tk`, `package require Tk`, and
mismatched Tcl/Tk patch releases. The model must predict both package records
and command surface without normalising away the lowercase transition.

### B12. “Parse-level restriction” is too broad a dialect criterion

The classification rule admits a dialect for command-use restrictions enforced
at parse level. That makes implementation location determine language identity:
moving the same allow-list check from an analyser pass into a parser would
change a package/environment into a dialect without changing the accepted byte
grammar.

The Tcl language parser's job is to divide commands into words and perform
substitutions; the command then interprets those words according to its own
contract (official
[`Tcl` language manual](https://www.tcl-lang.org/man/tcl9.0/TclCmd/Tcl.html),
version-pinned source at
[`doc/Tcl.n` lines 28–38](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/doc/Tcl.n#L28-L38)).
Safe interpreters demonstrate the separation: the parent and safe child in the
experiment both provide Tcl 9.0.4, yet `open` is visible in one and hidden in
the other. Command availability/policy is not a dialect even when statically
enforced.

Likewise, tcllib `oo::dialect` creates a definition language inside command
arguments while the outer Tcl grammar remains unchanged. That belongs in the
registry's argument/body grammar descriptors. iRules still qualifies as a
genuine family because it has observed lexical/expr differences such as the
ghost separator and word operators; its command ban list and closed-world
policy are additional environment facts, not the reason it is a dialect.

The proposal also says “if and only if” a variant owns a delta, then explicitly
allows Tcl 9.1 as a core release with no grammar delta. This is recoverable only
by distinguishing a *family* from releases on that family's target ladder.

**Required change.** Define:

- a language family by observable outer lexical/syntactic or core evaluation
  differences from another family;
- a release as a target point on an admitted family's ladder, whether or not
  every adjacent release changes grammar;
- a command sublanguage as registry data attached to an invocation; and
- availability, safety, and closed-world restrictions as realm/environment
  policy.

The classification gate must compare observable semantic fingerprints, not
which compiler module currently enforces a rule.

**Acceptance test.** Move a synthetic restriction between parser and analyser
implementations and prove its classification does not change. Verify a safe Tcl
interpreter remains family `tcl`, a Tcl package that defines a nested DSL remains
a package, and iRules remains a family when its command allow-list is removed
from the fingerprint but its lexical/expr deltas remain.

### B13. “Unknown word: warn and continue” is unsafe for semantic packs

The SpecTcl compatibility contract keeps one version-blind parser and says the
`speclib` version is only an author promise enforced by notices. The current
loader intentionally drops unknown properties, flags, traits, roles, colours,
dialects, and hooks while loading the rest of the pack
([`loader.rs` lines 35–47](../../rust/tcl-spectcl/src/loader.rs#L35-L47)).

That is tolerable for a missing help label. It is not tolerable when the unknown
word says “this argument is code”, “this method is a sink”, “this command
rebinds the command table”, or “this environment is closed-world”. An older
server would accept the command's partial spec and could issue a stronger,
safer-looking result precisely because it discarded the field it did not
understand. The loader-direction gate catches omissions in the current binary;
it cannot make an old installed binary understand future semantics.

**Required change.** Classify vocabulary by compatibility effect:

- presentation-only unknowns may warn and drop;
- unknown validation/assistance fields quarantine that invocation spec or mark
  the affected capability `Unknown`;
- unknown security, control-flow, binding, lowering, or codegen fields reject
  the affected command/pack from strong analysis; and
- an unsupported major `speclib` version fails closed, while a newer minor
  version is accepted only through declared feature/capability negotiation.

The notice must be surfaced on source files that consume the degraded spec, not
only on the pack file.

**Acceptance test.** Load a pack containing one unknown documentation word and
one unknown taint/effect word into an older fixture loader. The first preserves
the spec with a notice; the second prevents strong safety/codegen conclusions.
Add a downgrade test for every newly ratified semantic word.

## High-risk unresolved issues

### H1. Environment names, aliases, and overlays need a collision contract

The proposal makes environments the sole user-facing namespace while allowing
compiled, pack, user, and workspace definitions plus aliases, all with nearest-
wins precedence. Q18 reserves core family names but leaves the difficult cases
undefined:

- a pack's canonical name collides with another pack's alias;
- two aliases resolve to different canonical environments;
- a workspace defines `tk`, `f5-irules`, or a shipped EDA environment;
- an alias cycle appears;
- two environments claim the same extension or editor language ID; or
- a workspace overlay changes targets/ambient providers so the same canonical
  name means something different in two folders.

“Nearest wins” produces a deterministic answer, but not a stable identity. It
also makes diagnostics, caches, lockfiles, and generated editor data dependent
on which folders happened to be open.

**Recommendation.** Reserve all compiled canonical names, not only family
names. Give third-party environments namespaced stable IDs plus display names.
Treat user/workspace changes to targets and ambient packages as
`EnvironmentOverlay`s whose provenance and content hash are part of the
resolved identity; do not redefine the base object. Reject alias cycles and
same-precedence collisions. Define a separate, explicit precedence for file
detection, with an ambiguity result rather than lexicographic first-wins.

### H2. “Known anywhere” must distinguish discoverable, installable, and active

The proposed W002 source is every command in every discoverable provider,
including workspace and user packs. That makes a pack merely present on disk
change typo diagnostics in unrelated environments. It also conflates “this
package could be installed”, “the package is indexed”, “the package was
required”, and “this binding exists here”.

**Recommendation.** Retain four sets with provenance: globally documented,
installable/indexed for this project, expected from the selected environment,
and must/may active in this realm. W002 should name which level supplied the
candidate. Security and compilation queries use only realm bindings; completion
can opt into the broader sets with annotations.

### H3. The prior Jim probe needs a configuration matrix and lossless results

The Jim branch's release sweep is good evidence for default builds, but it
builds one configuration per release and maps heterogeneous failures to `ERR`.
That loses error class, output, exit status, and sometimes the difference between
parse rejection and missing command.

**Recommendation.** Preserve the release sweep, but key oracle rows by
`(release, configure flags, platform, commit)`. Record stdout, stderr, exit code,
and a structured observation category separately. Add default, minimal,
UTF-8-disabled, math-disabled, and no-default-extension builds where supported.
The checked-in derived matrix should name the source revision and configure
command for every column.

### H4. picol is a useful negative control, not automatically a supported family

picol's interpreter owns a mutable linked list of commands per interpreter
([`picol.c` lines 306–379](https://github.com/antirez/picol/blob/5f902e9b211fff21fb6e6abfd116df665e4091d5/picol.c#L306-L379)).
Registering a same-name command replaces the old function/client data, `proc`
registers new commands dynamically, and the standalone interpreter starts with
only a very small built-in set
([`picol.c` lines 749–785](https://github.com/antirez/picol/blob/5f902e9b211fff21fb6e6abfd116df665e4091d5/picol.c#L749-L785)).
It can also be embedded with the host registering an arbitrary surface.

This review does not claim #1631 must support picol. It is a useful negative
control: any supposedly general Tcl-family model should either represent
embedder/build capabilities and dynamic bindings, or reject the profile
explicitly. It must not invent a package catalogue and call that the runtime
surface.

### H5. Whole-file package activation is consumer-dependent

Q8 recommends retaining whole-file activation. That is defensible for broad
completion: after seeing `package require Foo`, offering Foo commands earlier in
the file is convenient. It is not defensible as a shared semantic fact. A call
before the require can dispatch elsewhere or fail; a require inside a
conditional or child interpreter may never affect the call.

**Recommendation.** Keep the approximation only in an assistance view. The
semantic view is position-, path-, and realm-sensitive, with widening for
unknown control flow. Ensure the two APIs have different names/types so a
compiler or taint pass cannot accidentally call the assistance shortcut.

## Required contracts before P0 can be ratified

The following is one viable corrected shape. The exact Rust spelling can change;
the separations and invariants cannot.

### 1. Core profile and build capabilities

```rust
pub struct CoreProfileId {
    pub family: FamilyId,
    pub release: ReleaseId,
    pub build: BuildProfileId,
}

pub struct CoreProfile {
    pub grammar: GrammarId,
    pub expr: ExprProfileId,
    pub character_model: CharacterModelId,
    pub capabilities: CapabilitySet,
}
```

`BuildProfileId` is not a free-form bag that consumers inspect by string. Its
typed capability values are resolved centrally and included in fingerprints.
Families which are build-invariant use one canonical build profile. Unknown
build facts stay unknown.

### 2. Version axes and sets

```rust
pub struct VersionAxisId(/* interned typed axis */);

pub struct VersionSet {
    pub axis: VersionAxisId,
    pub ranges: Arc<[HalfOpenRange]>, // normalised, disjoint
}

pub struct ItemHistory {
    pub introduced: Option<Version>,
    pub deprecated: Option<Version>,
    pub retired: Option<Version>,
}
```

`VersionSet` answers requirement/target set algebra. `ItemHistory` answers when
one declaration was introduced, deprecated, and retired. A declaration can have
several applicability sets for parallel trains without pretending its history
is one interval. Operations reject mismatched axes.

### 3. Surface declarations versus live bindings

```rust
pub struct SurfaceDeclaration {
    pub provider: ProviderId,
    pub applicable: VersionSet,
    pub predicate: CapabilityPredicate,
    pub invocation: InvocationSpecId,
    pub provenance: Provenance,
}

pub enum BindingKnowledge {
    Absent,
    Must(InvocationSpecId),
    May(Arc<[InvocationSpecId]>),
    Unknown,
}
```

Provider/catalogue resolution produces candidates. Tcl name resolution plus the
abstract transition state produces `BindingKnowledge`. Consumers are not given
an API that silently converts `May` or `Unknown` into one preferred spec.

Semantic properties common to free commands, ensemble arms, object methods, and
deeper dispatch belong on a shared `InvocationSpec` capability model. This is
the structural fix for G7/G15 rather than another list of fields copied into
`SubCommand`.

### 4. Stable environment definition plus explicit overlay

```rust
pub struct EnvironmentDefinition {
    pub id: EnvironmentId,       // canonical, reserved/namespaced
    pub core: CoreProfileSelector,
    pub expected_packages: Arc<[PackagePlacement]>,
    pub policy_defaults: EnvironmentPolicy,
    pub server_detection: DetectionFacts,
    pub editor_identity: Option<EditorLanguageIdentityId>,
    pub provenance: Provenance,
}

pub struct EnvironmentOverlay {
    pub base: EnvironmentId,
    pub target_changes: TargetChanges,
    pub package_changes: PackageChanges,
    pub origin: ConfigurationOrigin,
}
```

An overlay does not mutate the canonical definition. Its resolved content hash,
origin, and trust are part of cache identity and diagnostics. Only fixed,
contributed `EditorLanguageIdentityId` values may be advertised to an editor;
dynamic server environments select among them.

### 5. Realm-sensitive abstract state

```rust
pub struct AnalysisWorld {
    pub realms: RealmMap<InterpreterId, RealmState>,
}

pub struct RealmState {
    pub packages: PackageStateMap,
    pub command_bindings: CommandBindingMap,
    pub hidden_commands: HiddenCommandMap,
    pub namespace_state: NamespaceState,
    pub policy: InterpreterPolicy,
}
```

Transitions from `package`, `source`, `proc`, `rename`, `namespace import`,
`interp alias`, `interp hide/expose`, and child-interpreter operations update the
appropriate realm. Dynamic operands widen only the affected domain. A source
file can begin with an environment prior, but it cannot have one timeless live
command table.

### 6. Provenance, trust, and generation ownership

Every environment, surface, semantic fact, and hook carries provenance. Merging
is capability-specific: ordinary prose can use authoring precedence; security
facts use a monotone trust-aware join; runtime bindings use Tcl transition
semantics. Dynamic objects live in a registry generation that can be reclaimed.

### Non-negotiable invariants

| ID | Invariant | Gate |
|---|---|---|
| I1 | Equal core-profile IDs imply equal measured syntax/core semantics | cross-build and cross-release oracle matrix |
| I2 | Values from different version axes cannot be compared | type/compile-fail tests plus property tests |
| I3 | Package and binding facts are scoped to an interpreter realm and program point | parent/child/safe/ordering e2e suite |
| I4 | No taint/effect/lowering/codegen hook is selected before binding proof | ambiguity and dynamic-loader tests |
| I5 | Ambiguity widens effects or abstains; it never picks by catalogue order | load/import/rename permutation suite |
| I6 | Untrusted data cannot weaken trusted security facts | workspace-trust downgrade suite |
| I7 | Dropped registry generations release dynamic specs | 1,000-reload allocator test |
| I8 | Every advertised editor identity is actually contributed by that editor package | installed-extension manifest gate |
| I9 | Unknown semantic vocabulary fails closed | old-loader/new-pack downgrade fixtures |
| I10 | Pack migration preserves user-observable behaviour, not only serialised bytes | LSP/compiler/taint behavioural parity suite |

## Consequences for the proposal and migration plan

| Proposal area | Keep | Change before implementation |
|---|---|---|
| §2 classification | grammar families separate from packages | remove enforcement-location criterion; distinguish family, release, sublanguage, and policy |
| §3.1 dialect | family/release ladders and central grammar axes | add build/capability profile; define unknown-build behaviour |
| §3.2 package | provider-relative, multi-train surface declarations | declarations are candidates, not live bindings; use typed `VersionSet` |
| §3.3 environment | named compositions and central aliases | split fixed editor identity, stable definition, and workspace overlay; define collisions/trust |
| §4 availability | positive provider declarations and provenance | replace one `Lifecycle` window/context interval; do not use specificity for Tcl binding |
| §5 resolution | one ingress resolver for configuration names | add realm- and position-sensitive state after ingress; separate assistance from semantics |
| §5.4 ranges | explicit multi-target compatibility | reference evaluation across all distinct profiles; optimise only against that oracle; require primary |
| §6 SpecTcl | central declarative package source | fail closed on unknown semantics, close structural census gaps, and add generation ownership |
| P3/P5 equality | retain serialisation drift gate | add upstream-grounded behavioural parity and dynamic-surface honesty |

A safer phase order is:

1. **P0 — contracts and oracles.** Ratify the separations above, the version-set
   algebra, trust policy, binding-proof rule, editor-identity boundary, and
   immutable upstream oracle ledger. Resolve name/alias collision rules.
2. **P1 — core/environment model only.** Land families, releases, build
   profiles, stable environment definitions/overlays, and central ingress while
   leaving existing native package specs in place.
3. **P1a — realm state.** Integrate the existing state-transition model with
   provider candidates, package transitions, safe interpreters, and the one name
   resolver. Separate assistance and semantic queries.
4. **P1b — range reference implementation.** Land typed `VersionSet` and the
   correct per-distinct-profile evaluator first; only then add detector/parse
   optimisations.
5. **P2 — durable SpecTcl foundation.** Reclaimable generations, trust-aware
   provenance, semantic forward-compatibility policy, shared invocation
   capabilities, and closure or explicit abstention for the external census.
6. **P3 — Tk pilot.** Preserve bytes and behaviour; validate Tcl/Tk version
   independence and lowercase/uppercase loader semantics.
7. **P4 — smaller packages.** Move iApps/tmsh/Expect/EDA incrementally with the
   same behaviour and trust gates.
8. **P5 — tcllib by adversarial module.** Start with `struct::tree`,
   `struct::graph`, `fileutil::traverse`, and `oo::dialect`; scale only after
   those dynamic shapes are honest.
9. **P6 — Jim.** Rebase the prior measurements into release × build profiles;
   do not port one default-build column as the whole family truth.

This order preserves the proposal's migration goal while preventing the pack
move from cementing the wrong lifetime, trust, range, and runtime-binding APIs.

## What is sound and should be retained

The adversarial findings do not invalidate the central direction. The following
parts are strong:

- remove Tk/tcllib/Expect/EDA package surfaces from the dialect bitmask;
- keep actual grammar family axes central and typed;
- use positive allow-lists for closed-world iRules instead of subtraction;
- centralise canonical environment names and legacy spellings;
- attach availability to the provider's own version axis;
- make command knowledge declarative and shared by compiler, analyser, LSP,
  Studio, Explorer, and editor generators;
- add loader-direction coverage so documented fields cannot lack parser support;
  and
- migrate incrementally with reproducible upstream snapshots and drift gates.

The corrected model adds the missing boundary: declarative registry data tells
the tools what a provider *can* mean; interpreter-state analysis tells them what
a command *does* mean at this call site.

## Appendix A — reproducible experiments

Experiments were run on 2026-08-26 on macOS 26.6.2 arm64 with Apple clang
21.0.0. The Tcl oracle was Homebrew Tcl 9.0.4. All source repositories were
detached at the immutable revisions in the evidence table.

### A1. Same Jim release, two build profiles

Two separate worktrees at
`d5243a25c488dfe751ef218828f13516e04ea2ba` were built:

```sh
# Worktree 1
./configure
make -j8

# Worktree 2
./configure --minimal
make -j8
```

Run this against each resulting `jimsh`:

```sh
./jimsh -e '
set sqrt_code [catch {expr {sqrt(4)}} sqrt_result]
puts [list \
    patch [info patchlevel] \
    utf8len [string length é] \
    json [llength [info commands json::decode]] \
    tclprefix [llength [info commands tcl::prefix]] \
    zlib [llength [info commands zlib]] \
    sqrt $sqrt_code \
    sqrt_result $sqrt_result]
'
```

Observed:

```text
patch 0.84 utf8len 1 json 1 tclprefix 1 zlib 1 sqrt 0 sqrt_result 2.0
patch 0.84 utf8len 2 json 0 tclprefix 0 zlib 0 sqrt 1 \
    sqrt_result {syntax error in expression: "sqrt(4)"}
```

This experiment proves only the stated same-release configuration divergence.
It does not assert that every listed command is part of the Jim lexical core.

### A2. Tcl requirement sets and per-interpreter temporal state

Run with Tcl 9.0.4:

```tcl
puts "runtime [info patchlevel]"

foreach {actual requirements} {
    8.6.16 {8.5-9.0}
    9.0    {8.5-9.0}
    9.0.4  {8.5-9.1}
    9.0.4  {8.5-}
    9.0.4  {8.5}
    9.0.4  {8.5 9.0-9.1}
} {
    puts "vsatisfies $actual $requirements \
        [package vsatisfies $actual {*}$requirements]"
}

set loads 0
package ifneeded Demo 1.0 {
    incr ::loads
    proc demo {} {return loaded}
    package provide Demo 1.0
}

puts "before_require provide {[package provide Demo]} \
    command [llength [info commands demo]] loads $loads"
puts "require_result [package require Demo 1.0]"
puts "after_require provide [package provide Demo] \
    command [llength [info commands demo]] loads $loads"

interp create child
puts "child_initial_demo [child eval {llength [info commands demo]}] \
    child_initial_package {[child eval {package provide Demo}]}"
child eval {
    proc demo {} {return child}
    package provide Demo 2.0
}
puts "parent_demo [demo] parent_package [package provide Demo] \
    child_demo [child eval demo] \
    child_package [child eval {package provide Demo}]"

interp create -safe safe
puts "parent_open [llength [info commands open]] \
    safe_open [safe eval {llength [info commands open]}] \
    safe_hidden [expr {[lsearch -exact [interp hidden safe] open] >= 0}] \
    parent_tcl [package provide Tcl] \
    safe_tcl [safe eval {package provide Tcl}]"

rename demo {}
puts "after_rename_command [llength [info commands demo]] \
    package_still_loaded [package provide Demo]"
```

Observed:

```text
runtime 9.0.4
vsatisfies 8.6.16 8.5-9.0 1
vsatisfies 9.0 8.5-9.0 0
vsatisfies 9.0.4 8.5-9.1 1
vsatisfies 9.0.4 8.5- 1
vsatisfies 9.0.4 8.5 0
vsatisfies 9.0.4 8.5 9.0-9.1 1
before_require provide {} command 0 loads 0
require_result 1.0
after_require provide 1.0 command 1 loads 1
child_initial_demo 0 child_initial_package {}
parent_demo loaded parent_package 1.0 child_demo child child_package 2.0
parent_open 1 safe_open 0 safe_hidden 1 parent_tcl 9.0.4 safe_tcl 9.0.4
after_rename_command 0 package_still_loaded 1.0
```

### A3. Package/import order changes the command binding

```tcl
proc load_order {order} {
    interp create i
    i eval {
        namespace eval A {
            namespace export clash
            proc clash {} {return A}
        }
        namespace eval B {
            namespace export clash
            proc clash {} {return B}
        }
        package ifneeded A 1.0 {
            namespace import -force ::A::clash
            package provide A 1.0
        }
        package ifneeded B 1.0 {
            namespace import -force ::B::clash
            package provide B 1.0
        }
    }
    foreach package $order {
        i eval [list package require $package]
    }
    set result [i eval clash]
    interp delete i
    return $result
}

puts "package_order_AB [load_order {A B}] \
    package_order_BA [load_order {B A}]"

namespace eval A {
    namespace export clash
    proc clash {} {return A}
}
namespace eval B {
    namespace export clash
    proc clash {} {return B}
}
namespace import ::A::clash
set conflict [catch {namespace import ::B::clash} message]
namespace import -force ::B::clash
puts "first [A::clash] conflict $conflict message {$message} \
    forced [clash] origin [namespace origin clash]"
```

Observed:

```text
package_order_AB B package_order_BA A
first A conflict 1 message {can't import command "clash": already exists} \
    forced B origin ::B::clash
```

## Appendix B — primary-source traceability

The most important source-to-conclusion paths are summarised here so a future
review can re-run the audit without trusting this document's prose.

| Conclusion | Upstream implementation evidence | Local implementation evidence |
|---|---|---|
| build profile is semantic | Jim `auto.def`, `utf8.h` | prior Jim model/probe is one default-build column |
| packages are realm-local and executable | Tcl `tclPkg.c` package table, require, ifneeded, unknown | proposed document-global context/floors |
| bindings are mutable and order-sensitive | Tcl `tclBasic.c`, import/rename experiment | existing `state_transition.rs` already has binding/topology/policy domains |
| package targets are version sets | Tcl `package` manual and `vsatisfies` experiment | `Lifecycle` explicitly has a narrower contract |
| packages can define sublanguages/dynamic dispatch | tcllib `oodialect`, tree, graph, traverse | external census G1–G16; CommandSpec/SubCommand asymmetries |
| Tk names/versions have loader semantics | Tk `Makefile.in`, `tkWindow.c`, `configure.ac` | proposed static alias/`tracks-base` vocabulary |
| dynamic editor IDs are not portable | VS Code and Zed contribution documentation | proposal allows pack/workspace `language_id` |
| live reload leaks dynamic specs | — | loader `Box::leak` paths and registry size comment |
| workspace override is trust-sensitive | VS Code Workspace Trust contract | discovery, nearest-tier pack merge, `-override` |

## Limitations and explicit non-findings

- Picol was used only as a negative control; this review does not add it to the
  supported roadmap.
- The range finding identifies a missing correctness proof. It does not claim a
  currently observed Tcl 8.4–9.1 A→B→A endpoint bug.
- The package-state experiments use pure Tcl loader scripts. Native extensions
  make the possible state space larger, not smaller, but were not needed to
  falsify the document-global model.
- This review did not benchmark the corrected multi-profile range reference
  implementation. The release ladders are small; optimise only after measuring.
- Surface declarations remain valuable even when runtime state is unknown. The
  criticism is of treating them as proved live bindings, not of cataloguing
  package APIs.
