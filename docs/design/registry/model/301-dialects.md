# Step 301: the dialect model

Status: designed 26 November 2026. Plan item 301, step 1 of
[dialects, libraries and environments](../dialects-and-libraries.md). Builds on
the registry split's item 264 (the dialect crates exist). [302](302-libraries.md)
and [303](303-environments.md) build on it; [304](304-f5-libraries.md) uses its
f5-tcl builds.

## Problem

`tcl-dialect` mixes three things: the languages, the BIG-IP side axis, and the
environment catalogue.

- **Four families.** `Family { Tcl, F5Tcl, F5Irules, Jim }`
  (`rust/tcl-dialect/src/model/family.rs:53`). `F5Irules` is a library posing
  as a language: it overrides no lexical axis (`GRAMMAR_IRULES =
  GRAMMAR_F5_TCL`, `family.rs:551`). Its two flags,
  `closed_load_time_resolution` and `declaration_only_top_level`
  (`family.rs:247`, `:260`), are rule-loader policy. `F5Irules` appears 190
  times in 86 files, 104 of them outside tests.
- **F5 has no release axis.** Each F5 family has one release, `tmos` and `tmm`
  (`family.rs:338`, `:342`). BIG-IP versions live on a separate keyed axis:
  `VersionKey::BigipVersion` (`library.rs:44`) and its copy
  `KeyedAxis::BigipVersion` (`model/environment.rs:155`). The default is
  16.1.0 and the declared data baseline is 15.0.0 (`library.rs:72`, `:91`).
- **Builds.** `BuildProfileId { Canonical, JimFull, JimMinimal, F5Scriptd32,
  Unknown }` (`family.rs:701`). `CapabilitySet` has three answers: UTF-8
  character model, math extension and 64-bit word size (`family.rs:760`).
  Jim's optional extensions (`binary`, `ensemble`, `json`, `tclprefix`,
  `zlib`) have no flag. `jim-own-surface.tclspec:26-29` states them
  unconditionally for that reason.
- **Jim builds cannot be selected.** The `jim` environment hard-codes
  `BuildProfileId::Canonical` (`model/environment.rs:1308-1320`), and nothing
  else names `JimFull` or `JimMinimal`.
- **`DialectProfile`'s `CATALOG`** has 19 rows (`profile/catalog.rs:193`): five
  Tcl releases, the four F5 rows, six EDA shells, `expect`, `spectcl`,
  `sslictcl` and `bpf`. `PLAIN_TCL` and `TK_PROFILE` sit beside it
  (`catalog.rs:1127`, `:1182`). Jim has no row. The five lexer grammars are
  written twice, in `catalog.rs:85-183` and `family.rs:478-570`, and a test
  pins the two equal.
- **The lexer reads a profile or a name.** `LexerConfig::for_profile` (242
  production calls), `for_dialect` (123) and `for_file_dialect` (16)
  resolve the grammar through a `DialectProfile` or an environment name.
  `grammar_of_dialect_name` falls back to `DialectProfile::find`
  (`grammar.rs:174`). `from_grammar` (437) and `for_file_grammar` (236)
  already take a grammar.

`DialectProfile` is named 2,207 times in 657 production files. Retiring the
catalogue does not mean editing all of them at once (see the work list).

## Design

### Families, releases and builds

```rust
// tcl-dialect::model
pub enum Family { Tcl, Jim, F5Tcl }            // names "tcl", "jim", "f5-tcl"

pub struct Release { family: Family, line: u16 } // index into the family's ladder
pub struct DialectPoint {
    pub release: Release,
    pub patch: Option<Version>,   // 9.0.4, 21.1.0.1; on the family's axis
    pub build: Build,
    pub capabilities: CapabilitySet,
}
pub enum Build { Canonical, JimFull, JimMinimal, Tmm, Scriptd, Unknown }
pub struct DialectSelector {        // what an environment names (303)
    pub family: Family,
    pub default_release: Release,
    pub releases: VersionSet,       // on VersionAxisId::core(family)
    pub build: Build,
    pub capabilities: CapabilityOverrides,
}
impl Family {
    pub const fn ladder(self) -> &'static [ReleaseLine];
    pub const fn builds(self) -> &'static [Build];
    pub fn axis(self) -> VersionAxisId;            // core(Tcl), core(Jim), big_ip()
}
impl DialectPoint {
    pub fn core_profile(&self) -> CoreProfile;     // grammar, expr, chars, caps
    pub fn lexer_grammar(&self) -> LexerGrammar;   // = core_profile().grammar
}
```

