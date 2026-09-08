# Dialects, packages, and environments — the registry model (issue #1631)

> The model as built. §11 is the single ledger of what is still open in
> this programme; nothing outside it should be read as outstanding work.
> Where the prose still describes intent rather than code it says so.
>
> Companions:
> [dialect-and-package-registry-centralisation.md](dialect-and-package-registry-centralisation.md)
> (the registration/resolution contract, the retirement ledger's open rows,
> and the `tcl spec upgrade` specification),
> [spec-packs.md](spec-packs.md) (the SpecTcl format),
> [dialect-profile-model.md](dialect-profile-model.md) (the interned
> catalogue the lexer is still keyed on),
> [eda-library-packages.md](eda-library-packages.md),
> [contracts/dialect-detection.md](contracts/dialect-detection.md),
> [contracts/package-loading.md](contracts/package-loading.md),
> [bigip-irule-parser-measurements.md](bigip-irule-parser-measurements.md)
> (the live F5 evidence).

## 0. The ruling, and the model in four layers

Issue #1631 rules that a catalogue must not conflate two kinds of thing:
**dialects** — core-language variants that change how the lexer, parser,
and analyser behave — and **loadable packages** — plain Tcl plus a command
surface. The model keeps four layers apart:

1. A **dialect / core profile** is the language core: a *family* at a
   *release* under a *build/capability profile* (`tcl` at 8.4–9.1,
   `f5-tcl`, `f5-irules`, `jim` at 0.76–0.84 × its configure matrix). Core
   profiles live in the compiled catalogue and own every
   lexer/expr/numeral/escape/character-model axis; a pack may declare a
   further family with a `dialect` block (§6.2). The build axis is not
   optional: the same Jim 0.84 commit built default vs `--minimal` has a
   different character model, expr-function acceptance, and command
   surface, and unknown builds resolve to `Unknown`, never to a silently
   assumed default.
2. A **package** is a provider of versioned **surface declarations** —
   commands a provider *may* install, with version sets, predicates, and
   provenance (`Tk`, every tcllib module, the iApps/tmsh surfaces, Expect,
   the EDA vendor libraries). A surface declaration is catalogue evidence;
   it is never, by itself, proof that a command is bound in a given
   interpreter.
3. An **environment** is a named, selectable *definition* of what a
   project works against: a core-profile selector plus per-axis
   **version-set targets**, expected/ambient packages at platform-implied
   versions, server-side detection facts, policy defaults (closed-world,
   fixed ensembles, version ceiling), and a reference to a *fixed,
   contributed* editor language identity. `tcl8.6`, `f5-irules`,
   `f5-iapps`, `xilinx-eda-tcl`, and `tk` are all environment names.
   Environments are **dynamic data** — compiled-in for the core set,
   declarable by packs, and adjusted per workspace/user configuration
   through explicit **overlays** that never mutate the canonical
   definition — and they carry the alias table that keeps retired
   quasi-dialect names resolving.
4. An **analysis world** is the per-document semantic state the compiler,
   taint, side-effect, and codegen passes actually query: interpreter
   **realms**, each with temporal package state and command-binding
   knowledge (`Absent`/`Must`/`May`/`Unknown`), fed by the environment as
   a prior and by the transition vocabulary in
   `rust/tcl-registry/src/state_transition.rs`. Catalogue data says what a
   provider *can* mean; realm state says what a name *does* mean at this
   call site.

The only backwards compatibility maintained is (a) data-level: every name
a user can write (configs, language ids, directives, CLI flags, pack
`-dialect` rows) resolves through the environment alias table, and (b)
format-level: every published `speclib` 1.x pack keeps loading, with
unknown *semantic* vocabulary in newer packs failing closed (§6.1). There
are **no Rust-side compatibility shims**: `cargo xtask retired-api-gate`
holds the retired dialect-name validators, registry doors and `DialectSet`
at zero references. What survives of the pre-#1631 catalogue is the
interned `DialectProfile` the *lexer* is keyed on — §11's D5.

## 0.05 Standing design principles

**P-A — SpecTcl declares; Rust executes.** SpecTcl declares command specs,
and those declarations are compiled to a structure Rust uses efficiently.
The tclvm is entered *only* to execute small hooks, and only where a hook
is genuinely needed. The snapshot is the artefact, evaluation is a build
step, and analysis reads the compiled structure.

**P-B — performance in the general case, and hooks are the exception.**
Scripts must not run on every edit: every relation the declarative
vocabulary can express is checked natively with no VM entry, hooks declare
their inputs so results are shape-cached, and an edit that does not change
a hook's inputs reuses the cached verdict. When a declarative vocabulary
and a hook can both express something, the declarative form wins.

**P-C — retire unused surface.** Declared-and-unpopulated model surface is
deleted rather than carried, because a word no data uses invites packs to
guess at semantics the engine never implements. Anything genuinely needed
later returns *with* its consumer.

**P-D — prove it with an experiment.** Where a question is empirical,
answer it by measuring: build the interpreter, run the probe, record the
transcript, and turn it into a hermetic vector. The F5 conformance corpus
and the five `jimsh` builds are the pattern — both found real model
defects on their first run.

**P-E — follow ordinary Tcl design and patterns.** Where a surface needs
expressive power, reach for the shapes a Tcl programmer already knows
(`if`, `switch`, `foreach`, ordinary command syntax) before inventing a
mini-language.

## 0.1 Review evidence

Two adversarial reviews shaped the model. Their findings are built in —
the build axis (§3.1), per-interpreter temporal package state (§4.2), the
`VersionSet` algebra (§4.1), the surface-declaration / realm-binding
split, the fixed editor-identity set (§3.3), the trust lattice (§6.4), the
fail-closed vocabulary classes (§6.1), the classification-by-fingerprint
rule (§2), and the invariants I1–I10 (§8). Three of their probes remain
the reference experiments (`P-D`):

| Probe | Result the model must reproduce |
|---|---|
| Jim 0.84 (`d5243a25`) built `./configure` vs `./configure --minimal` | `string length é` is 1 vs 2, `json::decode` / `tcl::prefix` / `zlib` present vs absent, `expr {sqrt(4)}` evaluates vs `syntax error in expression` — same family and release, different core |
| Tcl 9.0.4 `package require` / `interp create` / `rename` | `package provide Demo 1.0` survives `rename demo {}`; a child interpreter inherits no package state; a safe child provides the same `Tcl` version while hiding `open`; `vsatisfies 9.0 8.5-9.0` is 0 (exclusive maximum) and `vsatisfies 9.0.4 8.5 9.0-9.1` is 1 (union) |
| Two packages exporting `clash`, loaded in either order | the binding follows load order; an ordinary `namespace import` rejects the collision, `-force` replaces it — no provider specificity can recover the runtime fact |

Upstream revisions the model was checked against: Tcl `core-9-0-4`
(`c655b477`), Tk `core-9-0-4` (`584f8fcf`), JimTcl `0.84` (`d5243a25`),
picol (`5f902e9b`, a negative control), tcllib `tcllib-2-0` (`2a63bf21`),
ticklecharts `v3.2.8`, pave (`875de1f1`), SpiceGenTcl (`e8aa45ce`).

## 0.2 F5 evidence

Every F5 grammar, command, variable, package, policy and evidence record is
keyed by `BigIpExecutionContext` (`rust/tcl-registry/src/f5/execution_context.rs`):
`TmmIRule`, `TmshCliScript`, `IAppImplementation`, `IAppPresentationApl`,
`IAppPresentationTclCallback`, `HostShellTcl`. Each variant carries its
family, build profile, environment name and core profile; the two APL
contexts answer `None` on every one of them, `is_tcl()` is `false` for
APL, and `promotes_facts_to` refuses every cross-context substitution.
`f5-bigip` is not a catch-all Tcl identity: BIG-IP configuration, APL,
APL's Tcl callbacks, iRules, tmsh scripts, iApp implementation Tcl, and
host Tcl are independently routed surfaces.

The measured facts ([measurements](bigip-irule-parser-measurements.md)):

- **The fork.** iRules, tmsh `cli script` and iApp implementation Tcl are
  **one parser**, a fork of Tcl 8.4.6 (`info patchlevel` 8.4.6 in all
  three; every 8.5 discriminator fails). The host `/usr/bin/tclsh8.4` is
  8.4.13 and unrelated. The tree is therefore `tcl@8.4.6` → family
  `f5-tcl` (the trunk, ladder keyed by TMOS release) → family `f5-irules`
  (an offshoot carrying its own parse-level fingerprint), with `f5-iapps`
  and `f5-tmsh` as environments riding the trunk directly.
  `rust/tcl-registry/src/f5/evidence.rs` carries the `(context, build,
  fact, provenance)` records from the 21.1.0.1 run: patchlevel per context
  (`tcl_patchLevel` **unset** in `TmshCliScript`), the three-way
  `tcl_platform` split (fabricated / empty / real Linux with `wordSize 4`
  — a 32-bit build, `BuildProfileId::F5Scriptd32`), the 16 discriminators,
  the 31 command-class rows, and the traffic lab's priority policy. The
  semantic door (`measured_fact`) is exact-build-only; the assistance door
  (`assistance_fact`) returns a labelled `NearestKnownAssistance` carrying
  the build it was really measured on; neither crosses a context boundary.
- **`}{`.** The separator is generic (`list {a}{b}` and `set x {a}{b}` both
  split), lexical (lexer data, no per-command grammar, no BIG-IP
  lifecycle), and gated on the word having *started* with `{` or `"`
  (`if{1}{…}` stays one bare word). `{*}` does not exist in the dialect —
  `list {*}{a b}` yields `* {a b}`. The same run found the independent
  brace-line continuation rule (§1).
