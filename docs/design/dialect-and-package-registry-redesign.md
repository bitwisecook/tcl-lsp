# Dialects, packages, and environments — the registry redesign (issue #1631)

> **Status: PROPOSAL.** This is the design for the post-release architecture
> directed in issue #1631, researched on branch
> `claude/tcl-dialect-registry-design-lrzbsn` (2026-08-25). Nothing here is
> implemented. §10 lists the open questions whose answers gate the design;
> recommendations are marked. Where this document and
> [dialect-profile-model.md](dialect-profile-model.md) disagree, this document
> describes the *intended* model and that one describes the *shipping* model.

Companions: [spec-packs.md](spec-packs.md) (the SpecTcl format contract this
extends), [eda-library-packages.md](eda-library-packages.md) (the precedent
this generalises), [contracts/dialect-detection.md](contracts/dialect-detection.md),
[contracts/package-loading.md](contracts/package-loading.md),
[contracts/shared-utility-contracts-rust.md](contracts/shared-utility-contracts-rust.md)
(the #1621 boundary docs marking the seams this design removes).

## 0. The ruling, and the model in three sentences

Issue #1631 rules that the current catalogue conflates two kinds of thing:
**dialects** — genuine core-language variants that change how the lexer,
parser, and analyser behave — and **loadable packages** — plain Tcl plus a
command surface. The redesign separates them and adds the one concept the
split exposes as missing:

1. A **dialect** is a grammar core: a language *family* at a *release*
   (`tcl` at 8.4–9.1, `f5-irules`, `jim` at 0.76–0.84). Dialects live in the
   compiled catalogue and own every lexer/expr/numeral/escape axis.
2. A **package** is a versioned command surface (`Tk`, every tcllib module,
   the iApps/tmsh surfaces, Expect, the EDA vendor libraries). Packages are
   SpecTcl packs — bundled, user, or workspace — and own availability via
   lifecycle windows on their own version trains.
3. An **environment** is a named, selectable combination the user actually
   works in: a dialect plus a **release target set** (a single release or a
   range such as tcl 8.5–9.0), ambient packages at platform-implied
   versions, identity (display name, language id, file extensions,
   detection signatures), and policy (closed-world, fixed ensembles,
   version ceiling). `tcl8.6`, `f5-irules`, `f5-iapps`, `xilinx-eda-tcl`,
   and `tk` are all environment names. Environments are **dynamic data** —
   compiled-in for the core set, declarable by packs, and definable and
   overridable per workspace/user configuration — and carry the alias
   table that keeps retired quasi-dialect names resolving.

The only backwards compatibility maintained is (a) data-level: every name a
user can write today (configs, language ids, directives, CLI flags, pack
`-dialect` rows) keeps resolving through the environment alias table, and
(b) format-level: every published `speclib` 1.x pack keeps loading
unchanged. There are **no Rust-side compatibility shims**: the tk triangle,
`TK_PROFILE`, `availability_for_name`'s union, `LanguageDialect::Set`,
`registry_for_dialect_profile`, and the retired `DialectSet` bits are
deleted, not wrapped.

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

> A **catalogue (dialect) entry** is justified if and only if the variant
> owns a grammar or parse-level semantic delta: a `LexerGrammar` value, a
> numeral/escape/expr-grammar axis value, or a parse-level structural rule
> (declaration-only regions, command-use restriction enforced at parse
> level) that no plain-Tcl release provides. Everything else — any variant
> whose whole content is "commands, options, and versions" — is a package,
> and any user-selectable name for "a base plus packages" is an
> environment.

The rule is machine-checkable and becomes an invariant test in
`rust/tcl-dialect`: every dialect in the catalogue must either be a core
release of a family or own at least one axis value distinct from its
family baseline; every environment must reference a dialect and add **no**
grammar knob (the `Environment` type simply has no grammar field, making
the violation unrepresentable rather than tested).

Applying it to today's 18 catalogue entries plus the two off-catalogue
profiles and the jim branch:

| Today | Classification | Notes |
|---|---|---|
| `tcl8.4` … `tcl9.1` | dialect (family `tcl`, releases 8.4–9.1) | 9.1 has no grammar delta vs 9.0 but is a core release; releases are the family's version ladder, not separate catalogue entries |
| `f5-irules` | dialect (family `f5-irules`, 8.4-based) | `}{` separator, nine expr word operators, declaration-only top level, static head identity |
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

pub struct Dialect {
    pub family: Family,
    pub release: Release,          // tcl: 8.4..9.1; jim: 0.76..0.84; irules: TMOS-keyed or single
}

impl Family {
    pub const fn releases(self) -> &'static [Release];
    pub const fn grammar(self, r: Release) -> LexerGrammar;      // total function
    pub const fn expr_surface(self, r: Release) -> ExprGrammar;  // total function — full contract below
}
```

**The `ExprGrammar` contract.** The word-operators/comments/numbers
triple is not enough for a non-Tcl family; Jim is the case that proves
the field list short. The full surface a family × release must own:

```rust
pub struct ExprGrammar {
    pub numbers: NumberSyntax,          // numeral grammar, incl. the special-float set
    pub comments: ExprCommentStyle,
    pub word_operators: &'static [OperatorLexeme],     // in/ni, lt/le/gt/ge, contains, …
    pub symbolic_operators: &'static [OperatorLexeme], // family extensions beyond the shared
                                                       // C-Tcl set: Jim's <<< and >>> (all
                                                       // releases), =* and =~ (≥0.84)
    pub precedence: PrecedenceTable,    // binding power per operator, per family
    pub mathfuncs: MathFuncSurface,     // the known-function set + call-target model
    pub command_arity: ExprCommandArity, // Tcl: N args concatenated with spaces;
                                         // Jim ≥0.81: exactly one argument
}
```

- **Precedence is a per-family fact, not a per-token fact.** Jim splits
  what C Tcl merges into two levels into four distinct ones (`in`/`ni`
  at 55; `eq`/`ne`/`=*`/`=~` at 60; `==`/`!=` at 70;
  `lt`/`gt`/`le`/`ge` at 75). Today's `binary_bp` in
  `rust/tcl-syntax/src/expr/parser.rs` is a free function keyed on
  operator text alone with no dialect parameter — it gains the
  `ExprGrammar` (or its `PrecedenceTable`) as an argument, and the
  shared C-Tcl table becomes the `Family::Tcl` value rather than the
  hardcoded truth.
- **Symbolic operators need lexer recognition, not just parsing.**
  `EXPR_WORD_OPERATORS` models word-shaped lexemes only; `<<<` must
  tokenise as one operator and `=~`/`=*` must not lex as `=` + junk, so
  the expr lexer's operator scanner reads the grammar's symbolic table
  the same way `word_operator_lexeme_at` reads the word table.
- **Mathfunc surfaces are family-keyed.** Today
  `tcl-syntax/src/expr/mathfunc.rs` keys on `TclVersion`
  (`spec_tcl90`/`spec_tcl91`); the surface becomes
  `MathFuncSurface::for(family, release)` — Jim lacks `entier`, `bool`,
  `min`, `max`, and `isqrt`, and the call-target model
  (`FixedBuiltin` vs `CommandTable`) already varies by release within
  the tcl family.
- **`expr`'s own arity is dual-homed.** The diagnostic ("`expr` takes
  exactly one argument from Jim 0.81") rides the ordinary registry
  `arity_windows` on the `expr` `CommandSpec` under provider
  `Core(jim)`; the *parse* behaviour — whether a multi-word `expr`
  concatenates its words with spaces before parsing — is the
  `command_arity` field here, because the analyser needs it before any
  spec is resolved.

`RuntimeExprSurface` (today: release floor ∧ dialect-set intersection)
re-derives from `ExprGrammar` plus provider availability; nothing keeps
a second operator table.

What changes versus `DialectProfile`:

- **Grammar is a function of (family, release), not a catalogue row.** The
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
- iRules keeps its structural grammar here: the ghost-separator flag, the
  declaration-only top-level rules, and the static-head-identity guarantee
  are `Family::F5Irules` facts.

### 3.2 `Package` — versioned command surfaces

A package is a named provider with a version train (or several concurrent
major trains) whose command surface is expressed in SpecTcl and gated by
lifecycle windows — the mechanism that already exists (`Lifecycle`,
`arity_windows`, versioned arg rows, versioned values, W135/W139/W149).
The redesign adds (§6):

- **Multi-train truth.** A command may carry several disjoint provided
  windows (`struct::graph` 1.x and 2.x shapes coexist; the resolver picks
  the train `package require`'s requirement selects). The importer already
  derives windows from release snapshots; it gains "same name, parallel
  trains" awareness.
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
pub struct Environment {
    pub name: Arc<str>,                // "tcl8.6", "f5-irules", "f5-iapps", "xilinx-eda-tcl", "tk"
    pub aliases: Vec<Arc<str>>,        // "irules", "tcl-irule", retired names, …
    pub display_name: Arc<str>,
    pub editor_language_id: Option<Arc<str>>, // "tcl86", "tcl-irule", "tcl-iapp", …
    pub family: Family,
    pub targets: TargetSpec,           // Single(release) | Range(min, max) | Set(…) — §5.4
    pub ambient: Vec<PackagePlacement>, // package, version (Pinned | TracksBase | Keyed), ambient/hosted
    pub policy: EnvironmentPolicy,     // closed_world, fixed_ensembles, version_ceiling, strict_ascii, …
    pub detection: DetectionFacts,     // file_extensions, filenames, content signatures, shebang words
    pub help_terms: Vec<Arc<str>>,
}
```

**Environments are dynamic.** They come from four sources, nearest wins by
the same tier discipline the pack discovery already uses: (1) the compiled
core set (family ladders, `f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`,
`tk`, …); (2) pack-declared environment blocks (§6.2); (3) workspace and
user configuration — a project can define `myproject-tool` = tcl@8.6 +
packs X, Y ambient + its own extensions, or override a named environment's
targets and ambient set per folder; (4) implicit derivation from a
`tclpkg.tcl` manifest (`tcl >=8.5 <9.1` + its `require` rows). Because
environments change at runtime (config edits, pack reloads), they are
**not** interned `&'static` statics with pointer-identity equality the way
`DialectProfile` is today: the environment registry holds
`Arc<Environment>` values, equality is by name plus a content generation,
and the salsa layer keys on `(name, generation)` — the same invalidation
discipline the pack overlay key already implements
(`specPacksReloaded` → registry rebuild). Compiled and pack-declared
entries are constructed once per (re)load; config-declared entries rebuild
on `didChangeConfiguration`.

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

## 4. The availability algebra

`DialectSet` — the per-release bitmask doing double duty as version range
(`TCL85_PLUS`), vendor tag (`IRULES`), and library marker (`TK`) — is
retired. Availability becomes one algebra used at every level (command,
subcommand, sub-subcommand, option, option value, side-effect, special
variable, form):

```rust
pub enum Provider {
    Core(Family),                  // the family's core surface
    Package(&'static str),         // "Tk", "struct::graph", "iapps", "xilinx", …
}

pub struct Provided {
    pub provider: Provider,
    pub window: Lifecycle,         // introduced / deprecated / retired — the existing type
}

// on CommandSpec and every nested gate:
pub availability: &'static [Provided],   // empty ⇒ inherit from parent level
```

- **Resolution**: a spec is available in a resolved document context iff
  some `Provided` row's provider is *active* in the context and the
  context's floor for that provider falls inside the window. Context
  floors come from the environment (dialect release for `Core`, ambient
  placements for packages) composed with `package require` facts under the
  already-landed max-then-closest `FloorSource` precedence.
- **Two query modes**: the context carries an *interval* per provider,
  not just a floor (§5.4). Assistance queries answer at the primary
  release (floor ∈ window); compatibility queries answer across the
  declared range (window ⊇ interval), which is what makes PyCharm-style
  multi-target warnings a mode of the same data rather than a second
  registry.
- **Core deltas become windows, not bits**: `lmap` is
  `[{Core(tcl), 8.6..}]`; `case` is `[{Core(tcl), 8.4..8.6retired}]`; a
  command shared with Jim adds `{Core(jim), 0.76..}` to the same spec.
  Today's `TCL85_PLUS` masks translate mechanically. The jim branch's
  76-command duplication becomes either multi-row availability on shared
  specs or narrow overriding specs (**Q6** decides the authoring shape);
  either way the multimap-by-name registry and a generalised
  most-specific-wins rule (narrowest provider set beats widest, replacing
  today's fewest-bits tiebreak) keep resolution deterministic.
- **iRules' safety property is preserved and strengthened.** Today the
  bare `IRULES` mask guarantees no core spec leaks into iRules without a
  ban list. In the new model the `f5-irules` environment is closed-world:
  only providers in its ambient closure resolve, and the iRules surface is
  an explicit allow-list (`Core(F5Irules)` rows or the pack-expressed
  equivalent). `trace`/`interp`/`namespace` stay unknown because nothing
  provides them — same property, expressed as presence, not subtraction.
- **W002 ruling** ("exists, but not in this dialect"): the known-anywhere
  set is re-sourced from *all discoverable providers* — every family's
  core surface plus every bundled/user/workspace pack — instead of the
  hardcoded pack list in `all_dialect_command_names()`. The message
  upgrades: "`button` is provided by package `Tk` (not active in this
  `f5-irules` environment)"; the EDA/SpecTcl exclusion asymmetry
  disappears.
- Fast paths that today rely on bit tests (spec filtering, the zed query
  generator's `TK_AND_TCL` unions, `grammar_union`) re-derive from the
  provider rows at registry build time; a small `FamilySet` bitset may be
  kept as an internal optimisation but is not part of the model.

## 5. Resolution: from bytes to a resolved context

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

1. **Environment** gives the dialect (family + release) and the ambient
   placement floors (e.g. iRules: `iapps`-style F5 surface at the
   BIG-IP-keyed version; `tk` environment: Tk ambient tracking base;
   `xilinx-eda-tcl`: xilinx pack at the ToolVersion-keyed floor).
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
3. `tclpkg.tcl` manifests — `tcl >=8.5 <9.1` and `require json 1.0.0`
   rows are already interval declarations.
4. The document's own `package require` facts: a requirement is already
   an interval under the `vsatisfies` algebra (`package require foo 1.2`
   means 1.2 ≤ v < 2; `8.5 9` is a union). Today floor resolution keeps
   only the lower bound (`requirement_lower_bound`); range targeting
   keeps the whole satisfiable set.

**Two query modes on the same availability data.** Resolution and
assistance (completion, hover, signature help) answer under a designated
*primary* release per provider — recommended: the range maximum, because
the newest grammar and surface accept a superset on almost every axis
(**Q15**). Compatibility checking answers under the whole interval:

- **Availability across the range**: a command, subcommand, option,
  option value, or arity window whose `Provided` window does not cover
  the target interval gets a range diagnostic naming both ends —
  "`lmap` requires tcl 8.6; declared targets include 8.5",
  "`case` was removed in tcl 9.0; declared targets include 9.0",
  "`struct::graph` 2.x form used; declared targets include struct 1.2"
  (the W149 deferred-verdict and W139 straddle-hedge diagnostics are the
  single-floor seeds of this family). The check is literally window ⊇
  interval instead of floor ∈ window — the windows are already ranges.
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
  Implementation shape: the axes are a small closed set, so this is a
  targeted post-lex pass over tokens whose axis differs across the
  interval — not a full second lex. The tclsh corpus (`tmp/tcl8.4.20` …
  `tmp/tcl9.1b0`) and the differential fuzzer are the oracles that
  validate each detector.
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

### 6.1 Compatibility contract (unchanged in kind, restated)

The loader stays **version-blind with a single parser**: every word ever
ratified is readable forever; the `speclib` version word remains an
author's promise about which loaders can read the file, enforced by
notices (`Log::since`), never by gating. `VOCABULARY_VERSION` (the cache
key) bumps once for 2.0 because translation output changes. Where 2.0
changes *meaning*, the change is expressed as a **new word plus a
translation of the legacy word**, never as per-version dispatch:

- `dialects {…}` (1.x) keeps loading forever: its 1.x vocabulary
  (`tcl8.5+`, `all-tcl`, `tk`, `f5-iapps`, `irules`, …) is translated at
  load through the environment alias table into `available` rows. New
  packs use the 2.0 word; `tcl spec upgrade` rewrites mechanically.
- Every 1.x pack in the wild (the eight bundled EDA packs, user packs)
  loads to an identical surface under the 2.0 loader — extended
  `every_known_vocabulary_loads_the_same_command_surface` coverage plus a
  frozen 1.x corpus gate pin this.

### 6.2 New vocabulary (the additive core of 2.0)

| Word | Purpose |
|---|---|
| `available {PROVIDER WINDOW…}` on commands/subcommands/options/values | the §4 algebra: `available {tcl 8.6-} {jim 0.78-}` / `available {package Tk 8.5-8.6}`; replaces `dialects` + implicit `required_package` gating |
| `provides NAME VERSION ?VERSION…?` (pack level) | declares the package trains this pack describes, including parallel majors; commands default their provider to the pack's `provides` |
| `environment NAME { … }` (pack level) | declares an environment: `dialect tcl 8.5`, `ambient PACKAGE VERSION\|tracks-base\|keyed KEY`, `hosted PACKAGE …`, `alias NAME…`, `language_id ID`, `file_extension`/`filename`/`signature` detection rows, `display_name`, `policy` knobs — subsumes and closes #1643 (`ambient_package -dialects`) by scoping placements to the declaring environment instead of flag-scoping a global claim |
| `placement` spellings: `ambient` / `hosted`, versions `Pinned` / `tracks-base` / `keyed KEY` | closes blockers 6–8: a pack can finally say "hosted, tracks the base Tcl" (Tk under tclsh) and "ambient at the BIG-IP-implied version, in this environment only" (iapps); the closed-world vendor gate re-derives from *all* declared environments, compiled and pack-declared alike |
| `alias PACKAGE NAME` | package-name aliases (`Tk` vs Tk 9's lowercase `tk` — verify against the 9.0 corpus during implementation; tcllib's D1 wrapper names) |
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

## 7. Rust surface changes (no shims)

What the research inventoried as the blast radius, stated as end-state
(the full site lists live in the research notes; counts are from the
sweeps):

- **`rust/tcl-dialect`**: `DialectSet` (bits, `parse`, `KNOWN_DIALECTS`,
  combinators, `TK_AND_TCL`), `DialectProfile`, `PLAIN_TCL`, `TK_PROFILE`,
  `resolve_known`, `availability_for_name`, `hosts_tk`, the per-name
  tables (`expr_grammar_base_version`, `TclVersion::from_profile`) are
  replaced by `Family`/`Release`/`Dialect`, `Environment`,
  `Environment::resolve`, and the availability algebra. The empty-string
  "no dialect stated ≠ plain tcl" behaviour pin from #1621 carries over as
  `Option<&Environment>`.
- **`rust/tcl-registry`**: `CommandSpec.dialects: Option<DialectSet>` →
  `availability: &[Provided]` (with the same `None`-inherits nesting);
  `build_default`'s unconditional `tk_specs()` load and `load_dialect`'s
  exact-bit match are replaced by provider-driven registry assembly;
  `ProfileQueries` becomes `ContextQueries` over (environment, floors);
  `all_dialect_command_names` re-sourced per the W002 ruling; detection
  tables move to environment data. The `commands/{tk,iapps,tcllib,expect}`
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

## 8. Migration plan

Each phase lands green on `rust` with `make rust-check` + smoke, deep
suites in CI; no phase leaves a consumer on a compatibility wrapper.

- **P0 — contracts.** This document ratified (questions answered);
  classification rule + invariant test; `AGENTS.md` owner-map corrections
  (§9); glossary entries (dialect, package, environment, provider,
  placement).
- **P1 — the model.** `Family`/`Dialect`/`Environment` + availability
  algebra land in `tcl-dialect`/`tcl-registry` with today's data expressed
  in the new model (tk/iapps still native specs, now provider-gated); all
  consumers move in the same series — mechanical because #1621 already
  funnelled ingress to a handful of seams. The four validators collapse to
  `Environment::resolve`. Editor catalogues regenerate (names unchanged ⇒
  small diffs). The tk triangle, `TK_PROFILE`, and `LanguageDialect::Set`
  die here.
- **P1b — range targeting.** `TargetSpec` on contexts, the
  covering-interval query mode, the range-availability diagnostic family
  (core and package providers uniformly), and the first grammar-divergence
  detectors (numerals — the octal case — then escapes, `${…}`, expr
  axes), each validated against the tclsh corpus. Ships behind the
  targets setting; single-target projects are unaffected.
- **P2 — SpecTcl 2.0.** New words + legacy translation + loader-direction
  gate + `spec upgrade`; spec-studio parity; `spec-author` skill refresh
  (its vocabulary section is already stale at 1.1).
- **P3 — Tk as a pack.** Invocation-refinement descriptor first (Tk is
  its migration test), then `specs/tk.tclspec` generated from the native
  specs, equality-gated (registry dump old vs new, byte-compared), then
  the native `commands/tk` deleted. The `tk` environment ships beside it.
  The Tk semantics epic (#1710) continues against the pack form.
- **P4 — iapps/tmsh/expect as packs.** Split the shared `IAPPS|TMSH`
  sources into two packs + shared `values`/`descriptor` tables; expect
  moves whole; the EDA environment shells move into their packs (Rust
  catalogue shrinks to core).
- **P5 — tcllib as packs.** Importer-driven from release snapshots
  (2.0 now; 1.17–1.21 as history for windows — **Q9** decides depth and
  bundling), per-module packs mirroring tcllib's own module structure,
  equality-gated against the native specs, then `commands/tcllib`
  deleted. Multi-train cases (`struct`, `struct::graph`, `struct::tree`)
  are the acceptance tests.
- **P6 — jim rebased.** The jim branch re-lands on the new model: its
  measured grammar data and probe scripts carry over intact as family
  data; its 134-file pack becomes either multi-row availability on shared
  core specs or a jim surface pack (**Q6**); its ten profiles and
  `JimVersion` disappear into the `jim` family ladder. (**Q10** may
  reorder P6 before P3–P5 if the branch should merge early.)
- **P7 — irules surface pack-expression (optional, deferred).** Requires
  the seven words + `event_requires` draft-model fix; the dialect (grammar,
  structure, closed world) stays compiled regardless. **Q5**.
- **P8 — sweep.** Docs (the ~15 design/contract docs and ~10 KCS pages the
  inventory flagged, README dialect tables), samples, `docs/generated`
  regeneration, deletion of the last transitional data.

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

1. **Core surface source of truth.** Keep `commands/{tcl,stdlib,irules}`
   (and jim) as native Rust specs, or move their *sources* to SpecTcl with
   build-time AOT generation of the Rust (the direction
   [spec-packs.md](spec-packs.md) already states for compiled-in packs)?
   ▸ Model first (P1), then AOT the core as a later phase; the
   availability algebra is identical either way, and the equality gate
   built for P3–P5 de-risks the eventual core conversion.
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
8. **Require position sensitivity.** Keep whole-file activation with W120
   (▸), or model "command used before its `package require`" as a new
   ordering diagnostic (implementable later; the scan already records
   ranges)?
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
15. **Primary release for a range.** When targets are `tcl 8.5–9.0`,
    which release do parsing and assistance answer under? ▸ The range
    maximum (newest grammar accepts the superset on almost every axis;
    divergence detectors cover the rest) — but say the word if you want
    the minimum ("oldest-first" authoring) or an explicit
    `primary` field on the target spec.
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
18. **Dynamic environment scope.** Confirm the definition sources and
    their precedence — compiled < pack-declared < user config <
    workspace/folder config (nearest wins, matching pack-tier
    discipline) — and whether a workspace may *redefine* a compiled name
    (▸ no: workspace definitions may extend or override targets/ambient
    of a named environment and define new names, but core family names
    stay canonical so diagnostics keep meaning the same thing
    everywhere).