- **`tcl`**: 8.4, 8.5, 8.6, 9.0, 9.1, build `Canonical`. This is unchanged.
- **`jim`**: 0.76 to 0.84, builds `JimFull` and `JimMinimal`. `Canonical`
  stays as the release-keyed alias that resolves to minimal through 0.81 and
  full from 0.82 (`family.rs:831`).
- **`f5-tcl`**: the BIG-IP releases, builds `Tmm` and `Scriptd`.
  `F5Scriptd32` becomes `Scriptd`, and its 32-bit word size becomes a property
  of that build (`wordSize 4`, measurements §4).

`F5_FORK_POINT = "8.4.6"` and the `ancestry()` edges stay. `f5-tcl` → `tcl` at
8.4 is a fork, and `jim` → `tcl` at 8.6 is a reimplementation. The
`f5-tcl`-from-`F5Tcl` edge for iRules goes away with the variant.

### f5-tcl's release ladder: the BIG-IP releases with evidence

The ladder lists release lines (`major.minor`). A point may carry a patch on
the BIG-IP axis (`VersionAxisId::big_ip()`, `version_set.rs:124`), so
`21.1.0.1` is line 21.1 with patch 21.1.0.1. Ordering and set algebra use that
axis, never the Tcl axis. The list below covers the lines that some checked-in
evidence or data names. The generator (item 202) emits it from the evidence
manifest, with each line's provenance class.

| Line | Evidence class | Where |
|---|---|---|
| 21.1 | **appliance-measured** (21.1.0.1 build 0.0.26) | `bigip-irule-parser-measurements.md` header; `BigIpBuild::MEASURED_21_1_0_1` (`tcl-registry/src/f5/evidence.rs:49`) |
| 14.0–21.1 | tmsh `cli version` values accepted on 21.1.0.1 (11.5.0 and 11.6.0 refused) | `evidence/tcl/bigip-tmsh-cli-version-scope.md` |
| 17.5 | selected and loaded in that transcript | same |
| 9.0, 9.3, 9.4, 10.0, 10.2, 11.0, 11.3, 11.4, 11.5, 15.0, 16.1, 17.0, 21.1 | lifecycle keys in iRules command and event data | `commands/irules/*`, `events/` |
| 13.1, 15.1, 16.0, 16.1 | profile-default snapshots (13.1.0.8, 13.1.5, 15.1.0.0, 16.0.1, 16.1.3.2) | `tcl-registry/src/profile_defaults/` |
| 13.1, 17.1 | iApp `requires-bigip-version-min/max` tests | `tcl-registry/src/f5/iapp_metadata.rs:298` |
| 12.0 | iRulesLX introduction | `docs/references/command-spec/callback-surface-catalogue.json` |

So the ladder is 9.0, 9.3, 9.4, 10.0, 10.2, 11.0, 11.3, 11.4, 11.5, 11.6, 12.0,
12.1, 13.1, 14.0, 14.1, 15.0, 15.1, 16.0, 16.1, 17.0, 17.1, 17.5, 21.0, 21.1.
11.6, 12.1, 14.1 and 21.0 appear only in prose (the docs and the vendor
reference headings), and the generator keeps them only if a data row names
them. BIG-IP Next (`20.0.1` in the iRulesLX row) is a different product train
and stays off this ladder. Only 21.1 is appliance-measured, so only it carries
`EvaluationEvidence::MeasuredFork`. Every other line inherits the grammar and
carries `Inherited` evidence. Nothing measured shows a grammar change between
lines.

The conformance release matrix for f5-tcl is {15.0, 16.1, 17.1, 21.1}: the
baseline, the default, the last 17.x line the iApp data names, and the
measured line.

### Capability flags

```rust
pub struct CapabilitySet {
    pub utf8_character_model: CapabilityAnswer,
    pub math_extension: CapabilityAnswer,
    pub word_size_64: CapabilityAnswer,
    pub extensions: ExtensionAnswers,   // per flag: Yes / No / Unknown
}
pub enum Extension { Binary, Ensemble, Json, Tclprefix, Zlib, Ssl, Ipv6 }
pub struct CapabilityOverrides(SmallVec<[(Extension, bool); 4]>);
```