- **tmsh syntax is its own axis.** `VersionAxisId::tmsh_syntax()` (and
  `VersionAxisId::big_ip()`) are typed axes in `tcl-dialect`, so every
  binary operation against a Tcl core or package axis is `AxisMismatch`.
  `rust/tcl-registry/src/f5/tmsh_syntax.rs` resolves the
  `tmsh::modify cli version active` form through the ordinary
  `InvocationArguments` view (literal → `Selected`, non-literal →
  `Unknown`) and carries `scope_is_measured: false` (§11 V6).
- **iApp action metadata.** `rust/tcl-registry/src/f5/iapp_metadata.rs`
  parses `requires-bigip-version-min/max`, `role-acl` and `run-as` into
  `IAppActionOverlay` — the declared interval on the BIG-IP axis,
  `effective_targets` intersecting it with the configured targets,
  `role-acl` as `None`/empty/roles, and an omitted `run-as` reading as the
  **calling user**, which widens authorisation analysis rather than
  inheriting an administrator surface.
- **The conformance corpus.** `rust/tcl-registry/src/f5/corpus.rs` holds
  205 hermetic vectors derived from the checked-in transcripts — 21 §4a
  parity cases, 9 §4a environment rows, 16 discriminators, 31 §4b command
  classes, 120 event-context cells, 8 priority facts — each citing its
  measurements section and asserted against the model. Rows carry a
  two-sided `ModelExpectation`: `Agrees` must keep agreeing, `Diverges`
  must keep diverging *exactly where recorded* (closing a gap is a
  deliberate edit), `NotComparable` names why there is no model answer.
  Six deliberate divergences remain: the `RULE_INIT` compile-acceptance
  rows, which must not be read as "valid to use".

**The probe contract (E4).** An appliance run that is to count as evidence
records `show sys version` and Tcl provenance without changing
configuration; labels host `tclsh` introspection **host only**; creates
each probe object (`cli script`, unattached iRule, minimal iApp template
and service) under a unique `__tcl_lsp_probe_*` name after an exact-name
absence check, runs it, deletes it, and proves it absent; exercises the
APL contexts only if a non-interactive presentation renderer exists and
records them `Unknown` otherwise — never copying the implementation result
into them; attaches nothing to a virtual server; and never runs `save sys
config`. A shell `EXIT` trap deletes only the exact probe names; a
collision aborts. The driver is
`scripts/dev/bigip-probes/lib/e4-context-probe.sh`; the measurements
document's §11 says which of its sections meet the contract (§3 and §4a
do; §5–§9 are consumed as `e4_conforming: false` — §11 V4).

**Coverage the corpus still wants** (§11 V1–V3):

| Context | Lab build (21.1.0.1) | One supported 17.x build | One older build |
|---|---|---|---|
| TMM iRules | measured | wanted | wanted |
| tmsh CLI script, Administrator | measured | wanted | wanted |
| tmsh CLI script, restricted role | wanted | desirable | desirable |
| iApp implementation | measured | wanted | desirable |
| iApp presentation APL / Tcl callback | `Unknown` | `Unknown` | `Unknown` |
| host `tclsh` | provenance only | provenance only | provenance only |

For every measured row the registry must predict the observed grammar and
surface without a command-name branch in a consumer; for every unmeasured
row, strong semantic hooks abstain.

## 0.3 Three gates on the shipped tree

**Is the model centralised?** `profile_queries::package_available` and
`ResolvedContext::is_closed_world_package` read one predicate
(`model::surface::is_closed_world_package`), and `retired-api-gate` owns
both closed-world spellings to `tcl-registry`'s surface model, so a third
set cannot quietly appear. The parity sweeps
(`per_spec_visibility_matches_the_old_model_for_every_profile`,
`spec_queries_reproduce_profile_queries_for_every_profile`) pin the
unification.

**Is a `SpecTcl` pack really a Tcl script?** A real `tclsh9.0` is the
arbiter (`rust/tcl-spectcl/tests/pack_is_real_tcl.rs`): every shipped pack
is sourced by a stock interpreter that knows nothing about `SpecTcl`, with
the vocabulary absorbed by `unknown` and the three words the loader really
does evaluate as a script — `speclib`, `command`, `subcommand` — recursing
into their brace group. Only parse failures count (`::errorCode` `NONE`).
The gate is mutation-tested: an unbalanced `[` inside one `command` body
fails it.

**Does the VM execute them?** `rust/tcl-spectcl/tests/eval_loader.rs`
loads every bundled pack twice — once captured from its parse tree, once
executed statement by statement by `tcl-vm` — and requires byte-identical
`command_entry_json` per declared command; the same test does it for the
`tcl spec upgrade` rewrite. The interpreter route is slow, not absent
([spec-packs.md](spec-packs.md) § *Measured*), which is what makes the
static fast path an optimisation worth gating rather than a second reading
of the file.

## 1. The grammar axis

Exactly five `LexerGrammar` constants exist (`rust/tcl-dialect/src/profile.rs`):
four plain-Tcl releases and the `f5-tcl` trunk. Every axis with a genuine
per-variant delta is centralised in `rust/tcl-dialect/src/grammar.rs`:

| Axis | Values |
|---|---|
| `{*}` expansion (TIP 157) | off in 8.4 and on the `f5-tcl` trunk; on 8.5+. On TMM the separator wins — `{*}$l` lexes as a literal `*` word plus the unexpanded list, silently; implementing expansion there would disagree with the appliance |
| implicit word break (`}{`) | the `f5-tcl` trunk (all three F5 catalogue rows select it): fires only when the word *started* with `{` or `"` (bare words, `$v{b}`, `${v}b`, `[cmd]{b}` are untouched), applies repeatedly and in every word position including the command name (`{set}zz 7` runs `set zz 7`), restarts word parsing from scratch (`}else{log` becomes the bare word `else{log`), emits no diagnostic, and leaves the `expr` sub-parser unmodified. The zero-width `Sep` token `Lexer::parse_brace` / `parse_quoted` emit (`rust/tcl-lexer/src/lexer.rs`) matches the appliance |
| brace-line continuation (N-rules) | the `f5-tcl` trunk: a newline does not terminate a command when the next line's first non-whitespace character is `{` (K&R brace style is legal). Unconditional (`list a b` ⏎ `{c}` → 3 elements), any nesting depth; a blank, whitespace-only, or comment line terminates normally; `else`/`elseif` are a separate one-newline lookahead by `if` that does not cross a blank line. The `BraceLineContinuation` axis, `Continues` on `GRAMMAR_F5_TCL` |
| `${…}` close rule | `FirstClose` (8.x, Jim) vs `Tcl9Nesting` (9.x) |
| leading-BOM skip on `source` | 9.x only |
| `#` comments in `[expr]` (TIP 582) | 9.x, Jim ≥0.81 |
| numeral grammar | `Tcl84` / `Tcl85` (`0b`/`0o`) / `Tcl90` (`0d`, `_` separators, leading-zero decimal) / `Jim` / `Jim080` |
| escape grammar (TIP 388, `TCL_UTF_MAX`) | `Tcl84` (=8.5) / `Tcl86` / `Tcl90` / `Jim` |
| expr word-operator lexemes | `eq`/`ne` ≥8.4, `in`/`ni` ≥8.5, `lt`/`le`/`gt`/`ge` ≥9.0, plus **ten** F5 word operators on the `f5-tcl` trunk (`contains`, `starts_with`, …, and the bare `matches`) — trunk facts, byte-identical in tmsh and iApp. `matches`' precedence and semantics are inferred, not measured (§11 V5): it takes its siblings' equality class, answers as string equality in the VM, and is excluded from constant folding |
| the five Jim lexical axes | `WordSeparators`, `BraceBackslashNewline`, `QuoteTermination`, `VarSyntax` (`$(expr)` as its own token kind, index-paren nesting), `ListParse` — [dialect-profile-model.md](dialect-profile-model.md) §2.3 |

`LexerGrammar` is an **extensible per-(family, release) record**: a new
axis is a field plus its family values, never a new profile.

Real Tcl 9 changes that are *not* grammar — tilde-expansion removal,
`fconfigure -profile` (TIP 656), the TIP 745 mathfuncs — live entirely in
registry command data. iRules additionally owns a parse-level *structural*
grammar (the declaration-only top level: `IrulesExecutionContext`,
IRULE5006/5007, `Traits::IRULES_TOP_LEVEL_ONLY`), and its K36322151 bans
make command-head identity statically decidable
(`rust/tcl-irules/src/when_block.rs`) — parse-level facts that keep
`f5-irules` a **dialect offshoot** of the shared trunk. `expect`,
`spectcl`, `sslictcl`, `bpf`, `tk`, and the six EDA entries use a core
release's grammar **verbatim**; `f5-bigip` is not Tcl at all (its own
tokeniser and tree-sitter grammar; the profile's grammar field is inert).

## 2. The classification rule

The criterion compares **observable semantic fingerprints**, never which
compiler module happens to enforce a rule:

> A **language family** is justified by observable *outer lexical/syntactic
> or core evaluation* differences from every other family — a
> `LexerGrammar`/expr/numeral/escape/character-model axis value no other
> family's ladder provides. A **release** is a target point on an admitted
> family's ladder, whether or not it changes grammar versus its neighbour
> (tcl 9.1 is a release on the `tcl` ladder). A **command sublanguage** —
> a DSL living inside command arguments, like tcllib's `oo::dialect`
> definition bodies or pave's widget tuples — is registry descriptor data
> attached to an invocation, not a dialect. **Availability, safety, and
> closed-world restrictions are environment/realm policy**, not language
> identity: a safe interpreter hides `open` while still being Tcl 9.0.4.
> Everything else — any variant whose whole content is "commands, options,
> and versions" — is a package, and any user-selectable name for "a base
> plus packages" is an environment.

