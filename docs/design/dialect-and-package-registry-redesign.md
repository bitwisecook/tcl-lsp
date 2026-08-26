# Dialects, packages, and environments — the registry redesign (issue #1631)

> **Status: PROPOSAL, revision 2.** This is the design for the post-release
> architecture directed in issue #1631, researched on branch
> `claude/tcl-dialect-registry-design-lrzbsn` (2026-08-25). Nothing here is
> implemented. Revision 2 (2026-08-26) incorporates the
> [adversarial review](dialect-and-package-registry-redesign-adversarial-review.md)
> of revision 1: all thirteen blocking findings are **accepted** and the model
> is corrected accordingly — §0.1 records the disposition. §10 lists the open
> questions whose answers gate the design; recommendations are marked. Where
> this document and [dialect-profile-model.md](dialect-profile-model.md)
> disagree, this document describes the *intended* model and that one
> describes the *shipping* model.

Companions:
[dialect-and-package-registry-centralisation.md](dialect-and-package-registry-centralisation.md)
(the end-to-end registration/resolution audit, retirement ledger, and
`tcl spec upgrade` specification), [spec-packs.md](spec-packs.md) (the
SpecTcl format contract this extends),
[eda-library-packages.md](eda-library-packages.md) (the precedent
this generalises), [contracts/dialect-detection.md](contracts/dialect-detection.md),
[contracts/package-loading.md](contracts/package-loading.md),
[contracts/shared-utility-contracts-rust.md](contracts/shared-utility-contracts-rust.md)
(the #1621 boundary docs marking the seams this design removes).

## 0. The ruling, and the model in four layers

Issue #1631 rules that the current catalogue conflates two kinds of thing:
**dialects** — genuine core-language variants that change how the lexer,
parser, and analyser behave — and **loadable packages** — plain Tcl plus a
command surface. The redesign separates them, and — corrected by the
adversarial review — keeps four layers apart that revision 1 collapsed into
one resolved context:

1. A **dialect / core profile** is the language core: a *family* at a
   *release* under a *build/capability profile* (`tcl` at 8.4–9.1,
   `f5-irules`, `jim` at 0.76–0.84 × its configure matrix). Core profiles
   live in the compiled catalogue — itself generated from SpecTcl
   `dialect` sources (§6.2), per the owner directive that SpecTcl
   supports dialects and packages alike — and own every
   lexer/expr/numeral/escape/character-model axis. The build axis is not optional: the same Jim 0.84
   commit built default vs `--minimal` has a different character model,
   expr-function acceptance, and command surface (review B1), and unknown
   builds resolve to `Unknown`, never to a silently assumed default.
2. A **package** is a provider of versioned **surface declarations** —
   commands a provider *may* install, with version sets, predicates, and
   provenance (`Tk`, every tcllib module, the iApps/tmsh surfaces, Expect,
   the EDA vendor libraries). Packages are SpecTcl packs — bundled, user,
   or workspace. A surface declaration is catalogue evidence; it is never,
   by itself, proof that a command is bound in a given interpreter (review
   B2/B5).
3. An **environment** is a named, selectable *definition* of what a project
   works against: a core-profile selector plus per-axis **version-set
   targets** (a single release or a set such as tcl `8.5-9.1`), expected/
   ambient packages at platform-implied versions, server-side detection
   facts, policy defaults (closed-world, fixed ensembles, version ceiling),
   and a reference to a *fixed, contributed* editor language identity.
   `tcl8.6`, `f5-irules`, `f5-iapps`, `xilinx-eda-tcl`, and `tk` are all
   environment names. Environments are **dynamic data** — compiled-in for
   the core set, declarable by packs, and adjusted per workspace/user
   configuration through explicit **overlays** that never mutate the
   canonical definition — and they carry the alias table that keeps retired
   quasi-dialect names resolving.
4. An **analysis world** is the per-document semantic state the compiler,
   taint, side-effect, and codegen passes actually query: a graph of
   interpreter **realms** (parent, children, safe interpreters), each with
   temporal package state and command-binding knowledge
   (`Absent`/`Must`/`May`/`Unknown`), fed by the environment as a prior and
   by the existing transition vocabulary in
   `rust/tcl-registry/src/state_transition.rs`. Catalogue data says what a
   provider *can* mean; realm state says what a name *does* mean at this
   call site (review B2/B4/B5).

The only backwards compatibility maintained is (a) data-level: every name a
user can write today (configs, language ids, directives, CLI flags, pack
`-dialect` rows) keeps resolving through the environment alias table, and
(b) format-level: every published `speclib` 1.x pack keeps loading — with
the review-directed refinement that unknown *semantic* vocabulary in future
packs fails closed rather than degrading silently (§6.1). There are **no
Rust-side compatibility shims**: the tk triangle, `TK_PROFILE`,
`availability_for_name`'s union, `LanguageDialect::Set`,
`registry_for_dialect_profile`, and the retired `DialectSet` bits are
deleted, not wrapped.

## 0.1 Review disposition (revision 2)

Every blocking finding of the
[adversarial review](dialect-and-package-registry-redesign-adversarial-review.md)
is accepted. The review's contract sketches (`CoreProfileId`, `VersionSet` /
`ItemHistory`, `SurfaceDeclaration` / `BindingKnowledge`,
`EnvironmentDefinition` / `EnvironmentOverlay`, `AnalysisWorld` /
`RealmState`), its non-negotiable invariants I1–I10, and its safer phase
order are adopted as part of this proposal. Where each landed:

| Finding | Disposition | Where |
|---|---|---|
| B1 build/capability profile | accepted — core key is `(family, release, build)`; unknown builds are `Unknown` | §0, §3.1 |
| B2 per-interpreter temporal state | accepted — analysis world of realms; whole-file activation demoted to a labelled assistance heuristic | §0, §4, §5.3 |
| B3 `Lifecycle` ≠ requirement algebra | accepted — axis-typed `VersionSet` (normalised half-open unions) for targets/requirements; `ItemHistory` for per-item history; bound inclusivity explicit everywhere | §4, §5.4 |
| B4 specificity ≠ Tcl resolution | accepted — provider specificity is catalogue *authoring* precedence only; runtime binding comes from the shared resolver + realm transitions; ambiguity widens, never picks | §4 |
| B5 version ≠ surface | accepted — `SurfaceDeclaration` (candidates, predicated) vs `RealmBinding` (proved); `package provide` proves nothing about bindings | §3.2, §4 |
| B6 SpecTcl census gaps | accepted — external census `[STRUCT]` closure + shared `InvocationSpec` + `DynamicSurface` honesty are P5 prerequisites; equality gate becomes representation **and** behavioural parity | §6.2, §6.3, §8 |
| B7 editor identities are static | accepted — fixed contributed `EditorLanguageIdentity` set; dynamic server environments select among them, never mint new ids | §3.3, §7 |
| B8 leaked-static reload growth | accepted — registry generations (arena/`Arc`) are a P2 prerequisite; ~3.1 MB per generation of ~2,400 specs makes leak-per-reload untenable | §6.3, §8 |
| B9 workspace trust | accepted — provenance + trust classes on every fact; monotone security join; untrusted workspaces cannot weaken shipped analysis facts | §6.4 |
| B10 endpoint detectors unproven | accepted — correctness is defined as agreement over every selected target; the reference evaluates every distinct grammar/semantic profile in the set; detectors are per-pair optimisations proved against that oracle; `primary` is explicit | §5.4 |
| B11 Tk's `tk`/`Tk` loader semantics | accepted — canonical identities + predicated co-provides/loader aliases; Tk keeps its own version axis, never `tracks-base` by default | §3.2, §6.2 |
| B12 enforcement-location criterion | accepted — classification compares observable semantic fingerprints; families vs releases-on-a-ladder; sublanguages are registry descriptors; restriction/safety is policy | §2 |
| B13 unknown-word fail-open | accepted — vocabulary classified by compatibility effect; semantic unknowns quarantine or fail closed; unsupported major `speclib` fails closed | §6.1 |

H1–H5 are likewise adopted: reserved canonical names + namespaced
third-party ids + overlay identity (§3.3), the four-tier known-anywhere
model for W002 (§4), the Jim probe matrix keyed by
`(release, configure flags, platform, commit)` with lossless observations
(§3.1, §8), picol as a negative control the model must reject explicitly
rather than misdescribe (§2), and the assistance/semantics API split with
different names and types (§5.3).

## 1. Evidence base