A build gives the defaults: `JimFull` has every extension, and `JimMinimal`
has none (`auto.def:26`). An environment's `CapabilityOverrides` can switch
one on or off, which covers a `--minimal --with-ext=json` build. A library or
command row may require a capability (302's `CapabilityPredicate`), so Jim's
`json::*` rows say `-requires json` instead of being unconditional.

### What leaves the family

| Fact | Today | Goes to |
|---|---|---|
| `irules_brace_separator`, `brace_line_continuation` | the `F5Tcl` and `F5Irules` grammars | the f5-tcl family grammar, for **every** build (see Corrections) |
| `closed_load_time_resolution`, `declaration_only_top_level` | `Family::F5Irules` methods | the iRules library's policy (304) |
| BIG-IP version | `VersionKey`/`KeyedAxis::BigipVersion` | f5-tcl's release axis; 303 removes both enums |
| `Release::F5_TCL_TMOS`, `F5_IRULES_TMM` (46 production sites) | single-release ladders | f5-tcl lines; `tmm` becomes a build |

### Retiring `CATALOG`

Each of `DialectProfile`'s 25 fields (`profile.rs:88-272`) goes to the
dialect point, the environment (303), a library (302) or is derived.

| Field | Goes to |
|---|---|
| `core_point` | `ResolvedEnvironment::point()` (a `DialectPoint`) |
| `name`, `aliases`, `display_name`, `short_name` | environment id, the one alias table, presentation (303) |
| `editor_language_id`, `filenames`, `file_extensions` | environment editor identity and detection rows (303) |
| `vendor_surface` | derived: the environment's ambient non-bundled libraries (302) |
| `surface_packages` | environment placements with `ambient` (303) |
| `base_layers` | the libraries the environment places, plus the dialect's core surface |
| `grammar_union` | derived for static grammars: the union of placed libraries' names over the environment's release set |
| `version_ceiling` | the selector's `releases` upper bound (a `VersionSet`) |
| `signature_base`, `runtime_base`, `expr_grammar_base` | `DialectPoint`: for f5-tcl the fork anchor 8.4 (via `ancestry`), else the release |
| `leading_zero_is_octal` | `CoreProfile::grammar.numbers` (already derivable) |
| `grammar` | `DialectPoint::lexer_grammar()` |
| `operators_as_commands` | the iRules library's policy (304); the f5-tcl core surface for scriptd keeps them |
| `tcloo` | derived: TclOO visible at the point (302's worked example) |
| `has_fixed_ensembles` | environment policy `fixed_ensembles` (already there, `environment.rs:216`) |
| `vm_runtime_version` | `DialectPoint` (the engine reads the point) |
| `evaluation_evidence` | per release line, from the evidence manifest |
| `libraries` (`LibraryPin`) | environment placements (302/303) |
| `help_terms` | environment presentation (already there) |

`DialectProfile` survives only as a projection for the migration (see the work
list). `CATALOG`, `PLAIN_TCL`, `TK_PROFILE`, the duplicate `GRAMMAR_*`
constants in `catalog.rs`, and `DialectProfile::find` are deleted.

### The lexer from the dialect alone

```rust
impl LexerConfig {
    pub fn for_point(point: &DialectPoint) -> Self;      // nested re-lex
    pub fn for_file_point(point: &DialectPoint) -> Self; // leading BOM rule
}
```

The grammar is a total function of (family, release line, build), with no
name lookup. `grammar_of_dialect_name` stops consulting `DialectProfile` and
resolves the name to an environment, then to the point. `for_profile` and
`for_dialect` become thin wrappers during the migration and are then removed.
Dynamic families (`model/dynamic.rs`) keep their own grammar through
`dynamic_core_grammar`. A selector over a range lexes under each distinct
grammar in the range (W151 today), so an f5-tcl range has one grammar.

### Dependencies and consumers

`tcl-dialect` keeps its leaf position: core types, `bitflags`, and no registry.
Consumers are `tcl-lexer` (the point only), `tcl-syntax`, the registry core,
the compiler, the engines (`vm_runtime_version` → the point), and 303's
environment model.

### Decisions

1. **f5-tcl's axis is the BIG-IP version axis, with a generated ladder of
   lines.** The data already keys iRules facts on patch-level versions
   (9.4.8, 13.1.0.8), so an ordinal-only release would lose them. Lines give
   the matrix and the pickers a finite set.
2. **The separator is a family axis, not a build axis.** The evidence says all
   three contexts agree (Corrections).
3. **`Canonical` stays for Jim** as a release-keyed alias, because the default
   build flipped at 0.82. A user who writes `jim` with no build gets what
   `./configure && make` gave at that release.
4. **`DialectProfile` is projected, not deleted, in the first cut.** 657 files
   cannot move in one step. The projection is built from (environment, point),
   never from a catalogue row.

### Open questions

- **Is the tmsh `cli script` interpreter `scriptd`?** It differs from the iApp
  context in `tcl_platform` (empty), `tcl_patchLevel` (unset) and `info
  vartype` (measurements §4a). Settled by a wordSize probe through `exec` in a
  cli script. If it differs, add a `Tmsh` build. Until then tmsh maps to
  `Scriptd`, and the difference is the tmsh library's ambient surface.
- **Do any f5-tcl lines differ in the core surface?** Only 21.1 is measured.
  Settled by running the §4 feature matrix on one 15.x or 16.x appliance.

## Work list

1. `family.rs`: drop `F5Irules` and `IRULES_LADDER`. `F5Tcl` keeps its
   existing spelling `"f5-tcl"` (owner decision, 26 November 2026). Add `Build`, `Extension`
   and `CapabilityOverrides`. Move `closed_load_time_resolution` and
   `declaration_only_top_level` to a `tcl-lib-irules` policy type (304).
2. `release_facts` (item 202): generate `F5_LINES` with provenance from the
   evidence manifest, and add a `--check` gate.
3. `point.rs`: `DialectPoint` gains `patch` and `capabilities`, and
   `lexer_grammar()`.
4. `tcl-lexer/src/lexer/config.rs`: add `for_point` and `for_file_point`, and
   reimplement `for_profile` and `for_dialect` over them.
5. `profile.rs`: rebuild `DialectProfile` as a projection of
   (`ResolvedEnvironment`, `DialectPoint`). Delete `CATALOG`, `PLAIN_TCL`,
   `TK_PROFILE` and `catalog.rs`'s `GRAMMAR_*`.
6. `jim-own-surface.tclspec`: add `-requires` to the extension rows (302
   vocabulary).
7. Migrate `DialectProfile` readers crate by crate, lexer first. Each crate's
   PR removes its `for_profile` calls.

## Verification

- **Done when** (tightening the plan's): `Family` has three variants. `rg
  'F5Irules' rust` matches only the SpecTcl 1.x/2.x shims. An environment
  selects `jim` `JimMinimal` with `json` on, and f5-tcl `Scriptd`. `rg
  'for_profile|for_dialect' rust/tcl-lexer` finds nothing, and
  `LexerConfig` takes a `DialectPoint`. `catalog.rs` is gone.
- **Lexer conformance:** the existing lexical suites
  (`tcl-lexer/tests/it_2/f5_lexical_conformance.rs`,
  `it_4/lexer_depth.rs`) pass unchanged with the f5-tcl family grammar under
  both builds.
- **Measurements:** the build cone of `tcl-dialect`, which should be
  unchanged, with 43 dependents (item 295). The rebuild count after a ladder
  edit. Lexing time under `jim` and `tcl8.6` on the corpus in
  `spectcl-compiled-surface-plan.md`, within ±2%.

## Risks

- **Patch-level versions on a ladder.** Mitigation: comparison goes through
  the BIG-IP `VersionSet` axis only, and a line is just an enumeration aid.
- **The `DialectProfile` projection lingers.** Mitigation: an only-shrinks
  count of `DialectProfile` references in the `registry-axes` ledger.

## Corrections to the overview

- **The brace separator is not a `tmm` property.** The overview and plan item
  301 say `irules_brace_separator` "belongs to `tmm`". The measurements show
  the R-rules and N-rules identical in TMM, `TmshCliScript` and
  `IAppImplementation` on all 34 parity cases
  (`bigip-irule-parser-measurements.md` §4a), and today's code puts both
  axes on the trunk grammar (`family.rs:541`). They belong to the f5-tcl
  family. Suggested plan text: "the `irules_brace_separator` and
  `brace_line_continuation` axes belong to the f5-tcl family, under both
  builds; the rule-loader flags move to the iRules library".
- **`authored_surface.rs`, `environment.rs` and `version_set.rs` are in
  `tcl-dialect/src/model/`.** They are not in `tcl-registry` or at the crate
  root.
- **The F5 release list is mostly unmeasured.** Only 21.1.0.1 is measured.
  Every other line is a data key or a tmsh-accepted version.