The rule is an invariant test in `rust/tcl-dialect`: every family in the
catalogue must own at least one axis value distinct from every other
family's ladder, and the environment type has no grammar field, so an
environment adding a grammar knob is unrepresentable. Two calibration
controls: a synthetic restriction moved between parser and analyser must
not change its classification, and picol — a mutable per-interpreter
command table over a tiny built-in set, whose second revision changes core
evaluation semantics under the same project name — is the negative control
the model rejects explicitly rather than misdescribes.

| Name | Classification | Notes |
|---|---|---|
| `tcl8.4` … `tcl9.1` | dialect (family `tcl`, releases 8.4–9.1) | the ladder is exactly these per-release identities. A per-major family split (`tcl8`/`tcl9`) was rejected — succession is not a fork, and the single total order is load-bearing for `ItemHistory` windows spanning the 8/9 boundary, for the `vsatisfies`-oracle-pinned `VersionSet` algebra (`tcl 8.5-` matches 9.0, as in Tcl itself), and for cross-major range targets. `tcl8`/`tcl9` named-range sugar was also declined |
| `f5-tcl` | dialect (family `f5-tcl`, fork of Tcl 8.4.6, ladder keyed by TMOS release) | the shared BIG-IP trunk, measured identical in all three F5 execution contexts (§0.2): the R-rules, the N-rules, the inert `{*}`, 8.4 numerals, full ordinary `proc` semantics, and the `expr` word operators |
| `f5-irules` | dialect (family `f5-irules`, offshoot of `f5-tcl`) + environment `f5-irules` | inherits the trunk grammar whole. What it adds is **load-time language rules**, measured live: the declaration-only top level — only `when`, `proc`, `priority`, `timing` at the root of a rule; top-level `proc` reachable only via `call` — closed-world command resolution at rule load (unaffected by `catch`), the event model with compile-enforced event-context validity, and `expr` math-function validation at load. Those rules are **lexical scans of braced script literals**: `eval {proc …}` and `uplevel #0 {proc …}` are rejected identically to a bare `proc`, but script text held in a variable escapes the scan, so the analyser recurses the load-time checks through braced literals under `eval`/`uplevel` and widens to `May` bindings on variable-held scripts. A runtime-defined proc is a persistent per-TMM global; `when` is not a runtime command in any scope. Unbraced `if $var` — the primitive of the only user-space JIT the dialect permits (the `static::`-cached compiled-expression idiom) — surfaces as a **warning, never an error**. The environment carries what is policy: the 31-command disabled list, split into **16 absent from TMM's interpreter** (`exec`, `open`, `socket`, `source`, `file`, `glob`, `cd`, `exit`, `load`, `pwd`, `fconfigure`, `unknown`, the four `auto_*`) and **15 present but refused by the rule compiler** (`namespace`, `time`, `rename`, `interp`, `package`, `gets`, `eof`, `seek`, `tell`, `flush`, `fblocked`, `fcopy`, `pid`, `update`, `vwait`), the latter reachable via `eval` at runtime — plus 8.3-era-only `trace`, static-head-identity, and fabricated `tcl_platform` (`os BIG-IP`, `wordSize 8`) |
| `jim` | dialect (family `jim`, releases 0.76–0.84) + **one** environment `jim` (aliases `jimsh`, `jimtcl`) | measured grammar deltas per release (`NumberSyntax::Jim`/`Jim080`, `EscapeSyntax::Jim`, expr comments ≥0.81, the five lexical axes, the expr precedence/operator/mathfunc/arity divergences — §3.1); a `Lineage::Reimplementation` ancestry edge to Tcl 8.6 carries the core command surface, narrowed by the compiled roster `rust/tcl-spectcl/core-surfaces/jim.tclspec` |
| `f5-iapps` | environment = dialect `f5-tcl` (32-bit build profile) + iapps pack (ambient, BIG-IP-keyed) + policy (fixed ensembles, W108 strict ASCII, `ambient-plus-require`) | environment deltas are real but non-grammatical: `exec` works here and not in TMM, `package names` is large, `tcl_platform` is real-Linux with `wordSize 4`. APL container routing is a language-id fact; the two APL contexts are `Unknown` |
| `f5-tmsh` | environment = dialect `f5-tcl` + tmsh pack (ambient, BIG-IP-keyed) + the tmsh syntax-version axis | a tmsh lexing mode **is** required — a `cli script` reproduces the entire trunk grammar. Environment deltas: `exec` works, `tcl_platform` is **empty**, `tcl_patchLevel` does not exist, a non-standard `info vartype` subcommand exists |
| `tk` | package `Tk` + environment `tk` (alias `wish`) = tcl@8.6 + Tk ambient on Tk's own axis | one package answers both `tk` (ambient) and plain Tcl (hosted) — §3.2 |
| `expect` | environment = tcl@8.6 + expect pack (ambient) | `expect`'s clause grammar is registry `CaseListSpec` descriptor data shared with `switch` |
| the six EDA shells | pack-declared environments (`specs/eda_*.tclspec`) | identity, extensions, keyed tool pins and `help_terms` are `environment` blocks; a generated seed (`rust/tcl-dialect/src/model/bundled_environments.rs`, `cargo xtask gen-bundled-environments`) lets them resolve before any pack is published |
| `spectcl`, `bpf` | environments over tcl@9.0, compiled as Rust catalogue profiles | grammar is `GRAMMAR_TCL9X` verbatim; each is a closed-world command surface (`rust/tcl-registry/src/commands/spectcl/`, `commands/bpf/`) |
| `sslictcl` | environment = tcl@9.0 + sslictcl pack (package surface) | `GRAMMAR_TCL9X` verbatim; the declaration vocabulary (`rust/tcl-registry/src/commands/sslictcl/`) exists inside a `.sslictcl` document and nowhere else, and evaluates nothing — [sslictcl-vocabulary.md](sslictcl-vocabulary.md) |
| `f5-bigip` | **neither** — a separate language surface | own tokeniser (`rust/tcl-bigip/src/conf_tokens.rs`), own tree-sitter grammar; keeps its identity/routing entry and stays off the Tcl availability axis (`f5-bigip` in an `available` row is an error) |

## 3. The three concepts

### 3.1 `Dialect` — family × release × build

```rust
// rust/tcl-dialect/src/model
pub enum Family { Tcl, F5Tcl, F5Irules, Jim }
// F5Tcl forks from Tcl@8.4.6; F5Irules forks from F5Tcl; Jim is a
// Lineage::Reimplementation anchored at Tcl 8.6. Grammar resolution walks
// the fork edges: an axis the offshoot does not override answers from the
// trunk, and the trunk from tcl@8.4.6.

pub struct Release(/* ordinal within the family's ladder */);
pub enum BuildProfileId { Canonical, JimFull, JimMinimal, F5Scriptd32, Unknown }

pub struct CoreProfileId { family: Family, release: Release, build: BuildProfileId }
pub struct CoreProfile {
    grammar: LexerGrammar,          // total over admitted ids
    expr: ExprGrammar,
    character_model: Option<StringCharacterModel>,
    capabilities: CapabilitySet,    // typed; resolved centrally
}
```

**The build axis is load-bearing.** `CapabilitySet::canonical` is keyed
by `Release`, not by `Family`: Jim's `auto.def` flipped its default at
0.82, so a bare `./configure` gives neither utf8 nor math through 0.81 and
both from 0.82 — `expr {sqrt(4)}` is a syntax error on a stock `jimsh
0.81` and answers `2.0` on a stock `0.82`. `--minimal` compiles out
`JIM_UTF8` (one byte = one char) and `JIM_MATH_FUNCTIONS` (nineteen of the
twenty-six mathfunc rows), but seven rows — `int`, `wide`, `abs`,
`double`, `round`, `rand`, `srand` — sit outside that `#ifdef`, so
`MathFunc::needs_math_extension` gates per function. Families that are
build-invariant declare one canonical build profile; an unknown build
resolves every unmeasured capability to `Unknown`. Tcl's own history has
the axis (`TCL_UTF_MAX` 3/4/6 builds); the `tcl` family carries one
canonical profile per release with the axis representable.

**The `ExprGrammar` contract.** The surface a family × release owns:

```rust
pub struct ExprGrammar {
    numbers: NumberSyntax,
    comments: ExprCommentStyle,
    word_operators: &'static [WordOperator],        // eq/ne, in/ni, lt/le/gt/ge, contains, …
    precedence: fn(&str) -> Option<(u8, u8)>,       // NOT derivable from the operator set
    symbolic_operators: &'static [(&'static str, Release)], // Jim's <<< >>> (all), =* =~ (0.84)
    mathfuncs: &'static [MathFunc],                 // a SET, not a floor
    arity: ExprArity,                               // Concatenating | ExactlyOne
    substitution: ExprSubstitution,                 // picol 2 proves the axis varies
}
```

Every Jim value is read from `jim.c`, `auto.def`, `utf8.h` and
`jim_tcl.txt` at the upstream tags 0.76 … 0.84 and checked against five
built `jimsh` binaries (0.76 `--full`, 0.79 `--full`, 0.81 default, 0.84
default, 0.84 `--minimal`):

- **Precedence is release-keyed, not per-token.** C Tcl merges the
  comparison operators into two levels; Jim splits them across four-plus
  (`in ni` 55, `eq ne =* =~` 60, `== !=` 70, `lt gt le ge` 75, `< > <= >=`
  80), so `expr {"a" eq "b" == 1}` parses as `("a" eq "b") == 1` under Tcl
  and `"a" eq ("b" == 1)` under Jim. `**` bound at 250 and
  left-associative at 0.76 and at 120, right-associative from 0.77:
  `expr {-2 ** 2}` is −4 on 0.76 and 4 from 0.77. `lt`/`le`/`gt`/`ge`
  arrive at **0.80**.