The research establishing current state (agent sweeps over the workspace,
the jimtcl branch `claude/jimtcl-dialect-rust-5q48z8`, issues #1599, #1621,
#1626–#1628, #1631, #1643/#1644/#1650, and the corpus under `tmp/`) found:

**The grammar axis already draws the #1631 line.** Exactly five
`LexerGrammar` constants exist (`rust/tcl-dialect/src/profile.rs`), four of
which are plain-Tcl releases and the fifth is iRules. Every axis with a
genuine per-variant delta is already centralised in
`rust/tcl-dialect/src/grammar.rs`:

| Axis | Values today |
|---|---|
| `{*}` expansion (TIP 157) | off in 8.4 and iRules; on 8.5+ |
| iRules `}{` ghost separator | `f5-irules` only — a zero-width `SEP` token injected in `Lexer::parse_brace` (`rust/tcl-lexer/src/lexer.rs:1242`), which is why `if {1}{#true}` segments as three words on BIG-IP and warns everywhere else |
| `${…}` close rule | `FirstClose` (8.x) vs `Tcl9Nesting` (9.x) |
| Leading-BOM skip on `source` | 9.x only |
| `#` comments in `[expr]` (TIP 582) | 9.x only |
| Numeral grammar | `Tcl84` / `Tcl85` (`0b`/`0o`) / `Tcl90` (`0d`, `_` separators, leading-zero decimal) |
| Escape grammar (TIP 388, `TCL_UTF_MAX`) | `Tcl84` (=8.5) / `Tcl86` / `Tcl90` |
| expr word-operator lexemes | `eq`/`ne` ≥8.4, `in`/`ni` ≥8.5, `lt`/`le`/`gt`/`ge` ≥9.0, plus the nine iRules word operators (`contains`, `starts_with`, …) |

That inventory is a snapshot, not a ceiling. The jim branch has since
landed five further **lexical** axes, each measured against built
reference interpreters: `WordSeparators` (`\r` separates words under Jim,
`\v` does not), `BraceContinuation` (backslash-newline folding inside
braces — `proc p {a b\⏎c}` binds three parameters under Tcl and two under
Jim, measured by *calling* the proc because Jim's `info args` reports raw
specifiers), `QuoteTermination` (`"abc"def` is legal in Jim), `VarSyntax`
(`$(expr)` sugar as its own token kind, and `$name(idx)` paren nesting),
and `ListParse`. The design therefore treats `LexerGrammar` as an
**extensible per-(family, release) record**, not the frozen seven fields
— a new axis is a field plus its family values, never a new profile.

Real Tcl 9 changes that are *not* grammar — tilde-expansion removal,
`fconfigure -profile` (TIP 656), the TIP 745 mathfuncs — already live
entirely in registry command data, calibrating the rule from the other
side. iRules additionally owns a parse-level *structural* grammar (the
declaration-only top level: `IrulesExecutionContext`, IRULE5006/5007,
`Traits::IRULES_TOP_LEVEL_ONLY`), and its K36322151 bans make command-head
identity statically decidable (`rust/tcl-irules/src/when_block.rs:85-95`)
— both parse-level facts, both dialect-qualifying. `f5-iapps`, `f5-tmsh`,
`expect`, `spectcl`, `bpf`, `tk`, and all six EDA entries use a core
release's grammar **verbatim**; `f5-bigip` is not Tcl at all (its own
tokeniser and tree-sitter grammar; the profile's grammar field is inert).

**The wiring tax is real and the version ladder is on the wrong axis.**
Adding JimTcl on its branch cost 171 files, of which 36 are pure wiring;
nine near-identical `jim0.76`–`jim0.84` profiles exist *only* because a
profile carries exactly one resolved `LexerGrammar`; a parallel
`JimVersion` enum mirrors `TclVersion`; and 76 core commands (`set`, `if`,
`proc`, …) are re-authored by hand because the bare-vendor-bit design has
no inherit-then-override mechanism. Ten user-facing surfaces each grew ten
rows.

**The package machinery already exists end-to-end.** Per-document
`package require` extraction with version/exact/conditional facts, the
request-time floor resolution (`FloorSource::{Require, PackAmbient,
ProfilePin}`, max-then-closest precedence), workspace require inheritance
over the source graph, a `PackageResolver` that reads real `pkgIndex.tcl`
files, W120/W135/W139/W149 diagnostics, SpecTcl 1.2's arity windows and
`ambient_package`, and pack-claimed file extensions advertised to editors
(#1626). Tk is already half a package: 68 specs carry
`required_package: Some("Tk")` and every plain-Tcl profile pins Tk as a
hosted library. `CommandRegistry::ambient_packages`'s doc comment names
#1631 as its reason for existing.

**The ecosystem demands multi-version, multi-train packages.** One tcllib
2.0 checkout ships `struct 1.5` *and* `struct 2.2`, `struct::graph 1.2.2`
*and* 2.4.4, `struct::tree 1.2.3` *and* 2.1.3 — simultaneous major trains
selected by `package require` requirements. pkgIndex files gate on the
core with multi-requirement unions (`package vsatisfies [package provide
Tcl] 8.5 9`), so pack applicability depends on the dialect release — and
`package require Tcl 8.5` is statically a failure under a 9.x core.
tcllib's own `0compatibility` module stages deprecation D1 (wrappers) →
D2 (blockers) → D3 (removed). The `package vcompare`/`vsatisfies` algebra
is already ported and oracle-pinned in `rust/tcl-dialect/src/version.rs`.

**Eleven concrete blockers stand between today's format and tk/tcllib/
iapps as packs** (§6 addresses each): the `subcommand_forms` exclusion
(53 of 67 sites are Tk), no `-dialects` scoping or hosted/keyed spelling
for `ambient_package`, the closed-world vendor gate being derived from the
compiled catalogue only, seven ratified-but-unimplemented DSL words masked
by the `DraftOpaque`-hides-`LoaderGap` blind spot, hook-body cost on hot
paths, and scale (~358 modules to migrate).

## 2. The classification rule

Corrected per review B12 — the criterion compares **observable semantic
fingerprints**, never which compiler module happens to enforce a rule
(moving an allow-list check from an analyser pass into the parser must not
turn a package into a dialect):

> A **language family** is justified by observable *outer lexical/syntactic
> or core evaluation* differences from every other family — a
> `LexerGrammar`/expr/numeral/escape/character-model axis value no other
> family's ladder provides. A **release** is a target point on an admitted
> family's ladder, whether or not it changes grammar versus its neighbour
> (tcl 9.1 is a release on the `tcl` ladder, not a separate justification
> problem). A **command sublanguage** — a DSL living inside command
> arguments, like tcllib's `oo::dialect` definition bodies or pave's widget
> tuples — is registry descriptor data attached to an invocation, not a
> dialect. **Availability, safety, and closed-world restrictions are
> environment/realm policy**, not language identity: a safe interpreter
> hides `open` while still being Tcl 9.0.4. Everything else — any variant
> whose whole content is "commands, options, and versions" — is a package,
> and any user-selectable name for "a base plus packages" is an
> environment.

The rule is machine-checkable and becomes an invariant test in
`rust/tcl-dialect`: every family in the catalogue must own at least one
axis value distinct from every other family's ladder; every environment
must reference a core-profile selector and add **no** grammar knob (the
environment type simply has no grammar field, making the violation
unrepresentable rather than tested). Two calibration controls from the
review: a synthetic restriction moved between parser and analyser must not
change its classification, and picol — a mutable per-interpreter command
table over a tiny built-in set — is the negative control the model must
either represent honestly (embedder/build capabilities plus dynamic
bindings) or reject explicitly, never misdescribe with an invented
catalogue. Picol 2 (antirez's February 2026 rewrite) sharpens that
control from a second direction: the *same project name* now carries
different core-evaluation semantics — capital-initial variables are
global by name shape, replacing `global` entirely, and its new `expr`
performs no interpolation, so Tcl's recommended braced form cannot work —
proving that a bare name identifies neither grammar nor semantics across
its own releases, just as Jim's build matrix proves it within one
release. Both picol revisions belong in the oracle ledger as
negative-control columns.

Applying it to today's 18 catalogue entries plus the two off-catalogue
profiles and the jim branch:

| Today | Classification | Notes |
|---|---|---|
| `tcl8.4` … `tcl9.1` | dialect (family `tcl`, releases 8.4–9.1) | 9.1 has no grammar delta vs 9.0 but is a core release; releases are the family's version ladder, not separate catalogue entries |
| `f5-irules` | dialect (family `f5-irules`, 8.4-based) | qualifies on lexical/expr fingerprint alone: the `}{` ghost separator and nine expr word operators (and the declaration-only top-level file form). The K36322151 command bans, closed-world resolution, and the static-head-identity consequence are environment/realm **policy** riding on top (review B12) |
| `jim0.76`–`jim0.84` (branch) | dialect (family `jim`, releases 0.76–0.84) | measured grammar deltas per release (`NumberSyntax::Jim`/`Jim080`, `EscapeSyntax::Jim`, expr comments ≥0.81, special-float set; since extended with the five lexical axes and the expr precedence/operator/mathfunc/arity divergences — §1, §3.1) |
| `f5-iapps` | environment `f5-iapps` = tcl@8.5 + iapps pack (ambient, BIG-IP-keyed) + policy (fixed ensembles, W108 strict ASCII, no hosted tcllib) | grammar is `GRAMMAR_TCL85` verbatim; APL container routing is a language-id fact, not a dialect fact |
| `f5-tmsh` | environment `f5-tmsh` = tcl@8.5 + tmsh pack (ambient, BIG-IP-keyed) | no tmsh lexing mode exists (the `AGENTS.md` owner-map claim is stale); the `IAPPS\|TMSH` spec files split into two packs sharing sources |
| `tk` (off-catalogue) | package `Tk` + environment `tk` (alias: "wish") = tcl@base + Tk ambient | erases the tk triangle |
| `expect` | environment `expect` = tcl@8.6 + expect pack (ambient) | `expect`'s clause grammar is registry `CaseListSpec` descriptor data shared with `switch` |
| 6 × EDA (`synopsys-eda-tcl`, …) | pack-declared environments | already packages; their catalogue shells (identity, extensions, keyed tool pins) move into their packs |
| `spectcl` | environment (recommended) or dialect — **Q3** | grammar is `GRAMMAR_TCL9X` verbatim; the DSL words are a command surface (`rust/tcl-registry/src/commands/spectcl/`) |
| `bpf` | environment (recommended) or dialect — **Q3** | grammar is `GRAMMAR_TCL9X` verbatim; its real content is a codegen target and a command surface |
| `f5-bigip` | **neither** — a separate language surface | own tokeniser (`rust/tcl-bigip/src/conf_tokens.rs`), own tree-sitter grammar; keeps its identity/routing entry but leaves the Tcl dialect axis — **Q3** |
| future `sslictcl` (#1543) | dialect if it earns a grammar axis; otherwise environment | the classification rule decides at proposal time, not by precedent |

## 3. The three concepts

### 3.1 `Dialect` — family × release

```rust
// rust/tcl-dialect (illustrative shapes, not final signatures)
pub enum Family { Tcl, F5Irules, Jim /*, SslicTcl? */ }

pub struct Release(/* ordinal within the family's ladder */);

pub struct CoreProfileId {
    pub family: Family,
    pub release: Release,          // tcl: 8.4..9.1; jim: 0.76..0.84; irules: TMOS-keyed or single
    pub build: BuildProfileId,     // review B1 — semantic, not metadata
}

pub struct CoreProfile {
    pub grammar: LexerGrammar,     // resolved, total over admitted ids
    pub expr: ExprGrammar,         //   — full contract below
    pub character_model: CharacterModelId,
    pub capabilities: CapabilitySet, // typed; resolved centrally, in every fingerprint
}
```

**The build axis is load-bearing, not metadata** (review B1). The same Jim
0.84 commit built with `./configure` versus `./configure --minimal` has a
different character model (`é` is length 1 vs 2), different expr-function
acceptance (`sqrt(4)` evaluates vs "syntax error in expression"), and a
different command surface (`json::decode`, `tcl::prefix`, `zlib` present
vs absent) — compiled in or out *before any interpreter exists*, so no
`package require` model can recover it. Tcl's own history has the same
axis (`TCL_UTF_MAX` 3/4/6 builds, threaded vs unthreaded 8.x). Families
that are genuinely build-invariant declare one canonical build profile; a
named profile may inherit a measured default; and an **unknown build
resolves every unmeasured capability to `Unknown`**, never to the default.
The reference-interpreter probe matrix is keyed by
`(release, configure flags, platform, commit)` with stdout/stderr/exit
status recorded losslessly (review H3) — the jim branch's per-release
sweep becomes one *column family* of that matrix, not the family truth.

Per the owner directive, the dialect data itself is **SpecTcl-authorable**:
a `dialect` block (§6.2) declares a family, its release ladder with build
profiles, and per-release values for the closed axis vocabulary — Rust
owns the axes and their implementations; packs own the values. The
shipped cores compile from those sources at build time, so "compiled
catalogue" and "loadable dialect pack" are two backends of one
description, and adding jim 0.85 or a future family is data plus measured
probes, not new Rust — unless it needs an axis no family has needed
before.

**The `ExprGrammar` contract.** The word-operators/comments/numbers
triple is not enough for a non-Tcl family; Jim is the case that proves
the field list short. The full surface a family × release must own:

```rust
pub struct ExprGrammar {
    pub numbers: NumberSyntax,          // numeral grammar, incl. the special-float set
    pub comments: ExprCommentStyle,
    pub word_operators: &'static [WordOperator],       // eq/ne, in/ni, lt/le/gt/ge, contains, …

    /// Binding power per operator. NOT derivable from the operator set:
    /// Jim and Tcl share `eq`/`ne`/`lt`/`in` yet bind them at different
    /// levels, so two cores can accept the identical operator set and
    /// produce different parse trees.
    pub precedence: fn(&str) -> Option<(u8, u8)>,

    /// Symbolic (non-word) operators beyond the shared C-Tcl set,
    /// release-gated WITHIN the family: Jim's `<<<` / `>>>` 64-bit
    /// rotates (every modelled release) and `=*` / `=~` glob/regexp
    /// match (0.84 only).
    pub symbolic_operators: &'static [(&'static str, Release)],

    /// The mathfunc surface as a SET, not a floor. A floor model
    /// ("available since 8.5") cannot express a family that simply
    /// never had a function.
    pub mathfuncs: &'static [MathFunc],

    pub arity: ExprArity,               // Concatenating | ExactlyOne

    /// Whether `$var` / `[cmd]` interpolate INSIDE the expr engine.
    /// Invisible while every modelled family substitutes (tcl, jim,
    /// irules) — picol 2 proves the axis varies in the wild: its expr
    /// performs no interpolation, `expr $a+$b` works only via ordinary
    /// word substitution, and the braced form cannot work at all.
    pub substitution: ExprSubstitution,
}
```

Every value below is measured on `jimsh` built from the upstream tag
(0.76–0.84) against tclsh 8.6/9.0 — the jim branch's model doc §6 carries
the probes:

- **Precedence is a per-family fact, not a per-token fact.** C Tcl merges
  the comparison operators into two levels (`tclCompExpr.c`):
  `== != eq ne` at one, `< > <= >= lt le gt ge in ni` at the other. Jim
  splits the same operators across four-plus (`jim.c:9252-9285`, `OPRINIT`
  precedences, stable across every modelled release): `in ni` 55,
  `eq ne =* =~` 60, `== !=` 70, `lt gt le ge` 75, `< > <= >=` 80. So
  `expr {"a" eq "b" == 1}` parses as `("a" eq "b") == 1` under Tcl and as
  `"a" eq ("b" == 1)` under Jim. Today's `binary_bp` in
  `rust/tcl-syntax/src/expr/parser.rs` is a free function keyed on
  operator text alone with no dialect parameter — it gains the grammar as
  an argument, and the shared C-Tcl table becomes the `Family::Tcl` value
  rather than the hardcoded truth. (The 8.4→9.1 ladder never moved a
  precedence, which is exactly why the gap was invisible until Jim.)
- **Symbolic operators need lexer recognition, not just parsing.**
  `EXPR_WORD_OPERATORS` models word-shaped lexemes only; `<<<` must
  tokenise as one operator and `=~`/`=*` must not lex as `=` + junk, so
  the expr lexer's operator scanner reads the grammar's symbolic table
  the same way `word_operator_lexeme_at` reads the word table. `=*`/`=~`
  are semantically iRules' `matches_glob`/`matches_regex` at Jim's
  spelling and precedence 60 — same semantic operation, three different
  lexical homes, one registry-level identity.
- **Mathfunc membership is a per-core-profile set.** Jim ships 26
  functions (`jim.c:9294-9321`) and lacks five that C Tcl 8.5+ has:
  `entier`, `bool`, `min`, `max`, `isqrt` — `expr {min(1,2)}` errors in
  every modelled Jim release. Today's `TclVersion`-floor keying in
  `tcl-syntax/src/expr/mathfunc.rs` (`spec_tcl90`/`spec_tcl91`) would
  read those as "available since 8.5" and silently offer them under Jim.
  The set is resolved per core profile — and per **build**: Jim's math
  extension is a configure choice, and a `--minimal` build rejects
  `sqrt(4)` outright (§3.1, review B1) — with the call-target model
  (`FixedBuiltin` vs `CommandTable`) still varying by release within the
  tcl family.
- **`expr`'s own arity is dual-homed, both homes keyed by core profile.**
  Measured: `expr 1 + 2` yields 3 through Jim 0.80 and is
  `wrong # args: should be "expr expression"` from 0.81 (Jim's own take
  on TIP 526; C Tcl still concatenates in 9.1). The diagnostic rides the
  registry's `arity_windows` on the `expr` spec under provider
  `Core(jim)` — the core surface is a provider (§3.2), so core-keyed
  arity windows are already representable. The *parse* behaviour —
  whether a multi-word `expr` concatenates its words with spaces before
  parsing — is the `arity` field here, because the analyser needs it
  before any spec is resolved.

`RuntimeExprSurface` (today: release floor ∧ dialect-set intersection)
re-derives from `ExprGrammar` plus provider availability; nothing keeps
a second operator table.

What changes versus `DialectProfile`:

- **Grammar is a function of (family, release, build), not a catalogue
  row.** The
  nine jim profiles, and the five tcl release profiles, collapse into
  family release ladders. Adding jim 0.85 is one enum variant plus its
  measured axis values — no new profile literal, no editor row explosion
  (pickers render `family × releases` from data).
- **One version type.** `TclVersion` and the branch's `JimVersion` unify
  behind `Release` ordinals per family, with a family-aware comparator
  (fixing the branch's lexical `"0.76" >= since` string comparison, which
  breaks at `0.100`). The `package vcompare` port remains the one
  version-string algebra for *package* trains.
- **The dialect carries no command surface.** Core command surfaces attach
  to providers (§4); the dialect only decides grammar and which core
  provider ladder the environment's floor points into.
- iRules keeps its lexical/expr grammar here: the ghost-separator flag and
  the word operators are `Family::F5Irules` facts; the command bans and
  closed-world guarantee are environment/realm policy (§2).

### 3.2 `Package` — providers of surface declarations

A package is a named provider of **surface declarations**: commands the
provider *may* install, each with an axis-typed applicability
`VersionSet`, an optional capability/platform predicate, an invocation
spec, and provenance. Declarations are catalogue evidence for analysis —
never proof of a live binding, because Tcl loads packages by evaluating
`ifneeded` scripts that can inspect platform state, define only some
commands, select accelerator backends, or fail after partial mutation
(review B5; tcllib's `try`, `snit`, and `sha1` all do this in release
form). The declaration mechanism extends what already exists
(`Lifecycle`, `arity_windows`, versioned arg rows, versioned values,
W135/W139/W149). The redesign adds (§6):

- **Multi-train truth.** A command may carry several disjoint
  applicability sets (`struct::graph` 1.x and 2.x shapes coexist; the
  resolver picks the train `package require`'s requirement selects). The
  importer already derives windows from release snapshots; it gains "same
  name, parallel trains" awareness. Per review B3, applicability is a
  `VersionSet` (normalised half-open unions on a named axis), while an
  item's introduced/deprecated/retired history stays a separate
  `ItemHistory` — one declaration can have several applicability sets
  without pretending its history is one interval.
- **Dynamic-surface honesty.** A pack can declare that a provider's
  member set is runtime-extensible (`DynamicSurface`/`UnknownMembers`)
  instead of pretending closure — tcllib's `struct::tree` discovers its
  method set with `info commands`, `oo::dialect` manufactures definition
  DSLs, and pave installs computed methods on single objects at runtime
  (review B6).
- **Package identity is not a flat alias table.** Tk 9 registers
  lowercase `tk` as the loading package and provides uppercase `Tk` only
  through an `ifneeded` chain that requires the exact lowercase version —
  and only when built without `TK_NO_DEPRECATED` (review B11). The model
  therefore has canonical package identities plus **predicated
  co-provides and loader aliases** ("requiring `Tk` requires exact `tk`;
  successful load co-provides `Tk`, under this build predicate"), and Tk
  keeps its own version axis: compatibility with Tcl is a requirement
  relation, never `tracks-base`, unless a specific host environment truly
  guarantees matched versions.
- **Placement claims.** A pack can say how a package is present in an
  environment: `ambient` (just there, at a platform-implied or keyed
  version — iRules' F5 surface, Tk under wish), `hosted` (installable,
  version tracks or floats — Tk under tclsh, tcllib), or absent. Today
  only profile pins can say `hosted`/`Keyed`; packs can only say
  unconditional `ambient` — that asymmetry is exactly blockers 6–8.
- **Core surfaces are providers too.** `package require Tcl 8.5` is real
  Tcl; the core surface rides the same algebra as provider `tcl` (and
  provider `jim`, provider `f5-irules`) so one lifecycle/window mechanism
  gates everything from `lmap` (tcl ≥8.6) to `case` (tcl 8.4..8.6) to
  `lsubst` (jim ≥0.84) to `struct::graph::op` (package ≥0.11). Whether
  core surfaces are *authored* as SpecTcl or as native Rust is **Q1**;
  their availability model is unified regardless.
- **Packages take range targets exactly like cores.** A project can
  declare it supports `struct 1.5–2.2` or `Tk 8.5–9.0` and get the same
  across-the-range compatibility checking §5.4 defines for core
  releases — one mechanism, because packages and cores share the window
  algebra.

### 3.3 `Environment` — the selectable, aliasable identity

```rust
pub struct EnvironmentDefinition {
    pub id: EnvironmentId,             // canonical, reserved or namespaced — see the collision contract
    pub aliases: Vec<Arc<str>>,        // "irules", "tcl-irule", retired names, …
    pub display_name: Arc<str>,
    pub editor_identity: Option<EditorLanguageIdentityId>, // from the FIXED contributed set — review B7
    pub core: CoreProfileSelector,     // family + build profile + default release
    pub targets: VersionSet,           // per-axis target sets — §5.4
    pub expected_packages: Vec<PackagePlacement>, // package, version set (Pinned | Keyed | requirement), ambient/hosted
    pub policy_defaults: EnvironmentPolicy, // closed_world, fixed_ensembles, version_ceiling, strict_ascii, …
    pub server_detection: DetectionFacts, // file_extensions, filenames, content signatures, shebang words
    pub help_terms: Vec<Arc<str>>,
    pub provenance: Provenance,        // built-in | bundled pack | user | workspace — trust class, §6.4
}

pub struct EnvironmentOverlay {
    pub base: EnvironmentId,
    pub target_changes: TargetChanges,
    pub package_changes: PackageChanges,
    pub origin: ConfigurationOrigin,   // hash + origin are part of resolved identity
}
```

**Environments are dynamic — as definitions plus overlays, never mutation.**
Definitions come from: (1) the compiled core set (family ladders,
`f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`, `tk`, …); (2) pack-declared
environment blocks (§6.2); (3) workspace and user configuration — a
project can define `myproject-tool` = tcl@8.6 + packs X, Y ambient.
Workspace/user *adjustments* to a named environment (targets, expected
packages — including the derivation from a `tclpkg.tcl` manifest's
`tcl >=8.5 <9.1` and `require` rows) are `EnvironmentOverlay`s whose
content hash, origin, and trust class are part of the resolved identity —
the canonical definition is never redefined in place (review H1). The
collision contract: **all compiled canonical names are reserved** (not
only family names); third-party environments get namespaced stable ids
plus display names; alias cycles and same-precedence collisions are load
errors, not nearest-wins picks; and file-detection precedence is a
separate, explicit ladder that reports *ambiguity* rather than
lexicographic first-wins. Because environments change at runtime (config
edits, pack reloads), they are **not** interned `&'static` statics with
pointer-identity equality the way `DialectProfile` is today: the registry
holds `Arc` values, equality is by id plus content generation, and the
salsa layer keys on `(id, generation, overlay hash)` — the same
invalidation discipline the pack overlay key already implements
(`specPacksReloaded` → registry rebuild).

**Editor identity is split out** (review B7). VS Code and Zed language
ids, extensions, and filename patterns are extension-manifest
*contribution points*, fixed at build/install time — a server cannot mint
a new editor language id from a workspace pack. `EditorLanguageIdentity`
is therefore a fixed, generated, contributed set (today's ids: `tcl`,
`tcl84`…`tcl91`, `tcl-irule`, `tcl-iapp`, `tcl-bigip`, `tclspec`, …), and
dynamic server environments *select among* them — a new workspace
environment attaches its documents to a generic contributed Tcl identity
while the server tracks the real environment. A pack may request
detection patterns; the editor adapter reports whether it can apply them
dynamically (VS Code: workspace `files.associations` per #1626; JetBrains:
#1650; Zed/Sublime: static manifests only) — the design never promises a
new native file type where the host cannot register one.

- **Environments are the only user-facing names.** All six ingress kinds —
  `# tcl-dialect:` directives, `tclLsp.dialect` settings and
  `folderDialects`, LSP language ids (including the Emacs/Helix pattern of
  sending canonical names as language ids), `--dialect` CLI flags, MCP
  tool enums, pack `file_extension … -dialect` rows — resolve through
  **one** function, `Environment::resolve(name)`, replacing the four
  divergent validators that exist today (`available_dialects`,
  `is_known_dialect_name`, the directive's `KNOWN_DIALECTS` match, and
  `resolve_known` — which already disagree about `tk`).
- **The alias layer is this table.** `tk` → environment `tk` (tcl@base +
  Tk ambient); `f5-iapps` keeps its name as an environment; retired
  spellings are `aliases` rows. This is data, not a shim: nothing maps old
  APIs onto new ones — old *names* are first-class rows in the new model.
  Per-release names (`tcl8.4` … `tcl9.1`, `jim0.76` … ) are generated
  environment rows over the family ladders, so today's flat spellings keep
  working everywhere while pickers can group by family.
- **Packs can declare environments.** A SpecTcl pack may carry an
  `environment` block (§6.2). The six EDA catalogue shells move into
  `specs/eda_*.tclspec`; the compiled-in environment set shrinks to the
  core families' ladders plus `f5-irules`, `f5-iapps`, `f5-tmsh`,
  `expect`, `tk`, `spectcl`, `bpf` (per **Q2/Q3** rulings). Environment
  values are `Arc`-held dynamic data with generation-keyed identity — see
  the dynamism note above — unlike the loaded `CommandSpec`s, which stay
  leaked-static.
- **Policy absorbs the last profile stragglers**: `has_fixed_ensembles`,
  the iApps W108 strict-ASCII rule (today keyed on `vendor_bit ==
  IAPPS`), the version ceiling, and closed-world resolution (§5.3). The
  tcllib-excluded-from-iApps rule stops being a subtractive
  `DialectSet::all().difference(IRULES | IAPPS)` on the tcllib pack and
  becomes "the `f5-iapps` environment is closed over its ambient set".

## 4. Availability: catalogue declarations, version sets, and binding knowledge

`DialectSet` — the per-release bitmask doing double duty as version range
(`TCL85_PLUS`), vendor tag (`IRULES`), and library marker (`TK`) — is
retired. What replaces it is **two layers**, per review B2–B5, not one:
a declarative catalogue algebra used at every level (command, subcommand,
sub-subcommand, option, option value, side-effect, special variable,
form), and a realm-scoped binding-knowledge layer that semantic passes
query.

### 4.1 The catalogue layer — surface declarations and version sets

```rust
pub enum Provider {
    Core(Family),                  // the family's core surface
    Package(PackageId),            // "tk", "struct::graph", "iapps", "xilinx", …
}

pub struct VersionAxisId(/* interned typed axis */);

pub struct VersionSet {
    pub axis: VersionAxisId,
    pub ranges: Arc<[HalfOpenRange]>,  // normalised, disjoint; exact points where the
}                                      // comparator requires them

pub struct ItemHistory {               // one item's own story on one axis
    pub introduced: Option<Version>,
    pub deprecated: Option<Version>,
    pub retired: Option<Version>,
}

pub struct SurfaceDeclaration {
    pub provider: Provider,
    pub applicable: VersionSet,        // when this shape exists — parallel trains = several sets
    pub predicate: CapabilityPredicate, // build/platform/feature conditions (B1/B5)
    pub history: ItemHistory,          // deprecation metadata, fixes
    pub invocation: InvocationSpecId,
    pub provenance: Provenance,        // trust class — §6.4
}
```

Two version types, deliberately (review B3): `Lifecycle`/`ItemHistory`
answers "when was this one item introduced, deprecated, retired";
`VersionSet` answers requirement/target set algebra — Tcl requirements
are **alternatives of ranges with exclusive maxima** (`8.5-9.0` excludes
9.0; `8.5` alone excludes 9.0.4; `8.5 9.0-9.1` is a union that admits
it), so requirements and targets are normalised unions of half-open
ranges, never a single interval. Every set carries its axis: a Tcl core
`Release`, a package version, a BIG-IP release, and an ECharts release
are not comparable by accident, and the normaliser plus `contains`/
`intersect`/`subset` are differentially tested against real
`package vsatisfies` (invariant I2). Wherever this document or any UI
writes a range, bound inclusivity is explicit — "tcl 8.5–9.0
(inclusive)" and Tcl's own `8.5-9.0` (max-exclusive) are different sets
and must never be conflated in a settings value.

- **Core deltas become declarations, not bits**: `lmap` is
  `[{Core(tcl), 8.6-}]`; `case` is `[{Core(tcl), 8.4-9.0}]` (retired at
  9.0, exclusive); a command shared with Jim adds `{Core(jim), 0.76-}` to
  the same spec. Today's `TCL85_PLUS` masks translate mechanically. The
  jim branch's 76-command duplication becomes either multi-row
  availability on shared specs or narrow overriding specs (**Q6** decides
  the authoring shape).
- **Authoring precedence is not resolution.** The generalised
  most-specific-wins rule (narrowest provider set beats widest) decides
  only *which declaration a catalogue author intended to override* —
  pack tiers and specificity are authoring precedence. It never decides
  which command Tcl will call (review B4): that is the next layer's job.

### 4.2 The binding layer — realms and knowledge

```rust
pub enum BindingKnowledge {
    Absent,
    Must(InvocationSpecId),            // proved: this binding, here
    May(Arc<[InvocationSpecId]>),      // candidates; order/branch not proved
    Unknown,                           // dynamic loader, unknown interp target, …
}

pub struct AnalysisWorld {             // per document/compilation unit
    pub realms: RealmMap<InterpreterId, RealmState>,
}

pub struct RealmState {
    pub packages: PackageStateMap,     // unknown / available / loading / provided(version)
    pub command_bindings: CommandBindingMap,
    pub hidden_commands: HiddenCommandMap,   // safe-interp hide/expose
    pub namespace_state: NamespaceState,     // imports, aliases, renames
    pub policy: InterpreterPolicy,
}
```

Package state is **per interpreter and temporal** (review B2): Tcl keeps
the package table on the interpreter, `ifneeded`/`unknown` run arbitrary
scripts, a child interpreter inherits nothing, a safe child hides core
commands while providing the same `Tcl` version, and
`package provide Demo 1.0` survives `rename demo {}` — so a provided
version proves nothing about the live command table. The transitions that
update realm state — `package`, `source`, `proc`, `rename`,
`namespace import` (ordinary vs `-force` differ observably), `interp
alias`, `interp hide`/`expose`, child-interpreter operations — already
have a home: `rust/tcl-registry/src/state_transition.rs` (command
bindings, interpreter topology, policy, widening) integrates here rather
than being bypassed by a document-global floor. Dynamic operands widen
the affected domain to `May`/`Unknown`.

The consumer contract (invariants I3–I5): **no taint, side-effect,
lowering, or codegen hook is selected before its binding is proved**;
ambiguity takes the conservative union of effects or abstains, and never
picks a candidate by catalogue order or provider specificity. Load-order
permutations that change the real binding (two packages exporting one
name; `namespace import` vs `-force`) must change — or widen — the
resolved answer.

- **iRules' safety property is preserved and strengthened.** Today the
  bare `IRULES` mask guarantees no core spec leaks into iRules without a
  ban list. In the new model the `f5-irules` environment is closed-world
  *policy* over an explicit allow-list surface, and the realm layer is
  what makes it sound: `trace`/`interp`/`namespace` stay unknown because
  nothing provides them, and because iRules has no dynamic binding
  machinery the realm state stays `Must`-almost-everywhere — the static
  decidability it enjoys today, now derived rather than assumed.
- **Known-anywhere has four tiers, not one** (review H2): globally
  documented; installable/indexed for this project; expected from the
  selected environment; and must/may-active in this realm. W002 names the
  tier that supplied its candidate ("`button` is provided by package
  `tk` — indexed in this workspace but not required here"), replacing the
  hardcoded pack list in `all_dialect_command_names()`. A pack merely
  present on disk must not silently change typo diagnostics in unrelated
  environments. Security and compilation queries use realm bindings only;
  completion may opt into the broader tiers with annotations.
- Fast paths that today rely on bit tests (spec filtering, the zed query
  generator's `TK_AND_TCL` unions, `grammar_union`) re-derive from the
  declarations at registry build time; a small `FamilySet` bitset may be
  kept as an internal optimisation but is not part of the model.

## 5. Resolution: from bytes to environment, targets, and realms

### 5.1 Environment resolution (ingress)

The 6-tier detection chain survives with its data re-sourced: directive →
shebang (`wish` selects the `tk` environment; `tclsh8.5` selects
`tcl8.5`; `jimsh` selects `jim`) → tokenised `package require Tcl` guard →
content signatures → filename/extension (pack-declared extensions already
consulted first) → configured default. Detection facts live on
environments (compiled or pack-declared), which fixes the current
duplication between `TCL_SOURCE_EXTENSIONS`, profile `file_extensions`,
and editor manifests: the indexing/watcher/rename extension set, the
`workspaceContains` glob, and the editor language registrations all
generate from the one environment registry (plus live pack advertisements
via the existing `getEffectiveConfig` / `specPacksReloaded` channel from
#1626). The `# tcl-dialect:` directive accepts environment names and
aliases — making `# tcl-dialect: tk` (used in e2e tests today but rejected
by the server-side directive tier) finally coherent.

### 5.2 Activation and `package require` processing

Per document, in order:

1. **Environment** gives the core profile (family + release + build
   profile) and the expected placements (e.g. iRules: the F5 surface
   ambient at the BIG-IP-keyed version; `tk` environment: Tk ambient on
   its own axis; `xilinx-eda-tcl`: xilinx pack at the ToolVersion-keyed
   floor).
2. **Workspace facts** add hosted availability and floors: discovered
   packs (bundled/user/workspace tiers unchanged), `tclpkg.tcl` manifests
   (`require json 1.0.0`, `tcl >=8.6` — today zero-coupled to the spec
   system; the manifest's requires become workspace-level require facts),
   and `pkgIndex.tcl` files the `PackageResolver` already indexes
   (workspace-provided packages with no pack → "known, unspecced" →
   surface the `spec-author` workflow as the fix).
3. **Document facts**: the existing `package require` scan (name, version
   requirements, `-exact`, conditional flag) selects, per package, the
   best satisfying version train and floor using the ported
   `vsatisfies`/`vcompare` algebra — including multi-requirement unions
   (`8.5 9`) and same-major selection (`require struct::graph 1.2` picks
   the 1.x train even when 2.4.4 is present). Cross-file inheritance over
   the source graph continues to work as today.

**This chain feeds two differently-named APIs** (review B2/H5). The
*assistance view* — completion, hover annotations, W120 — may keep
whole-file activation as an explicitly labelled heuristic: after
`package require Foo` anywhere in the file, offering Foo's commands
everywhere is convenient. The *semantic view* — compiler, taint,
side-effects, codegen — is position-, path-, and realm-sensitive: a call
before the require, a require inside a conditional, or a require in a
child interpreter must not activate the surface at that program point,
and unknown control flow widens. The two views have different names and
types so a semantic pass cannot accidentally call the assistance
shortcut (invariant I3).

New diagnostics this enables (numbers illustrative, assigned at
implementation): requirement unsatisfiable by any known train of the
package; `package require Tcl 8.5` under a 9.x dialect (needs `8.5-` or
`8.5 9`); require of a package retired/renamed at the resolved floor;
require of a package no provider knows (with the spec-author code action);
requirement satisfied only by a train whose core constraint excludes the
current dialect release (the tcllib-2.0-under-8.4 case).

### 5.3 Strictness policy

Resolution visibility keeps today's lenient default — hosted packages
resolve, complete, and hover everywhere, with W120 ("add `package
require`") as the nudge and floors gating version-sensitive facts — but
the policy becomes an environment field:

- `open` (plain Tcl, EDA shells): hosted packs visible, W120 advisory.
- `closed` (`f5-irules`): only the ambient closure exists; hosted packs
  and `package require` are not part of the language (require itself is
  not an iRules command).
- `ambient-only-plus-require` (`f5-iapps`, `f5-tmsh` — recommended):
  ambient surface plus explicitly required packages; hosted-but-unrequired
  packs are excluded from resolution (this is today's subtractive tcllib
  exclusion, expressed positively). **Q7** confirms the default per
  environment.

### 5.4 Version-range targeting (multi-target projects)

The PyCharm-style feature: a project declares that it supports a *range*
of targets — `tcl 8.5–9.0`, and equally a *library* range such as
`struct 1.5–2.2` or `Tk 8.5–9.0` — and the analyser warns about anything
that is not valid, or does not mean the same thing, across the whole
declared range. This is a first-class mode of the §4 algebra, applied
uniformly to every provider: a target is an **interval (or set) per
provider**, core families and packages alike.

**Where targets come from** (intersected, most specific wins per
provider):

1. The environment's `targets` field (default: a single release — today's
   behaviour, and the feature is off for single targets).
2. Workspace/folder/user configuration (`tclLsp.targets`, e.g.
   `{ "tcl": "8.5-9.0", "Tk": "8.5-9.0", "struct": "1.5-2.2" }`).
3. `tclpkg.tcl` manifests. *Corrected by the centralisation audit*: the
   shipping manifest grammar accepts one operator + one version
   (`tcl >=8.5 <9.1` is rejected today, and the stored constraint is
   never evaluated), and the resolver is deliberately upper-bound-free
   MVS — so manifests become a target source only via the companion's
   ruling R6: a multi-clause range grammar for the `tcl` constraint plus
   a resolver-invisible `supports NAME RANGE` directive, with `require`
   staying a bare MVS floor.
4. The document's own `package require` facts: a requirement is already
   an interval under the `vsatisfies` algebra (`package require foo 1.2`
   means 1.2 ≤ v < 2; `8.5 9` is a union). Today floor resolution keeps
   only the lower bound (`requirement_lower_bound`); range targeting
   keeps the whole satisfiable set.

**Correctness is defined first, then optimised** (review B10).
Compatibility means: the relevant parse and semantic facts agree for
**every selected target** in the version set — not merely at its
endpoints. Targets can be non-contiguous sets, an axis can change and
change back across a ladder, and a grammar delta can alter *word
structure* (Jim's quote termination, brace continuation, variable
syntax), not just one token's value — so endpoint comparison is not a
proof. The **reference implementation** evaluates every distinct
grammar/semantic profile represented in the finite set (releases with
identical resolved profiles deduplicate — the ladders are small),
preserving a token-spanned parse per distinct grammar where structure
differs. Per-axis **detectors are optimisations for one profile pair**,
admitted only after differential corpus/fuzz testing proves them
equivalent to the reference for that pair; a synthetic A→B→A test axis
keeps endpoint-only shortcuts from regressing in. Assistance (completion,
hover, signature help) answers under an **explicit `primary`** target —
required for any multi-target project, defaulting to the newest selected
release but never silently (**Q15**): "maximum is usually a superset" is
a heuristic, not a contract. Compatibility checking answers over the
whole set:

- **Availability across the set**: a command, subcommand, option,
  option value, or arity window whose applicability `VersionSet` does
  not cover the target set gets a range diagnostic naming the failing
  targets — "`lmap` requires tcl 8.6; declared targets include 8.5",
  "`case` was removed in tcl 9.0; declared targets include 9.0",
  "`struct::graph` 2.x form used; declared targets include struct 1.2"
  (the W149 deferred-verdict and W139 straddle-hedge diagnostics are the
  single-floor seeds of this family). The check is `targets ⊆
  applicable` on the §4.1 set algebra — the declarations are already
  sets.
- **Grammar divergence across the range** (core providers): for each
  grammar axis whose value differs between the interval's endpoints, a
  detector flags constructs whose *meaning or validity* diverges:
  - numerals — the motivating example: `expr {010}` is 8 under tcl 8.x
    and not octal under 9.x; under targets 8.5–9.0 the leading-zero
    literal gets a warning with a fix-it to `0o10` (valid from 8.5) or a
    decimal rewrite when 8.4 is in range; `0d…` and `_` separators are
    9.x-only; `0b…`/`0o…` are 8.5+.
  - escapes — `\x` with more than two hex digits (meaning changes at
    8.6), `\U` (8.6+), astral `\U` (9.0+), the octal third-digit rule.
  - `${a{b}c}` — `FirstClose` vs `Tcl9Nesting` parse the same bytes to
    different variable names.
  - expr — `#` comments (9.x-only), `lt`/`le`/`gt`/`ge` (9.x operators,
    8.x bareword errors), `in`/`ni` (8.5+), `**` (8.5+).
  - words — `{*}` (8.5+), the leading-BOM rule.
  Implementation shape: the reference multi-profile evaluation above is
  the semantics; where an axis pair provably diverges only token-locally
  (numerals, escapes), a targeted post-lex detector replaces the second
  parse for that pair once the differential corpus/fuzz gate proves it
  equivalent. Axes that change word structure (Jim's brace continuation,
  quote termination, `$(…)`) keep the per-profile parse. The tclsh corpus
  (`tmp/tcl8.4.20` … `tmp/tcl9.1b0`), the built reference interpreters,
  and the differential fuzzer are the oracles.
- **Semantic divergence across the range**: differential constant folding
  at the interval endpoints (`const_fold_versioned` already exists per
  release; disagreement ⇒ warning), and the small table of runtime
  semantic switches (TIP 278 namespace fallback, string character model,
  byte-string encoding) flag constructs that touch a diverging semantic.
- **Package interplay**: every `package require` must be satisfiable at
  every core target (`package require Tcl 8.5` fails ≥9 targets —
  suggest `8.5-` or `8.5 9`; tcllib 2.0's own `vsatisfies 8.5 9` gate is
  checked against the core interval), and ambient placements must
  resolve at every target.

**Precedent and unification.** Today's unversioned fallback is the
degenerate form of this feature: `PLAIN_TCL`'s `ALL_TCL` mask with
`leading_zero_is_octal: Ternary::Inert` silently *abstains* where the
releases disagree, and `satisfies_any_ternary` already implements the
three-valued line-vs-requirement test (subset ⇒ Yes, disjoint ⇒ No,
overlap ⇒ Inert). Range targeting subsumes the fallback as "targets =
the family's full ladder, lenient mode", and upgrades abstention to an
actionable warning when the user has *declared* the range.

## 6. SpecTcl 2.0 (`speclib … 2.0`)

### 6.1 Compatibility contract (corrected by review B13)

**Reading older packs stays maximal; degrading newer packs fails closed.**
The loader keeps a single parser — every word ever ratified is readable
forever, and where 2.0 changes *meaning*, the change is expressed as a
**new word plus a translation of the legacy word**, never as per-version
dispatch:

- `dialects {…}` (1.x) keeps loading forever: its 1.x vocabulary
  (`tcl8.5+`, `all-tcl`, `tk`, `f5-iapps`, `irules`, …) is translated at
  load through the environment alias table into `available` rows. New
  packs use the 2.0 word; `tcl spec upgrade` rewrites mechanically.
- Every 1.x pack in the wild (the eight bundled EDA packs, user packs)
  loads to an identical surface under the 2.0 loader — extended
  `every_known_vocabulary_loads_the_same_command_surface` coverage plus a
  frozen 1.x corpus gate pin this.
- `VOCABULARY_VERSION` (the cache key) bumps once for 2.0 because
  translation output changes.

The **forward** direction — an older loader meeting newer vocabulary —
drops the revision-1 "warn and continue for everything" stance. An
unknown word that says "this argument is code", "this method is a sink",
or "this environment is closed-world" must not be discarded while the
rest of the spec loads: the old server would then issue *stronger*,
safer-looking results precisely because it ignored the field it did not
understand. Vocabulary is therefore classified by compatibility effect
(invariant I9):

- **presentation-only** unknowns (hover prose, display names, help
  terms): warn and drop, as today;
- **validation/assistance** unknowns (arity shapes, roles, value sets):
  quarantine the affected invocation spec — the command stays known, but
  the affected capability reports `Unknown` instead of a confident
  verdict;
- **security / control-flow / binding / lowering / codegen** unknowns:
  the affected command (or pack, for pack-level words) is excluded from
  strong analysis — no taint verdicts, no specialised lowering, no
  codegen hooks — and the degradation notice surfaces on the *source
  files that consume the degraded spec*, not only on the pack file;
- an unsupported **major** `speclib` version fails closed; a newer minor
  loads through declared feature/capability negotiation.

Each newly ratified semantic word ships with a downgrade fixture: an
old-loader test proving the word's absence yields abstention, never a
stronger claim.

### 6.2 New vocabulary (the additive core of 2.0)

| Word | Purpose |
|---|---|
| `available {PROVIDER WINDOW…}` on commands/subcommands/options/values | the §4 algebra: `available {tcl 8.6-} {jim 0.78-}` / `available {package Tk 8.5-8.6}`; replaces `dialects` + implicit `required_package` gating |
| `provides NAME VERSION ?VERSION…?` (pack level) | declares the package trains this pack describes, including parallel majors; commands default their provider to the pack's `provides` |
| `environment NAME { … }` (pack level) | declares an environment definition: `core tcl 8.5 ?-build PROFILE?`, `ambient PACKAGE VERSION\|tracks-base\|keyed KEY`, `hosted PACKAGE …`, `alias NAME…`, `editor_identity ID` (selecting from the **fixed contributed set** — review B7, never minting a new editor language id), `file_extension`/`filename`/`signature` server-side detection rows, `display_name`, `policy` knobs — subsumes and closes #1643 (`ambient_package -dialects`) by scoping placements to the declaring environment instead of flag-scoping a global claim |
| `placement` spellings: `ambient` / `hosted`, versions `Pinned` / `tracks-base` / `keyed KEY` / requirement sets | closes blockers 6–8: a pack can say "hosted, floored by requirement" (Tk under tclsh — on Tk's **own** axis, per review B11) and "ambient at the BIG-IP-implied version, in this environment only" (iapps); `tracks-base` survives only for hosts that genuinely guarantee matched versions; the closed-world vendor gate re-derives from *all* declared environments, compiled and pack-declared alike |
| `co_provides` / loader aliases (predicated) | corrected per review B11 — Tk 9 registers lowercase `tk` as the loading package and provides uppercase `Tk` via an `ifneeded` chain requiring the exact lowercase version, only when built without `TK_NO_DEPRECATED`. The spelling is a predicated relation ("requiring `Tk` requires exact `tk`; successful load co-provides `Tk`, under this build predicate"), not a flat alias; tcllib's D1 wrapper names ride the same mechanism |
| `dynamic_surface` / `unknown_members` | the honesty escape hatch (review B6): a provider whose member set is runtime-extensible (`struct::tree` methods via `info commands`, `oo::dialect` DSLs, pave's computed methods) declares so instead of pretending closure |
| `dialect NAME { … }` (pack level) — **owner directive: SpecTcl declares dialects, not only packages** | declares a language family or a release on one: `release R ?-build PROFILE?` rows building the ladder, and per-release **axis values from the closed, typed axis vocabulary** — `axis expand_syntax on`, `axis numbers jim080`, `axis braced_var first-close`, `axis escapes …`, `axis expr_comments …`, word-separator/brace-continuation/quote-termination/var-syntax/list-parse values, expr precedence table, symbolic-operator rows, mathfunc set, expr arity and substitution model, character model, capability predicates. A pack *sets values for axes Rust defines*; a new axis is still a Rust change (the lexer must implement it), so the closed vocabulary is the soundness boundary. Pack-declared dialects pass the §2 classification gate at load — a `dialect` block whose axis values equal an existing family's release is rejected with a notice naming the environment it should have been. Grammar declarations sit at the **top of the §6.4 trust lattice**: compiled family names are reserved, workspace-untrusted packs cannot alter any compiled dialect's axes, and a third-party dialect is namespaced like a third-party environment. This is also the vehicle for **Q1's endgame**: the shipped `tcl`/`f5-irules`/`jim` cores become SpecTcl `dialect` + surface sources compiled to Rust at build time (`tcl spec build --emit rust`), so the compiled catalogue and a loadable dialect pack are two backends of one description |
| invocation-refinement descriptor (name TBD at implementation) | the declarative replacement for `command_forms`/`subcommand_forms` (**Q12**): per-form word patterns, traits, mutator/query split, and effects as data — Tk's 53 sites are the migration test; until it lands, Tk cannot round-trip |
| the seven ratified-but-unimplemented words | `result_stability`, `event_requirement_form`, `data_collection`, `body_scope`, `side_switch_target`, `event_handler_priority`, `bpf_op` get loader implementations (prerequisite for any iRules surface pack-expression, and for closing the round-trip blind spot) |
| `include` / surface composition (**Q6**, optional) | `include from PROVIDER {names…}` with overrides — the alternative to jim-style duplication for family surfaces |

### 6.3 Structural fixes that ride along

- **Kill the `DraftOpaque`-masks-`LoaderGap` blind spot**: the round-trip
  gate gains a loader-side direction (synthetic packs exercising every
  documented word against the loader, not only renderer output), so a
  ratified word without a loader arm fails CI instead of silently
  dropping. The `object_class` incident and the seven words above are the
  motivating precedents.
- **Hooks in shipped packs**: performance-critical resolvers (17 Tk
  `script_timing_resolver`s sit on the semantic-tokens hot path; 28 µs
  Tcl-body vs 410 ns native) are kept native with stable IDs and
  referenced as `… -native ID` from the shipped packs — the sanctioned,
  round-trip-equal pattern, aligned with the #1372 hook-host direction.
  Community packs use Tcl bodies with `-inputs` shape-caching as today.
- **Cache honesty**: `LOADER_BUILD` stops being hand-maintained (derive
  from a build hash) before tens of thousands of tcllib lines depend on
  it.
- **Registry generations, not leaks** (review B8): the loader's
  leak-per-load discipline (documented in `loader.rs`) is untenable once
  the largest catalogues become reloadable packs — a `CommandSpec` is
  1,296 bytes and a full generation of ~2,400 specs is ~3.1 MB before
  nested slices, so a Spec Studio session editing a mass-migrated surface
  leaks hundreds of MB. Dynamic pack specs and all their nested data move
  into an arena/`Arc<RegistryGeneration>`; queries return
  generation-bound handles rather than public `&'static` references;
  dropping the last registry snapshot drops the generation; salsa keys
  carry the generation id. Immutable built-ins stay true statics. This is
  a **P2 prerequisite**, gated by a reload-1,000-times allocator test
  (invariant I7).
- **Shared `InvocationSpec`** (review B6): semantic properties common to
  free commands, ensemble arms, object methods, and deeper dispatch —
  taint sinks, forms, deprecation replacements, effects — move to a
  shared invocation capability model instead of being copied field by
  field into `SubCommand`. This is the structural fix for the external
  census's G7/G15 (method-level sinks and forms), and the prerequisite
  for honest specs of ticklecharts' method-level file-write sink and
  SpiceGenTcl's `runAndRead` process sinks.
- **The migration gate is representation *and* behaviour** (review B6):
  byte-compared registry dumps only prove the new form preserves what the
  old form said — not that either describes the upstream library. Each
  migration phase adds behavioural-parity fixtures (completion, hover,
  semantic token roles, arity, control flow, taint, side effects,
  deprecation, binding transitions) grounded in upstream source, and the
  external census's `[STRUCT]` gaps must be closed — or explicitly
  abstained from via `dynamic_surface` — before a library's migration is
  called complete (invariant I10).

### 6.4 Trust and provenance (workspace data is a security boundary)

Review B9: nearest-wins tier precedence is an *editing* model, not a
security lattice. A repository-controlled `.tcl-lsp/*.tclspec` can today
`-override` a shipped command; under this design it could otherwise also
weaken a taint sink, open a closed-world environment, or alter a hook —
precisely the facts that warn about that repository's own code. Every
declaration and resolved fact therefore carries provenance and a trust
class — at minimum: built-in, signed/bundled, user-trusted,
workspace-trusted, workspace-untrusted, live Studio override — and merges
are capability-specific (invariant I6):

- ordinary prose (hover, display names, docs) merges by authoring
  precedence, as today;
- **security facts merge monotonically**: untrusted data can add sinks
  and restrictions, never remove or weaken built-in taint, side-effect,
  safety, closed-world, or codegen facts;
- in an untrusted workspace (the editor's Workspace Trust state), pack
  additions may improve colouring, completion, and documentation; native
  or Tcl hook execution is disabled; and overriding a canonical
  environment or shipped command requires explicit trusted opt-in;
- diagnostics and hover expose the winning fact's provenance, so a
  trusted override is visible, not silent.

## 7. Rust surface changes (no shims)

What the research inventoried as the blast radius, stated as end-state
(the full site lists live in the research notes; counts are from the
sweeps):

- **`rust/tcl-dialect`**: `DialectSet` (bits, `parse`, `KNOWN_DIALECTS`,
  combinators, `TK_AND_TCL`), `DialectProfile`, `PLAIN_TCL`, `TK_PROFILE`,
  `resolve_known`, `availability_for_name`, `hosts_tk`, the per-name
  tables (`expr_grammar_base_version`, `TclVersion::from_profile`) are
  replaced by `Family`/`Release`/`CoreProfile`, `EnvironmentDefinition` +
  `EnvironmentOverlay`, `Environment::resolve`, and the axis-typed
  `VersionSet` algebra. The empty-string "no dialect stated ≠ plain tcl"
  behaviour pin from #1621 carries over as an optional environment
  handle.
- **`rust/tcl-registry`**: `CommandSpec.dialects: Option<DialectSet>` →
  surface declarations (`availability` rows with the same `None`-inherits
  nesting); `build_default`'s unconditional `tk_specs()` load and
  `load_dialect`'s exact-bit match are replaced by provider-driven
  registry assembly; `ProfileQueries` splits into assistance-view queries
  over (environment, floors) and semantic-view queries over realm
  `BindingKnowledge` (§4.2, integrating `state_transition.rs`);
  `all_dialect_command_names` re-sourced per the four-tier W002 ruling;
  detection tables move to environment data. The `commands/{tk,iapps,tcllib,expect}`
  native packs are deleted at their migration phases (§8), `commands/
  {tcl,stdlib,irules}` (and jim) remain per **Q1**.
- **`rust/tcl-spectcl` / `tcl-spec-studio`**: 2.0 vocabulary in loader,
  renderer, schema, draft, help, and coverage witnesses (the four-surface
  parity rule in `AGENTS.md` applies to every new field); `DIALECT_BITS`/
  `BIT_ONLY_LABELS` replaced by environment resolution; install's vendor
  gate re-derived from declared environments.
- **Compiler / lsp-core / server**: the analyser's `tk_dialect` flag and
  `tk_checks` activation become "provider `Tk` active"; the W108/iapps and
  fixed-ensemble gates read environment policy; `profile_for_dialect` and
  `registry_for_dialect_profile` (ruling B's documented hop) are deleted —
  the environment registry is the single ingress #1621 was approximating;
  `LanguageDialect::{Profile,Set}` collapses to an environment handle;
  the salsa string-keyed dialect inputs re-key on environment
  `(name, generation)` plus the resolved target spec (§3.3, §5.4).
- **Editors and codegen**: all ten generators iterate the environment
  registry; language ids and their `tcl-iapp`/`tcl-apl`-style spellings
  persist as environment fields, so *generated output changes minimally*
  where names survive; the hand-written Sublime `_SYNTAX_DIALECT_MAP`
  gains a generator or a drift gate; `callback-surfaces` row ids re-key on
  environment names (a one-time regeneration of ~1800 rows); the JetBrains
  dynamic-file-type work (#1650) targets pack-declared environment
  extensions.
- **CLIs / MCP**: `--dialect` and MCP enums list environments from the one
  registry (the `tcl registry-dump` "plain-Tcl only" predicate already
  matches the new shape); `tk_layout`'s `dialect: "tk"` default resolves
  through the alias table unchanged.
- **Gates**: `command-backing` re-keys on "provider `tcl` core at 9.0";
  `audit-option-dialects` unchanged in spirit; new gates: the
  classification invariant (§2), the single-resolver property test (every
  ingress accepts exactly the environment names + aliases), pack/native
  equality during each migration (§8), and the loader-direction round-trip
  (§6.3).

## 8. Migration plan (review-corrected order)

Each phase lands green on `rust` with `make rust-check` + smoke, deep
suites in CI; no phase leaves a consumer on a compatibility wrapper. The
order follows the review's correction: contracts and oracles before the
model, realm state and the range reference implementation before any
optimisation, durable SpecTcl foundations before any mass migration —
so the pack move cannot cement the wrong lifetime, trust, range, or
runtime-binding APIs.

These invariants hold from P0 onward and every phase cites the ones it
gates on (adopted verbatim from the review):

| ID | Invariant | Gate |
|---|---|---|
| I1 | Equal core-profile ids imply equal measured syntax/core semantics | cross-build and cross-release oracle matrix |
| I2 | Values from different version axes cannot be compared | type/compile-fail tests plus property tests |
| I3 | Package and binding facts are scoped to an interpreter realm and program point | parent/child/safe/ordering e2e suite |
| I4 | No taint/effect/lowering/codegen hook is selected before binding proof | ambiguity and dynamic-loader tests |
| I5 | Ambiguity widens effects or abstains; it never picks by catalogue order | load/import/rename permutation suite |
| I6 | Untrusted data cannot weaken trusted security facts | workspace-trust downgrade suite |
| I7 | Dropped registry generations release dynamic specs | 1,000-reload allocator test |
| I8 | Every advertised editor identity is actually contributed by that editor package | installed-extension manifest gate |
| I9 | Unknown semantic vocabulary fails closed | old-loader/new-pack downgrade fixtures |
| I10 | Pack migration preserves user-observable behaviour, not only serialised bytes | LSP/compiler/taint behavioural parity suite |

- **P0 — contracts and oracles.** This document (revision 2) ratified:
  the four-layer separation, the `VersionSet` algebra differentially
  tested against `package vsatisfies`, the trust policy, the
  binding-proof rule, the editor-identity boundary, the name/alias
  collision contract, and the immutable upstream oracle ledger (pinned
  revisions + build matrix, per review Appendix B / H3) — made concrete
  by the companion's §7 name-resolution oracle programme: reference
  interpreters built for all five releases, the Tk trees fetched, and
  the per-domain conformance-vector plan seeded from the C test suites
  and the stdlib's executable specifications. `AGENTS.md`
  owner-map corrections (§9); glossary entries.
- **P1 — core/environment model only.** `Family`/`Release`/`CoreProfile`
  (with build profiles), `EnvironmentDefinition`/`EnvironmentOverlay`,
  and central ingress land in `tcl-dialect`/`tcl-registry` with today's
  data expressed in the new model — existing native package specs stay in
  place. The four validators collapse to `Environment::resolve`. Editor
  catalogues regenerate (names unchanged ⇒ small diffs). The tk triangle,
  `TK_PROFILE`, and `LanguageDialect::Set` die here. Gates: I1, I2, I8.
- **P1a — realm state.** Integrate `state_transition.rs` with provider
  candidates: package transitions, safe interpreters, import/alias/rename
  effects, and the one shared name resolver produce `BindingKnowledge`;
  the assistance and semantic query APIs split. Gates: I3, I4, I5.
- **P1b — range targeting, reference first.** Typed `VersionSet` targets
  on contexts, the `targets ⊆ applicable` diagnostic family (core and
  package providers uniformly), and the **per-distinct-profile reference
  evaluator**; detector/parse optimisations (numerals first — the octal
  case) land only after the differential gate proves each pair against
  the reference. Ships behind the targets setting; single-target projects
  are unaffected.
- **P2 — durable SpecTcl foundation.** Registry generations (I7),
  trust-aware provenance (I6), the fail-closed vocabulary classes (I9),
  shared `InvocationSpec` capabilities, the loader-direction gate, 2.0
  words + legacy translation + `spec upgrade`, spec-studio parity, and
  closure or explicit `dynamic_surface` abstention for the external
  census's `[STRUCT]` gaps; `spec-author` skill refresh (its vocabulary
  section is already stale at 1.1).
- **P3 — Tk pilot.** Invocation-refinement descriptor first (Tk's 53
  `subcommand_forms` sites are its migration test), then
  `specs/tk.tclspec` generated from the native specs, gated on
  representation parity **and** behavioural parity (I10) — including
  Tcl/Tk version independence and the lowercase/uppercase loader
  semantics (B11) — then the native `commands/tk` deleted. The `tk`
  environment ships beside it. The Tk semantics epic (#1710) continues
  against the pack form.
- **P4 — smaller packages.** iapps/tmsh (splitting the shared
  `IAPPS|TMSH` sources into two packs + shared `values`/`descriptor`
  tables), expect, and the EDA environment shells move into their packs
  incrementally with the same behaviour and trust gates; the Rust
  catalogue shrinks to core.
- **P5 — tcllib by adversarial module.** Importer-driven from release
  snapshots (2.0 now; 1.17–1.21 as history — **Q9** decides depth and
  bundling), per-module packs mirroring tcllib's structure — **starting
  with the hostile shapes**: `struct::tree`, `struct::graph`,
  `fileutil::traverse`, and `oo::dialect`, scaling to the long tail only
  after those dynamic surfaces are honest. Multi-train cases (`struct`,
  `struct::graph`) are the version-set acceptance tests.
- **P6 — jim rebased.** The jim branch re-lands on the new model: its
  measured grammar data and probe scripts carry over as **release ×
  build-profile** columns (never one default-build column as the family
  truth — H3); its 134-file pack becomes either multi-row availability on
  shared core specs or a jim surface pack (**Q6**); its ten profiles and
  `JimVersion` disappear into the `jim` family ladder. (**Q10** may
  reorder P6 earlier if the branch should merge early.)
- **P7 — irules surface pack-expression (optional, deferred).** Requires
  the seven words + `event_requires` draft-model fix; the dialect
  (grammar, structure) and closed-world policy stay compiled regardless.
  **Q5**.
- **P8 — sweep.** Docs (the ~15 design/contract docs and ~10 KCS pages
  the inventory flagged, README dialect tables), samples,
  `docs/generated` regeneration, deletion of the last transitional data.

## 9. Defects found during research (fix regardless of this design)

1. **Salsa lexer-config truncation**: `LexerCfgKey` interns only
   `expand_syntax` + `irules_brace_separator` and rebuilds with
   `..LexerConfig::default()`, so 8.x documents lex `${a{b}c}` and
   `\x4142` under 9.0 close/escape rules on the memoised path
   (`rust/tcl-lsp-db/src/lib.rs:3017-3034`; `ProcBodyKey` likewise at
   `:1720-1750`; the doc comment above it is stale).
2. **Stale owner-map claim**: `AGENTS.md:74` "tmsh mode per dialect" — no
   such lexer mode exists.
3. **Contradictory prose on iRules' expr base**: `grammar.rs:85-88` and
   `expr_lexer.rs:318-319` say iRules has a `None` expr base; the
   catalogue sets `Some(V8_4)` (`profile.rs:623`). Behaviour is right,
   prose is wrong.
4. **Seven ratified DSL words silently unimplemented** (§6.2 list) —
   symptoms of the `DraftOpaque`-masks-`LoaderGap` blind spot (§6.3).
5. **`# tcl-dialect: tk` inconsistency**: used by `tk_dialect.rs` e2e
   tests and module docs, rejected by the server-side directive tier
   (`tk` is not in `KNOWN_DIALECTS`).
6. **Ungated hand-written map**: `editors/sublime-text/plugin.py`
   `_SYNTAX_DIALECT_MAP` (and its missing `tcl8.6`/`tcl9.1` rows).
7. **Stale doc counts**: `dialect-detection.md`'s 16-name list vs 18;
   `dialect-profile-model.md` §8 "16 catalog entries"; the `spec-author`
   skill's vocabulary list stopping at 1.1.
8. **Withdrawn** (originally: lexical version comparison in the jim
   branch's lifecycle gating). Incorrect — the branch's gating resolves
   through `Lifecycle::introduced_in` → `version::compare`, which walks
   numeric segments: `compare("0.100", "0.76")` is `Greater` and
   `meets_min("0.100", "0.76")` is true. Lexical and numeric orders
   merely coincide across 0.76–0.84, and the branch now pins the
   property at the two inputs where the orders diverge. The unified
   `Release` comparator (§3.1) remains a *unification* win — one
   ordering type instead of two parallel enums — not a bug fix.

## 10. Open questions for the owner

Recommendations marked ▸. Answers gate P0.

1. **Core surface source of truth.** *Direction ruled by the owner:
   SpecTcl must be extended to support dialects and packages* — the
   format carries `dialect` blocks (§6.2) setting values for the closed
   Rust-owned axis vocabulary, so the shipped `tcl`/`f5-irules`/`jim`
   cores become SpecTcl sources compiled to Rust at build time
   (`tcl spec build --emit rust`), with the compiled catalogue and a
   loadable dialect pack as two backends of one description. Remaining
   question is **sequencing only**: ▸ model first (P1) with native
   sources, land the `dialect` vocabulary in P2, convert the shipped
   cores to SpecTcl sources once the P3–P5 equality/behaviour gates have
   proven the pipeline — the availability algebra is identical
   throughout.
2. **Pack-declared environments.** Confirm the EDA shells (identity,
   extensions, signatures, keyed tool pins) move out of the compiled
   catalogue into `specs/eda_*.tclspec` environment blocks. ▸ Yes —
   it is the "fully centralised" end-state and #1626 built the editor
   channel it needs. (Also: is `Environment` the right name? The issue and
   #1628 both say "environment"; alternatives considered: platform,
   target, host.)
3. **Fate of the borderline three.** `spectcl` (▸ environment on tcl@9.0
   + the speclib surface pack — it has zero grammar delta; keeping the
   language id and file-type is environment identity), `bpf` (▸
   environment; its essence is a codegen target + surface, but say the
   word if BPF's restrictions should be parse-enforced like iRules', which
   would argue dialect), `f5-bigip` (▸ leaves the Tcl axis entirely; keeps
   its own language pipeline and detection identity).
4. **Selection UX.** Keep the flat per-release environment names
   (`tcl8.4`…`tcl9.1`, `jim0.82`) as the generated, stable spellings
   everywhere (▸), or move editors to a family + release two-control
   picker with the flat names as compat aliases?
5. **iRules surface pack-expression** (P7): in scope for this programme,
   or explicitly deferred? ▸ Deferred; the dialect stays compiled either
   way, and P7's prerequisites (seven words, `event_requires` draft model)
   are independent deliverables.
6. **Family surface composition.** For jim (76 shared commands today) and
   any future family: multi-row availability on shared core specs, narrow
   per-family override specs, or a 2.0 `include from` composition word?
   ▸ Multi-row availability on shared specs for identical behaviour +
   override specs where jim genuinely differs; add `include` only if
   authoring pain proves it out — it keeps the allow-list property without
   a new mechanism.
7. **Strictness defaults** (§5.3): confirm `open` for plain Tcl/EDA,
   `closed` for irules, `ambient-only-plus-require` for iapps/tmsh — and
   whether `open` should gain an opt-in strict mode for teams that want
   unrequired-package completion suppressed.
8. **Require position sensitivity.** *Settled by review B2/H5, narrowed
   to a UX question.* Whole-file activation survives only in the
   assistance view, explicitly labelled; the semantic view is position-,
   path-, and realm-sensitive (§5.2). Remaining question: should the
   assistance view also surface an ordering hint ("used before its
   `package require`") by default, or leave that to the semantic
   diagnostics? ▸ Surface it — the scan already records ranges.
9. **tcllib depth and shipping.** Which releases become windows (▸ 2.0
   authoritative + windows derived back to 1.17), per-module packs (▸ yes,
   135 modules mirroring upstream) — and are they *bundled* with the
   binaries like EDA, or an installable set with only a curated subset
   bundled (▸ bundled: the always-on tcllib surface is today's behaviour;
   size is an authoring-scale problem, not runtime)?
10. **jim branch sequencing.** Rebase `claude/jimtcl-dialect-rust-5q48z8`
    onto the new model before it merges (▸ — its wiring tax mostly
    evaporates and the measured data ports cleanly), or merge it first and
    migrate it in P6 with everything else?
11. **speclib numbering.** Confirm `2.0` + the new-word-plus-translation
    policy (no per-version dispatch, 1.x readable forever, one
    `VOCABULARY_VERSION` bump) — and that `dialects`/`ambient_package`
    become documented-legacy spellings rather than removed words. ▸ Yes.
    *Amended by review B13*: the forward direction adds the fail-closed
    vocabulary classes of §6.1 — an unsupported major fails closed, and
    semantic unknowns quarantine rather than warn-and-drop.
12. **Invocation refinement.** Green-light designing the declarative
    replacement for `command_forms`/`subcommand_forms` (whole-descriptor,
    all-or-nothing swap; Tk's 53 sites as the migration test), or prefer
    the closed-identity route (every native `CommandForm` member gets a
    stable ID referenced from packs)? ▸ The declarative descriptor:
    closed IDs keep the semantics in Rust and would make Tk's pack a
    facade.
13. **`DialectSet` residue.** Delete the type outright (▸) or keep a
    `FamilySet` bitset as an internal-only optimisation where profiling
    demands it?
14. **Keyed version UX.** `--bigip-version` / `--tool-version` (the
    `Keyed` axes) stay CLI/config-level knobs that set environment
    placement floors (▸), or become general per-package version overrides
    (`--package-version NAME=V`) now that packages are first-class?
15. **Primary target for a range/set.** *Narrowed by review B10*:
    `primary` is an explicit, required field for any multi-target
    project — "maximum is usually a superset" is a default, not a
    contract, and compatibility never depends on it (the reference
    evaluates every selected profile). Remaining question: what does the
    UI default `primary` to when the user declares targets without one?
    ▸ The newest selected release, stated visibly in the status UI.
16. **Range diagnostics shape.** New diagnostic family for range
    compatibility (▸ — a dedicated W15x-style block covering
    "introduced after range min", "removed before range max", and each
    grammar/semantic divergence detector, so users can tune them
    independently), or fold into the existing W135/W139/W149 version
    family?
17. **Range strictness and defaults.** When a range is declared, are
    range-compatibility findings warnings by default (▸), and should
    assistance *filtering* also go strict (completion only offers
    range-safe commands) or stay permissive with annotations (▸
    permissive: offer everything at the primary release, annotate
    "8.6+" the way version floors already annotate)?
18. **Dynamic environment scope.** *Superseded in part by review H1,
    which this revision adopts*: all compiled canonical names are
    reserved (not only family names); workspace/user adjustments are
    `EnvironmentOverlay`s that never redefine the base; third-party
    environments get namespaced stable ids; alias cycles and
    same-precedence collisions are errors. Remaining question: the
    namespacing scheme for third-party ids — pack-name-prefixed
    (`spicegentcl/ngspice`) vs reverse-DNS vs free-form-with-registry.
    ▸ Pack-name-prefixed: short, collision-free by construction, and
    legible in a status bar.
19. **Trust defaults and UX** (§6.4, review B9). Confirm the trust
    classes and the untrusted-workspace rules — and decide the opt-in
    surface: is trusting a workspace pack's overrides (a) the editor's
    Workspace Trust alone, (b) a per-pack tcl-lsp consent recorded in
    user config, or (c) both required for security-weakening overrides?
    ▸ (c): editor trust gates hook execution; per-pack consent gates
    security-fact overrides, with provenance always shown in hover.
20. **Build-profile scope for the Tcl family itself** (review B1). Jim
    gets the full build axis. Does the `tcl` family model historical
    build variance (`TCL_UTF_MAX` 3/4/6, threaded vs unthreaded 8.x) as
    build profiles, or declare one canonical build profile per release
    and treat deviant builds as out of scope? ▸ One canonical profile
    per release now, with the axis *representable* so a future
    `tcl-utf6` profile is data, not surgery — the 8.x UTF-6 builds still
    exist in EDA vendor tools.
21. **Realm-analysis depth for the first release** (review B2). Full
    `AnalysisWorld` realm tracking (child interps, safe interps,
    hide/expose) can land incrementally: confirm the initial scope —
    ▸ single-realm position-sensitive package/binding state first
    (already sound for the vast majority of scripts), with
    `interp create`/`interp eval` widening everything they touch to
    `Unknown` until the multi-realm map lands (P1a completes it).
22. **Stub fate** (centralisation audit, ruling R1). Confirm inline
    `# tcl-lsp: stub` and sidecar stubs ingest as provenance-tagged
    `SurfaceDeclaration`s (`Document`/`Workspace` trust) with the
    separate `StubOverlay` type retired — the authoring syntax is
    unchanged. ▸ Yes.
23. **The variable axis** (ruling R2). Confirm special variables become
    declarations authorable in SpecTcl `dialect`/package blocks
    (family/build-sensitive: Jim's `env`, picol 2's capital-initial
    globals), retiring `special_vars.rs`'s private dialect-name ingress
    and folding `dynamic_names` into realm variable-domain widening.
    ▸ Yes.
24. **`tclpkg.tcl` targets vs MVS** (ruling R6). The manifest's `tcl`
    constraint gains a multi-clause range grammar and a new
    resolver-invisible `supports NAME RANGE` directive declares analysis
    targets, while `require` stays a bare MVS floor and the three
    version comparators collapse onto the oracle-pinned algebra —
    confirm, or prefer giving the MVS resolver real upper bounds?
    ▸ The `supports` directive: it keeps the resolver's design intact
    and cleanly separates "what I install" from "what I claim to
    support".
25. **Hook `ctx` vocabulary** (ruling R5). Pack hook bodies read
    `dict get $ctx dialect`; 2.0 adds an `environment` key and keeps
    `dialect` as a documented legacy alias forever. ▸ Yes.