- **Symbolic operators need lexer recognition**: `<<<` tokenises as one
  operator and `=~`/`=*` must not lex as `=` + junk. `=*`/`=~` are iRules'
  `matches_glob`/`matches_regex` at Jim's spelling and precedence 60 — one
  registry-level identity, three lexical homes.
- **Mathfunc membership is a set per core profile**: 26 from 0.77 (23 at
  0.76 — `atan2`, `hypot`, `fmod` arrive at 0.77), and `entier`, `bool`,
  `min`, `max`, `isqrt` appear at no Jim tag although each is a real C
  Tcl 8.5 function.
- **`expr`'s own arity is core-keyed**: `expr 1 + 2` yields 3 through Jim
  0.80 and is `wrong # args` from 0.81 (compat-gated: a `--compat` build
  still concatenates; the ladder carries the default build's value).
- **The lexical axes are Jim's own**: `${…}` closes `FirstClose`, no
  leading BOM is skipped, plus the five axes of §1.

| probe | 0.76 | 0.79 | 0.81 default | 0.84 default | 0.84 `--minimal` |
|---|---|---|---|---|---|
| `expr {sqrt(4)}` | 2.0 | 2.0 | **syntax error** | 2.0 | **syntax error** |
| `expr {int(4.7)}` | 4 | 4 | 4 | 4 | **4** |
| `expr {min(1,2)}` | error | error | error | error | error |
| `expr {atan2(1,1)}` | **error** | 0.785… | 0.785… | 0.785… | error |
| `string length é` | 1 | 1 | 1 | 1 | **2** |
| `string length [subst \U0001F600]` | 1 | 1 | 1 | 1 | **4** |
| `expr {"abc" lt "abd"}` | **error** | **error** | 1 | 1 | 1 |
| `expr {"abc" =* "a*"}` | error | error | **error** | 1 | 1 |
| `expr 1 + 2` | 3 | 3 | **wrong # args** | **wrong # args** | **wrong # args** |
| `expr {-2 ** 2}` | **-4** | 4 | 4 | 4 | 4 |
| `expr {2 ** 3 ** 2}` | **64** | 512 | 512 | 512 | 512 |
| `expr {010}` | 10 | 10 | 10 | 10 | 10 |
| `expr {1_000}` | error | error | error | **error** | **error** |

`f5_core_expr_grammar()` maps every `F5Tcl`-cored profile to the model
`ExprGrammar`; the expr lexer reads that table and W003 keys on the family
grammar, so tmsh and iApp documents accept the operators they measurably
accept. `RuntimeExprSurface::for_tcl_version` survives for the plain-Tcl
ladder (§11 D9's neighbour, the centralisation ledger's C12).

Where a family's data lives: the shipped cores are native Rust; a pack may
declare a further family with a `dialect` block (§6.2), setting values for
the axes Rust defines — a new axis is a Rust change, because the lexer
must implement it.

### 3.2 `Package` — providers of surface declarations

A package is a named provider of **surface declarations**: commands the
provider *may* install, each with an axis-typed applicability
`VersionSet`, an optional capability/platform predicate, and provenance.
Declarations are catalogue evidence, never proof of a live binding: Tcl
loads packages by evaluating `ifneeded` scripts that can inspect platform
state, define only some commands, select accelerator backends, or fail
after partial mutation (tcllib's `try`, `snit`, and `sha1` all do).

- **Multi-train truth.** A command may carry several disjoint
  applicability sets (`struct::graph` 1.x and 2.x shapes coexist; the
  resolver picks the train `package require`'s requirement selects).
  `tcl_registry::model::tcllib::TCLLIB_MODULES` is a 200-row census read
  out of `tmp/tcllib-2.0` — each module's `package require` name, the
  trains its `pkgIndex.tcl` offers, its `package require Tcl` floor —
  supplying every tcllib declaration's applicability on the module's
  **own** axis; seven modules (`md5`, `sha1`, `snit`, `struct::tree`,
  `struct::graph`, `doctools::idx`, `doctools::toc`) are genuine parallel
  trains. `UNBACKED_PACKAGE_NAMES` records the catalogue names no
  `package provide` backs, and `the_identity_census_is_closed` fails the
  build if a new one appears.
- **Dynamic-surface honesty.** A pack declares that a provider's member
  set is runtime-extensible (`dynamic_surface` / `unknown_members`)
  instead of pretending closure — `struct::tree` discovers its method set
  with `info commands`, `oo::dialect` manufactures definition DSLs.
- **Package identity is not a flat alias table.** Tk 9 registers
  lowercase `tk` as the loading package and provides uppercase `Tk`
  through an `ifneeded` chain requiring the exact lowercase version, only
  when built without `TK_NO_DEPRECATED`. `co_provides` is parsed and
  carried as data; Tk keeps its own version axis — never `tracks-base` —
  and compatibility with Tcl is a requirement relation.
- **Placement claims.** `Tk` is placed **ambient** under `tk` and
  **hosted** under every plain-Tcl environment, and that pairing forces
  the model to distinguish a *library with an ambient host* from a
  *closed-world vendor runtime*: `is_closed_world_package` is derived
  from the placement data (ambient somewhere ∧ hosted nowhere), never a
  name list. No compiled environment places a tcllib module, so none is
  closed-world (`hosted_modules_are_never_ambient`); the floor comes from
  the document's own `package require`, and the per-module Tcl floor
  gates the whole distribution consistently.
- **Core surfaces are providers too.** `package require Tcl 8.5` is real
  Tcl; the core surface rides the same algebra as provider `tcl` (and
  `jim`, `f5-irules`), so one window mechanism gates everything from
  `lmap` (tcl ≥8.6) to `case` (tcl 8.4..8.6) to `lsubst` (jim ≥0.84) to
  `struct::graph::op` (package ≥0.11).
- **Packages take range targets exactly like cores** (§5.4).

### 3.3 `Environment` — the selectable, aliasable identity

```rust
// rust/tcl-dialect/src/model/environment.rs
pub struct EnvironmentDefinition {
    id: EnvironmentId,                     // canonical, reserved or namespaced
    aliases: Vec<Arc<str>>,                // "irules", "tcl-irule", "wish", …
    display_name: Arc<str>,
    editor_identity: Option<EditorLanguageIdentityId>, // from the FIXED contributed set
    core: Option<CoreProfileSelector>,     // family + build profile + default release
    targets: VersionSet,                   // per-axis target sets — §5.4
    expected_packages: Vec<PackagePlacement>, // Pinned | TracksBase | Keyed | Requirement; ambient/hosted
    policy_defaults: EnvironmentPolicy,    // WorldPolicy {Open, Closed, AmbientPlusRequire}, fixed ensembles, ceiling, …
    server_detection: DetectionFacts,      // file_extensions, filenames, content signatures, shebang words
    help_terms: Vec<Arc<str>>,
    provenance: Provenance,                // BuiltIn | BundledPack | User | WorkspaceTrusted | WorkspaceUntrusted | StudioOverride | Document
}
```

**Environments are dynamic — as definitions plus overlays, never
mutation.** Definitions come from the compiled core set (family ladders,
`f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`, `tk`, `spectcl`, `bpf`,
`sslictcl`), from pack-declared `environment` blocks (§6.2; the six EDA
shells and their generated seed), and from workspace and user
configuration. Adjustments to a named environment are
`EnvironmentOverlay`s whose content hash, origin, and trust class are part
of the resolved identity. The collision contract: **all compiled
canonical names are reserved**; third-party environments get
pack-name-prefixed ids (`PACK/DIALECT`) plus display names; alias cycles
and same-precedence collisions are load errors; file-detection precedence
is a separate ladder that reports *ambiguity* rather than first-wins.
Environments are `Arc` values with generation-keyed identity —
`tcl_registry::model::registration::sync_environment_sources` swaps a
rebuilt registry in at the next generation, transactionally and
idempotently, so an unchanged reload invalidates nothing and a pack that
has left the workspace retires its environments — unlike the loaded
`CommandSpec`s, which stay leaked-static (§11 D10).

**Editor identity is split out.** VS Code and Zed language ids,
extensions, and filename patterns are extension-manifest contribution
points, fixed at install time. `EditorLanguageIdentity` is a fixed,
generated, contributed set (`tcl`, `tcl84`…`tcl91`, `tcl-irule`,
`tcl-iapp`, `tcl-bigip`, `tclspec`, `sslictcl`, the six `tcl-<vendor>`
ids, …), and dynamic server environments *select among* them. A pack may
request detection patterns; the editor adapter reports whether it can
apply them (VS Code: workspace `files.associations`; JetBrains: IDE-global
`FileTypeManager` associations with a persisted ledger; Zed/Sublime:
static manifests only) — [spec-packs.md](spec-packs.md) § *Editor
registration*.

- **Environments are the only user-facing names.** All six ingress kinds —
  `# tcl-dialect:` directives, `tclLsp.dialect` settings and
  `folderDialects`, LSP language ids, `--dialect` CLI flags, MCP tool
  enums, pack `file_extension … -dialect` rows — resolve through **one**
  function, `tcl_registry::model::ingress::resolve_environment` (the
  validating form is `resolve_known_environment`). The directive resolves
  canonical ids, aliases, contributed editor identities and pack-declared
  environments alike; a name no environment carries makes the tier
  abstain.
- **The alias layer is this table.** `tk` → environment `tk`; retired
  spellings are `aliases` rows. Per-release names (`tcl8.4` … `tcl9.1`)
  are environment rows over the family ladders.
- **Policy absorbs the last profile stragglers**: `has_fixed_ensembles`,
  the iApps W108 strict-ASCII rule, the version ceiling, and closed-world
  resolution (§5.3). tcllib's exclusion from the F5 shells is environment
  membership under `AmbientPlusRequire`, not a subtraction.

## 4. Availability: catalogue declarations, version sets, and binding knowledge

Availability is **two layers**: a declarative catalogue algebra used at
every level (command, subcommand, sub-subcommand, option, option value,
side-effect, special variable, form), and a realm-scoped binding-knowledge
layer that semantic passes query. There is no per-release bitmask: a spec
*states* `SpecSurface` rows and a context *asks* at a `SurfaceQuery`
point.

### 4.1 The catalogue layer — surface declarations and version sets

```rust
pub enum Provider { Core(Family), Package(PackageId), Document }

pub struct VersionAxisId(/* interned typed axis: core(family), package(name), document(), big_ip(), tmsh_syntax() */);

pub struct VersionSet { axis: VersionAxisId, ranges: Arc<[HalfOpenRange]> } // normalised, disjoint

pub struct ItemHistory { introduced, deprecated, retired: Option<Version> } // one item's own story on one axis

pub struct SurfaceDeclaration {          // rust/tcl-registry/src/model/surface.rs
    provider: Provider,
    applicable: VersionSet,              // when this shape exists — parallel trains = several sets
    predicate: CapabilityPredicate,      // build/platform/feature conditions
    history: ItemHistory,
    provenance: Provenance,              // trust class — §6.4
}
```

Two version types, deliberately: `Lifecycle`/`ItemHistory` answers "when
was this one item introduced, deprecated, retired"; `VersionSet` answers
requirement/target set algebra. Tcl requirements are **alternatives of
ranges with exclusive maxima** (`8.5-9.0` excludes 9.0; `8.5` alone
excludes 9.0.4; `8.5 9.0-9.1` is a union that admits it), so requirements
and targets are normalised unions of half-open ranges. Every set carries
its axis: a Tcl core `Release`, a package version, a BIG-IP release, and an
ECharts release are not comparable by accident (invariant I2 — intersecting
across axes is a typed `AxisMismatch`), and the normaliser plus
`contains`/`intersect`/`subset` are differentially tested against real
`package vsatisfies`. Wherever a range is written, bound inclusivity is
explicit.

- **Core deltas are declarations, not bits**: `lmap` is
  `[{Core(tcl), 8.6-}]`; `case` is `[{Core(tcl), 8.4-9.0}]`; a command
  shared with Jim adds `{Core(jim), 0.76-}` to the same spec.
- **Authoring precedence is not resolution.** Most-specific-wins
  (narrowest provider set beats widest, `CommandRegistry::best_visible`)
  decides only which declaration a catalogue author intended to override.
  It never decides which command Tcl will call — that is the next
  layer's job.
- Inline `# tcl-lsp: stub` blocks and `.tcl.stubs` sidecars ingest the
  same way (`tcl_registry::model::declaration`): provider
  `Provider::Document`, the `VersionAxisId::document()` axis,
  `Provenance::Document` for a buffer and `Provenance::WorkspaceUntrusted`
  for a sidecar, read through the one `DocumentCommandSurface` door. The
  surface's role lookup unions the catalogue's answer with the document's:
  an untrusted addition may improve assistance and can never weaken a
  shipped analysis fact.

### 4.2 The binding layer — realms and knowledge

```rust
pub enum BindingKnowledge {              // rust/tcl-registry/src/model/binding.rs
    Absent,
    Must(BindingTarget),                 // proved: this binding, here (Spec — the hook licence — or Document)
    May(Arc<[BindingTarget]>),           // candidates; order/branch not proved
    Unknown,                             // dynamic loader, unknown interp target, …
}
```

Package state is **per interpreter and temporal**: Tcl keeps the package
table on the interpreter, `ifneeded`/`unknown` run arbitrary scripts, a
child interpreter inherits nothing, a safe child hides core commands while
providing the same `Tcl` version, and `package provide Demo 1.0` survives
`rename demo {}`. The transitions that update realm state — `package`,
`source`, `proc`, `rename`, `namespace import` (ordinary vs `-force`
differ observably), `interp alias`, `interp hide`/`expose`,
child-interpreter operations — have one vocabulary,
`rust/tcl-registry/src/state_transition.rs` (`CommandBindingTransition`
and its three stock descriptors, which every shipped mutator names).
Dynamic operands widen the affected domain to `May`/`Unknown`.

The single realm is what ships: the compiler's document realm scan
(`tcl_compiler::realm` — the offset-keyed top-level command-binding state)
and the analyser's one `exists` oracle
(`command_existence_oracle`/`command_binding_knowledge`) produce
`BindingKnowledge` per program point, W123 fires exactly on `Absent`, and
settlement, const-dispatch, W113 and W123 share one context-filtered
registry set (`builtin_command_names`). Child interpreters, safe
interpreters and `hide`/`expose` widen everything they touch to `Unknown`
(§11 D8).

The consumer contract (invariants I3–I5): **no taint, side-effect,
lowering, or codegen hook is selected before its binding is proved** — the
model's three selection primitives require the head to resolve to a
spec's declaration under the document's environment
(`ResolvedContext::resolve_spec`); `Absent` ⇒ no selection; no context ⇒
`NotRequired`, and every deliberately context-less reader is a documented
widening query. Ambiguity takes the conservative union of effects or
abstains, never a candidate by catalogue order. The side-effect hint walk
stays inside the primitive because proved-single-winner selection is
measured non-equivalent at nine points (`c7_hint_walk_counterexamples`).

- **iRules' safety property is derived, not assumed.** The `f5-irules`
  environment is closed-world *policy* over an explicit allow-list
  surface, and because iRules has no dynamic binding machinery the realm
  state stays `Must`-almost-everywhere.
- **Known-anywhere has four tiers**: globally documented;
  installable/indexed for this project; expected from the selected
  environment; and must/may-active in this realm. Security and
  compilation queries use realm bindings only; completion may opt into
  the broader tiers with annotations. W002's candidate source is still a
  hardcoded pack list (`all_dialect_command_names`, the centralisation
  ledger's C10).

## 5. Resolution: from bytes to environment, targets, and realms

### 5.1 Environment resolution (ingress)

The detection chain: directive → shebang (`wish` selects `tk`; `tclsh8.5`
selects `tcl8.5`; `jimsh` selects `jim`) → tokenised `package require Tcl`
guard → content signatures → filename/extension (pack-declared extensions
first) → configured default. Detection facts live on environments,
compiled or pack-declared. `registration::extension_routes` is a pure
function of the loaded pack set (explicit `file_extension … -dialect D`
rows first, then each `environment` block's own detection rows, first
claim winning), published at the pack *merge* so an identical reload
cannot lose it, and advertised to the client (`pack_file_extensions`) so
an editor opens the file as Tcl at all. `# tcl-dialect: NAME` resolves
through `resolve_known_environment`: canonical ids, aliases (`wish` →
`tk`, `irules` → `f5-irules`), contributed editor identities, and any
environment a loaded pack declares.

### 5.2 Activation and `package require` processing

Per document, in order:

1. **Environment** gives the core profile and the expected placements
   (iRules: the F5 surface ambient at the BIG-IP-keyed version; `tk`: Tk
   ambient on its own axis; `xilinx-eda-tcl`: `vivado` at the
   ToolVersion-keyed floor).
2. **Workspace facts** add hosted availability and floors: discovered
   packs (bundled/user/workspace tiers), `tclpkg.tcl` manifests, and
   `pkgIndex.tcl` files the `PackageResolver` indexes.
3. **Document facts**: the `package require` scan (name, version
   requirements, `-exact`, conditional flag) selects, per package, the
   best satisfying version train and floor using the ported
   `vsatisfies`/`vcompare` algebra — including multi-requirement unions
   (`8.5 9`) and same-major selection (`require struct::graph 1.2` picks
   the 1.x train even when 2.4.4 is present). Cross-file inheritance over
   the source graph applies.

**This chain feeds two differently-named APIs.** The *assistance view* —
completion, hover annotations, W120 — keeps whole-file activation as an
explicitly labelled heuristic; `H301` (Hint, default-on) reports a command
used above the `package require` that provides it. The *semantic view* —
compiler, taint, side-effects, codegen — is position-, path-, and
realm-sensitive, and unknown control flow widens. The two views have
different names and types so a semantic pass cannot call the assistance
shortcut (invariant I3).

### 5.3 Strictness policy

Visibility is an environment field, `WorldPolicy`:

- `Open` (plain Tcl, EDA shells): hosted packs resolve, complete, and
  hover everywhere, with W120 as the nudge and floors gating
  version-sensitive facts. The leniency is a statement about packages
  **no environment owns as its own runtime**: a closed-world vendor
  surface stays invisible outside its own environment.
- `Closed` (`f5-irules`, `bpf`, `spectcl`, `sslictcl`): only the ambient
  closure exists; hosted packs and `package require` are not part of the
  language. A closed world does not resolve the Tk surface.
- `AmbientPlusRequire` (`f5-iapps`, `f5-tmsh`): the ambient surface plus
  what the source required; a hosted pack it never required is absent.

`ResolvedContext::package_active` is the availability question;
`package_provider_active` is the carrier question ("does something here
actually provide this?"). W120 is silent where the package is ambient
(`tk` / `wish`), unchanged elsewhere.

### 5.4 Version-range targeting (multi-target projects)

A project declares that it supports a *range* of targets — `tcl 8.5–9.0`,
and equally a *library* range such as `struct 1.5–2.2` or `Tk 8.5–9.0` —
and the analyser warns about anything that is not valid across the whole
declared range. A target is a `VersionSet` per provider, cores and
packages alike.

**Where targets come from** (intersected, most specific wins per
provider):

1. The environment's `targets` field (default: a single release; the
   feature is off for single targets).
2. `tclLsp.targets` (settings → `AnalyserConfig::targets` →
   `Analyser::with_declared_targets`; VS Code exposes the object setting).
3. The top-of-file `# tcl-lsp: supports NAME RANGE` directive, parsed
   beside the `disable` directives; the directive wins per provider over
   settings. `tcl` names the core axis (honoured only on a Tcl-family
   core — `supports jim …` under `tcl8.6` is dropped, and the family name
   matches any `Family::name()`); any other `NAME` is a package axis.
   Malformed declarations are dropped, never guessed at.
4. The document's own `package require` facts (a requirement is already
   an interval under the `vsatisfies` algebra).

The `tclpkg.tcl` manifest is not a target source: its grammar accepts one
operator + one version (§11 D18, R6).

**Targets grammar.** Clauses are space-separated and union; a bare `V`
names **that release line only**; `MIN-` is open-ended (clamped to the
modelled ladder on a core axis); `MIN-MAX` runs through the **whole line
of `MAX`** — `tcl 8.5-9.0` includes 9.0.x, because the strict vsatisfies
exclusive-max reading would silently drop the release the declaration
names. On package axes a line is `[V, V+ε)`. Implemented as
`targets_from_clauses`.

**Correctness is defined first.** Compatibility means the relevant parse
and semantic facts agree for **every selected target** — not merely at the
endpoints: targets can be non-contiguous, an axis can change and change
back, and a grammar delta can alter *word structure* (Jim's quote
termination, brace continuation, variable syntax). The reference for that
definition is a per-distinct-grammar evaluation over the finite set;
per-axis detectors are optimisations for one profile pair, admitted only
after a differential gate proves them equivalent. What ships is the pair
that provably diverges token-locally — lifecycle windows and numerals; the
reference evaluator and the other axes are §11 D16. Assistance
(completion, hover, signature help) answers under the environment's
`primary`; a declared range never moves it (§11 O7).

**Model.** `ResolvedContext` carries **declared** target sets separately
from the environment floors (`declare_targets` / `declared_targets` /
`declared_target_sets`), with `targets_outside_window(axis, introduced,
retired)` for lifecycle-spelled items and `targets_uncovered_by_gate` for
surface-spelled ones, and `requirement_spelling` / `ladder_releases_in`
naming the failing targets in messages.

**Diagnostics.** **W150** — item not available across the whole declared
range: the version-gate flush evaluates every lifecycle site against the
declared window, the mask-gated command/subcommand/option sites against
gate coverage, and the argument-DSL sites (`string is` classes,
format/scan conversions, binary modifiers) against their minimum release
("`'lmap'` requires Tcl 8.6 but the declared targets include 8.5").
**W151** — the numeral grammar delta: whole-word literals are read under
every `NumberSyntax` era the declared range spans; a value divergence
(`expr {010}`: 8 under 8.x, 10 under 9.0) or validity divergence
(`0b`/`0o` before 8.5, `0d`/`_` before 9.0) warns. A failure at the
**primary** keeps today's semantic-floor diagnostic
(W002/W135/W136/W139/W144) and always outranks the range warning at the
same word; W150/W151 fire only for items clean at the primary, at Warning
severity. The package half needs no separate code: `supports Tk 8.5-8.6`
gates Tk items on the Tk axis through `lifecycle_axis`, and `supports
struct::tree 1.2-2.2` warns on `::struct::tree::prune` naming the failing
targets.

## 6. SpecTcl 2.0 (`speclib … 2.0`)

A pack file is a Tcl program evaluated in the sandboxed tclvm whose
registration calls produce a frozen registry snapshot
([spec-packs.md](spec-packs.md) § *Authoring rules*). The declarative
vocabulary below is what those registration commands accept.

### 6.1 Compatibility contract

**Reading older packs stays maximal; degrading newer packs fails closed.**
The loader keeps a single reader — every word ever ratified is readable
forever, and where 2.0 changes *meaning*, the change is a **new word plus
a translation of the legacy word**, never per-version dispatch:

- `dialects {…}` (1.x) keeps loading forever: its vocabulary (`tcl8.5+`,
  `all-tcl`, `tk`, `f5-iapps`, `irules`, …) is translated at load through
  the environment alias table into `available` rows, and a body spelled
  either way loads to a byte-equal `CommandSpec`
  (`available_and_dialects_load_byte_equal_specs`). `tcl spec upgrade`
  rewrites mechanically.
- `VOCABULARY_VERSION` (the cache key) is `"2"`: one bump, because
  translation output changed.

The **forward** direction — an older loader meeting newer vocabulary —
classifies vocabulary by compatibility effect (`VocabularyClass`,
invariant I9): **presentation** unknowns warn and drop; **assistance**
unknowns (shapes, roles, value sets) mark the loaded `PackCommand`
`degraded`, so the affected capability reports `Unknown`; **semantic**
unknowns (security, control flow, binding, lowering, codegen — and every
unknown word inside a `dialect` or `environment` block) exclude the
command or block from strong analysis, with a distinct notice class. The
escalation applies **only** to a pack declaring a vocabulary this build
postdates; an unknown word in a vocabulary the build knows in full is an
author's typo and keeps the warn-and-drop treatment. An unsupported
**major** fails the whole pack closed: nothing loads, `Pack::load_error`
is `Some(LoadError::UnsupportedMajor)`, and one notice says why. An
unknown *minor* within a supported major keeps loading maximally.

### 6.2 The 2.0 vocabulary

`KNOWN_VOCABULARY_VERSIONS` / `NEWEST_VOCABULARY_VERSION` name what the
loader speaks (2.1 is newest); the spelling tables are
[`spec-dsl-examples/README.md`](spec-dsl-examples/README.md).

| Word | Purpose |
|---|---|
| `available {PROVIDER WINDOW…}` / `-available`, at every scope `dialects` is accepted (pack `default`, `command`, `subcommand`, `sub_subcommand`, `option`, object-class method, `form`, `side_effect`, the option relations) | the §4 algebra: `available {tcl 8.6-} {jim 0.78-}` / `available {package Tk 8.5-8.6}`, with `RANGE` in Tcl requirement syntax (`8.6-`, `8.4-9.0`, or a bare `8.5` naming that release line only). Replaces `dialects` + implicit `required_package` gating. A package version window is validated and reported but only the name is carried (§11 D17-P) |
| `provides NAME VERSION ?VERSION…?` (pack level) | the package trains this pack describes, including parallel majors; commands default their provider to it |
| `environment NAME { … }` (pack level) | an environment definition — `core`, `ambient` / `hosted` placements (`Pinned` / `tracks-base` / `keyed KEY` / requirement sets), `alias`, `editor_identity` (from the **fixed contributed set**, never minting an id), `file_extension` / `filename` / `signature`, `display_name`, `policy`, `help_terms`, `version_ceiling`. Converted at the declaring tier's `Provenance` and registered through the one publish point (`bundled::set_active` → `registration::publish_pack_set`), so a pack's environments resolve through the ordinary ingress, route documents by claimed extension, and retire when the pack leaves the workspace. Compiled canonical names and aliases are reserved. The block body is an evaluated scope: a version several environments share is one variable. `environment NAME -extend { … }` adds detection facts and placements to an environment declared elsewhere |
| `co_provides` | parsed and carried as data (Tk 9's predicated `tk`/`Tk` relation); the loader-alias mechanics that consume it are not built |
| `dynamic_surface` / `unknown_members` | the honesty escape hatch: a provider whose member set is runtime-extensible declares so (`allow_unknown_subcommands` on commands, `allow_unknown_methods` on `object_class`) |
| `dialect NAME { … }` (pack level) | a language family: `release R ?-build P?` ladder rows and `axis NAME VALUE` rows against a closed axis vocabulary (`expand_syntax`, `braced_var`, `expr_comments`, `numbers`, `escapes`, `irules_brace_separator`, `bom_skip`). An unknown axis or value rejects the whole block, naming the axis; a block whose axes reproduce a compiled family release is rejected naming the environment it should have been. A validated block becomes a `tcl_dialect::model::DynamicFamily` (namespaced `PACK/DIALECT` id, one `LexerGrammar` per release, the declaring tier's provenance), and an `environment … { core DIALECT RELEASE }` row registers a `DynamicCore` binding whose grammar `dynamic_core_grammar` answers with. Grammar declarations sit at the top of the §6.4 trust lattice. **Nothing on the analysis path lexes with it yet** (§11 D5) |
| `refine NAME { … }` at command and subcommand scope | the invocation refinement — the declarative replacement for `command_forms` / `subcommand_forms`; Tk's form sites round-trip through it |
| `include from SOURCE into TARGET ?-available {WINDOW}? {names…}` | the roster of ancestor command names a *reimplementing* family actually has, with the window on the **target's** own ladder. An unrostered pair fails **open**; only a trusted tier may narrow a compiled family. Jim's roster is the one user |
| `result_stability`, `event_requirement_form` (with a nested `event_requires` block), `data_collection -native ID`, `body_scope`, `side_switch_target`, `event_handler_priority` | loader readers at the shared row-reader seam, riding the export gates |
| `bpf_op -native ID` | documented and **unread** (§11 D3) |

### 6.3 Structural rules

- **Hooks in shipped packs stay native.** Performance-critical resolvers
  (17 Tk `script_timing_resolver`s sit on the semantic-tokens hot path;
  28 µs Tcl-body vs 410 ns native) are referenced as `… -native ID`;
  community packs use Tcl bodies with `-inputs` shape-caching.
- **Cache honesty.** `EvalSnapshotKey` (content × vocabulary × loader-eval
  version × tier) is stamped with the build; there is no hand-maintained
  loader-build constant.
- **The migration gate is representation *and* behaviour** (invariant
  I10): byte-compared registry dumps only prove the new form preserves
  what the old form said. A surface conversion adds behavioural-parity
  fixtures (completion, hover, semantic token roles, arity, control flow,
  taint, side effects, deprecation, binding transitions) grounded in
  upstream source. No shipped surface has been converted: Tk and tcllib
  are packages with real placements whose *declarations* are compiled
  Rust, and a conversion still needs SpecTcl vocabulary for the
  widget-class and option-database shapes, the Tk co-provide model, and
  the parity harness.
- **Open**: registry generations (§11 D10), a shared `InvocationSpec`
  (D11), and a loader-direction round-trip gate that would make a
  documented-but-unread word fail CI (§11.4 E3).

### 6.4 Trust and provenance

Nearest-wins tier precedence is an *editing* model, not a security
lattice. Every declaration and resolved fact carries provenance and a
trust class — `BuiltIn`, `BundledPack`, `User`, `WorkspaceTrusted`,
`WorkspaceUntrusted`, `StudioOverride`, `Document` — and merges are
capability-specific:

- ordinary prose (hover, display names, docs) merges by authoring
  precedence;
- **security facts merge monotonically** (invariant I6):
  `tcl-registry::security_floor` applies to **every** override from
  **every** tier — set-valued security facts union, single-valued ones
  keep the shipped value — so a four-line workspace pack declaring
  `command exec -override { arity 1.. }` cannot strip `exec`'s
  `TAINT_SINK` (`rust/tcl-spectcl/tests/i6_security_floor.rs`). The floor
  is deliberately not tier-keyed: nothing on the discovery path is told
  the editor's Workspace Trust state (§11 O9);
- the workspace and studio tiers cannot `-override` a compiled command
  name, extend a compiled environment, declare a `dialect` block, or claim
  a reserved environment name — refused at registration with the
  provenance named;
- per-pack consent for security-weakening overrides and the "which tier
  won this fact" hover field are not built (§11 O8).

## 7. What of the old surface still stands

`DialectProfile` with `PLAIN_TCL` and `TK_PROFILE` as interned statics the
seam consumes — the lexer's grammar key (§11 D5) and the editor
catalogues' identity key; `KNOWN_DIALECTS` as the CLI `--dialect`, MCP
`dialect_schema` and explorer payload vocabulary (§11 D15); and
`ProfileQueries`, `pub(crate)` to `tcl-registry`. Everything else the
model replaced is deleted and held at zero by `cargo xtask
retired-api-gate`.

## 8. Invariants

These hold across the tree and every lane cites the ones it gates on:

| ID | Invariant | Gate |
|---|---|---|
| I1 | Equal core-profile ids imply equal measured syntax/core semantics | cross-build and cross-release oracle matrix |
| I2 | Values from different version axes cannot be compared | typed `AxisMismatch`; property tests |
| I3 | Package and binding facts are scoped to an interpreter realm and program point | assistance/semantic API split by type |
| I4 | No taint/effect/lowering/codegen hook is selected before binding proof | the three selection primitives require the environment binding proof |
| I5 | Ambiguity widens effects or abstains; it never picks by catalogue order | load/import/rename permutation tests |
| I6 | Untrusted data cannot weaken trusted security facts | `security_floor`; `i6_security_floor.rs` |
| I7 | Dropped registry generations release dynamic specs | **not held** — §11 D10 |
| I8 | Every advertised editor identity is actually contributed by that editor package | the fixed `EditorLanguageIdentity` set |
| I9 | Unknown semantic vocabulary fails closed | `VocabularyClass`; downgrade fixtures |
| I10 | Pack migration preserves user-observable behaviour, not only serialised bytes | golden-snapshot gate plus behavioural parity per conversion |

## 11. The open-questions ledger

**Everything this programme has not closed, in one place.** Each row says
*what it is*, *what it blocks*, and *what would resolve it*. Where a phase,
ruling, or ledger row elsewhere says "still open", it points here.

Deliberately **not** in this ledger: items the owner ruled out (a
per-major `tcl` family split; `tcl8`/`tcl9` named-range sugar; contraction
of a programmed pack back into a program; an ahead-of-time `.tclspec` →
`.rs` backend — the shipped cores stay native Rust), and the six
*deliberate* F5 divergences the conformance corpus records.

### 11.1 Owner decisions pending

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| O3 | **Standing studio overrides are not yet visible.** The `StudioOverride` patch-pack tier is ratified with an amendment: a studio indicator, and a `spectcl_check` warning once a patch has outlived a threshold, so an override reads as a staging area rather than a home. `PackStore::standing_overrides` is the queryable report; neither surface reads it yet | Nothing about correctness | The indicator and the warning |
| O4 | **`spectcl_check` has no tier parameter.** It evaluates the pack as trusted (the author's own file) and appends the `Tier::Workspace` provenance verdict as a notice. The ruling wants an explicit tier defaulting to the tier the pack would actually install at, with the trusted view available on request | A check that predicts a user-tier install exactly | The parameter and its default |
| O5 | **The iRules surface as a pack.** Six of the seven prerequisite words have loader readers, so the deferral is a standing choice rather than a blocked one | The iRules command surface stays compiled Rust. The dialect (grammar, structure) and the closed-world policy stay compiled either way, so nothing about correctness rides on it | An owner decision to schedule it |
| O7 | **`primary` for a multi-target project.** The primary is the environment's, always; a declared range never moves it | Multi-target projects cannot choose which release assistance answers under. Compatibility checking evaluates the whole set, so this is an assistance-quality gap | An owner answer plus the settings/directive surface to carry it |
| O8 | **Per-pack trust consent and provenance in hover.** The registration-time tier gate exists; the consent surface and the "which tier won this fact" hover do not | A workspace pack cannot be granted security-fact overrides at all, which is the safe direction | An owner ruling on where consent is recorded, then the config surface and the hover field |
| O9 | **Workspace Trust is not plumbed.** `Provenance::WorkspaceUntrusted` is unreachable from pack discovery, so the editor's trust state cannot gate wholesale environment override; the security floor (§6.4) holds regardless | O8, and any rule that must distinguish a trusted workspace from an untrusted one | Plumbing the LSP client's Workspace Trust state to `discovery`, and deciding the behaviour for clients that do not report it |

### 11.2 Deferred model items

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| D3 | **`bpf_op`'s id catalogue.** `CommandSpec::bpf_op` is `Option<&'static BpfOpSpec>` and every shipped value is a private per-command `static OP` under `tcl-registry/src/commands/bpf/`, so `bpf_op -native ID` has nothing to resolve `ID` against | A `bpf` surface pack; and it is the live proof that a loader-direction gate (§11.4 E3) does not exist | A named `id → &'static BpfOpSpec` table in `tcl_registry::bpf_op`, then the ordinary `-native ID` reader |
| D4 | **Body-scoped completion codes.** `::struct::tree::prune` exits with completion code 5, but *only inside a `walk` body*. A body slot carries a timing and a kind, never the set of codes its command consumes, so `prune` carries **no** control-flow trait (`CONTINUES_LOOP` would be a lie the CFG builder acts on) and the CFG for a `struct::tree` walk is wrong-but-conservative | Every library-defined completion code | `body_completion_codes: &[(u8, CompletionCode, &str)]` on `SubCommand`/`OptionSpec`, plus scoping the `BREAKS_LOOP` machinery to that body |
| D5 | **Pack-declared dialects cannot lex.** A validated `dialect` block is a `DynamicFamily` with a real `LexerGrammar` per release, and an `environment … { core DIALECT RELEASE }` binding resolves it — but `LexerConfig::for_dialect(name)` is `from_grammar(grammar_of_dialect_name(Some(name)))`, whose `DialectPoint::of_dialect_name` reads the compiled environments' `core` selector, and a pack-declared environment carries `core: None` plus a `DynamicCore` binding, so its name sinks to the default grammar. The salsa lexer key already interns the environment **id**, so the lexer is asked for a grammar by id at exactly one door | The `dialect` block's whole point: a third-party family that actually parses differently | Give `EnvironmentDefinition` its own `LexerGrammar` (and runtime release) — from the compiled profile for a catalogue environment, from `DynamicFamily` for a pack-declared one — and replace `for_dialect(name)` with `for_environment(&ResolvedContext)` at the ~200 boundary call sites. `Family`'s closed enum can stay; `SemanticContext::runtime_version` is the only other surviving profile projection |
| D7 | **A byte-counting character model.** A non-utf8 Jim build counts bytes, a third rule `StringCharacterModel`'s two Tcl models cannot express; `character_model` answers `None` (every consumer abstains) and the measured fact travels on `CapabilitySet::utf8_character_model` | `string length` folding under `--minimal` jim | A `StringCharacterModel::Bytes` variant and its `count_for` agreement rule, which changes constant folding for **every** dialect and needs the differential gate |
| D8 | **The multi-realm `AnalysisWorld`.** Child interpreters, safe interpreters and `hide`/`expose` widen everything they touch to `Unknown` rather than being modelled | Precision only: an `interp create` widens more than it must | Build the realm map on `BindingKnowledge` and the transition vocabulary, with the parent/child/safe/ordering e2e suite invariant I3 names |
| D9 | **The analyser's binding tables are indexes, not typed transitions.** `command_aliases` / `renamed_commands` / `deleted_commands` and the offset maps are populated from `CommandBindingTransition` facts but are `AnalysisResult`-shaped values, and `indirection.rs` walks them in that form | D8 wants one vocabulary; the "no parallel binding tables" half of the one-oracle rule is gated rather than structural | Re-typing the tables reaches ~30 files across `tcl-lsp-core`'s navigation providers — a lane of its own |
| D10 | **Registry generations** (invariant I7). The loader leaks per load (`Box::leak` in `tcl-spectcl/src/loader.rs`); no `RegistryGeneration` type exists | A Spec Studio session editing a large surface leaks ~3.1 MB per generation of ~2,400 specs; a prerequisite for any mass conversion | Move dynamic pack specs into an arena/`Arc<RegistryGeneration>`, return generation-bound handles, key salsa on the generation id — gated by a 1,000-reload allocator test |
| D11 | **Shared `InvocationSpec`.** Taint sinks, forms, deprecation replacements and effects are copied field by field into `SubCommand` instead of living in one invocation capability model | Honest specs for method-level sinks (ticklecharts' file write, SpiceGenTcl's `runAndRead`) | The refactor, behind the four-surface parity rule |
| D15 | **The tooling payload rows.** Every *ingress* resolves through the seam; the *user-visible* payloads still enumerate `DialectProfile::all()` — the CLI's `--dialect` possible values and unknown-dialect message, the MCP `dialect_schema` enum, the studio picker, `registry-dump --all-dialects`, `listDialects`, `callback-surfaces` row ids, the hand-written Sublime `_SYNTAX_DIALECT_MAP` (which lacks `f5-tmsh`, `spectcl`, `bpf`, `sslictcl` and `microchip-libero-eda-tcl` rows and nothing fails when the catalogue moves), `_registry_data.tcl`, and the hardcoded `tcl8.6` defaults | Environment names becoming the user-facing vocabulary — also what the KCS "Applies-to" vocabulary regeneration (D18, R9) waits on | Each is a deliberate user-visible change: re-key the payload, regenerate the artefact, and accept the diff |
| D16 | **The per-distinct-profile reference evaluator** (§5.4). The two shipped detectors are token-local (lifecycle windows, numerals) | Every other range axis: escapes, `${a{b}c}`, expr comments and operators, `{*}`, the leading-BOM rule, numerals *inside* compound `expr` bodies, differential constant folding at the endpoints, `package require` satisfiability per target, per-folder `tclLsp.targets`, and the W151 fix-its | Build the multi-profile evaluation, then admit each per-pair detector after the differential gate proves it equivalent |
| D17-P | **A package version window in `available` is parsed, reported, and dropped.** `available {package Tk 8.7-}` validates the range and carries only the name, so the row admits wherever Tk is present at any version. The loader says so at load | Per-package version gating | `SurfaceQuery::packages` becomes name-and-version, and `SpecSurface::package_in`'s windows are then answerable — together with D17-J |
| D17-J | **Jim's own commands can be written but not read.** `available {jim 0.81-}` is expressible and Jim's additions (`loop`, `range`, `lsubst`, `alias`, `local`, `upcall`, `xtrace`, `ref`/`getref`/`setref`, `timerate`, `os.*`, `json::*`) enter the assembled registry — but a `jim` document's authoring point is `(Tcl, 8.6)`, the ancestry anchor, and `surface_admits` matches the queried family exactly, so a `Core(Jim)` row is admitted by no point any document asks at | Jim's additions stay unauthorable in practice, and the same shape blocks any future family that inherits a surface and adds to it | A point that carries the document's **own** family alongside its ancestry anchor — `AuthoringScope::core` becomes a small ordered list and `SurfaceQuery::core` with it, so `surface_admits` accepts a row from either, nearest first. Residue regardless: `binary` and `zlib` are on the roster unconditionally although `--minimal` compiles both out |
| D18 | **Gap rulings that did not land as code** (the centralisation companion's §4). **R2** — special variables are `special_vars.rs`'s compiled table rather than SpecTcl declarations, so Jim's `env` and picol 2's capital-initial globals have no home. **R3** — `FILE_SCOPED_ENVS` is a hardcoded one-row Rust table (`("tclpkg.tcl", &TCLPKG_MANIFEST_ENV)`) rather than a detection-scoped environment whose surface is a pack. **R4** — `render_spectcl`'s `is_dialect_set` matches `"surface" \| "safe_on_uninit"` together, conflating a behaviour predicate with availability. **R5** — the hook `ctx` dict carries only a `dialect` key; no `environment` key. **R6** — the `tclpkg.tcl` manifest's `tcl` constraint has no multi-clause range grammar, and the three version comparators (`tcl_dialect`, `tcl_registry::version`, `tcl_pkg::version`) have not collapsed. **R7** — `spectcl_check` is MCP-only; there is no `tcl spec check` verb. **R9** — the KCS "Applies-to" controlled vocabulary is unregenerated | Each blocks a different small thing | Each is independently landable |

### 11.3 Evidence gaps

Measurements the model wants and does not have. Where the model must
answer without them it abstains explicitly.

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| V1 | **The two APL contexts** (`IAppPresentationApl`, `IAppPresentationTclCallback`). Never exercised; both answer `Unknown` through both evidence doors | An APL `tcl` callback has no family, no build profile and no surface, and the model must keep refusing to copy `IAppImplementation`'s row into it. Also blocks a typed **embedded-range descriptor** for APL's `tcl` clauses — a document/range concern for the compiler and server | E4 step 6: exercise a non-interactive presentation renderer on an appliance |
| V2 | **A second and third BIG-IP build** (one supported 17.x, one older), same suites | Every "since when" question about the F5 tree is `Unknown`: nearest-known assistance answers cannot become interpolation, and the `f5-tcl` ladder's post-fork deltas stay hypothesis | Two more appliance runs |
| V3 | **A restricted-role tmsh column.** The probe corpus was run as the SSH login user, so the command surface carries no role annotation | The tmsh role/visibility overlay has no measured input and cannot be wired honestly | One appliance run under a restricted role, with `systemauth.disablebash` and the other policy settings captured |
| V4 | **The E4 re-run of the §5–§9 suites.** The 85-builtin surface, the 120-cell event matrix and the traffic lab are consumed as `e4_conforming: false` | ~180 corpus vectors stay "strong transcript" rather than ratified evidence. §11 of the measurements says the re-run is mechanical — `lib/runner.sh` needs a prefix change and a pre-create absence check | Re-run under the E4 contract |
| V5 | **`matches` precedence and semantics.** `expr {"abc" matches "abc"}` is a single-operator expression and exercises no binding power, and the same probe is exact equality and discriminates none of the equality/containment/glob readings | The VM answers string equality — the one reading the measured cell exercises — and the compiler refuses to constant-fold it | Two probes: `expr {"abcd" matches "bc"}` and `expr {1 or 0 matches 0}` |
| V6 | **The realm scope of `tmsh::modify cli version active`.** Script-, tmsh-process-, session-, or system-scoped? | Where the tmsh-syntax transition's state lives; wired with `scope_is_measured: false` | One appliance probe |
| V7 | **The runtime half of the six deliberate `RULE_INIT` divergences.** Does calling an `HTTP::*` command in `RULE_INIT` always fail at runtime, or only when it touches connection state? | Nothing in the model — the rows are meant to diverge. Confirming it would sharpen the divergence reason from policy to fact | A per-cell runtime re-probe in the traffic lab |
| V8 | **A jim probe corpus.** The five `jimsh` binaries and their transcripts are on disk, not a hermetic in-tree fixture set keyed by `(release, configure flags, platform, commit)` | Every jim claim is re-derivable only by rebuilding the interpreters; jim has no tripwire of the F5 corpus's kind | Build the corpus in the F5 shape: typed records, two-sided expectations, hermetic vectors |
| V9 | **Corpus-*generated* F5 rows, the transcript-schema validator, and the prose/rows/tests drift gate.** The 205 vectors *assert* the hand-authored F5 catalogue rows; they do not produce them | A measurement and a registry row can still be edited apart — the corpus catches it at test time rather than making it unrepresentable | The generator, the schema validator (which must never run in CI), and the drift gate |
| V10 | **The tmsh role overlay and the `tcl_platform` CMP-effect overlay.** Both are recorded as evidence and neither is wired as an overlay | The role overlay is additionally blocked by V3. The CMP overlay has its input — TMM's seven fabricated keys are pinned against `special_vars.rs` | Wire the effect refinement; for the role overlay, V3 first |
| V11 | **Single-binary probes on the five-version matrix.** `find_tclsh`'s first-hit callers were not upgraded when the five interpreters started building | Release-differentiating behaviours probed by a single binary answer for whichever tclsh is first on `PATH` | Thread `TCL_LSP_TCLSH{84,85,86,90,91}` through those call sites |
| V12 | **The oracle programme's vector domains** (companion §7.2–§7.5). The reference interpreters, the path fixes, the Tk trees and the command/variable/namespace-op vector files exist; the binding vectors, the package/autoload vectors, the real-corpus index parity and the consumer conformance lattice do not | The lattice is the "every consumer leverages this properly" checklist, and it is not yet a set of passing gates | Land the vector files per §7.2's format and wire the five consumers |

### 11.4 Doc-and-code divergences

| # | The claim | The code | Disposition |
|---|---|---|---|
| E3 | §6.3 once promised a loader-side direction to the round-trip gate, "so a ratified word without a loader arm fails CI instead of silently dropping" | No such gate exists. `bpf_op` is documented, unread, and nothing fails | **Open** (D3 is its live instance). The golden-snapshot gate, the static-fast-path gate and the export gates are real, but none exercises a *documented-but-unimplemented* word |
