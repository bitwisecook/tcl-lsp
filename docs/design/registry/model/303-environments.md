# Step 303: environments as data

Status: designed 26 November 2026. Plan item 303, step 3 of
[dialects, libraries and environments](../dialects-and-libraries.md). Builds on
[301](301-dialects.md) (dialect selectors) and [302](302-libraries.md)
(libraries and placements). [304](304-f5-libraries.md) and
[308](308-positioned-context.md) build on it.

## Problem

Environments already exist as data (`EnvironmentDefinition`,
`tcl-dialect/src/model/environment.rs:448`), but three other tables still
carry the same facts.

- **`DialectProfile`'s `CATALOG`** (19 rows, `profile/catalog.rs:193`), plus
  `PLAIN_TCL` and `TK_PROFILE`. `ingress.rs` leaks a `DialectProfile` per
  resolved environment (`tcl-registry/src/model/ingress.rs:957`) so that old
  consumers keep reading profile fields.
- **The compiled environments** are hand-written functions in
  `model/environment.rs` (`compiled_definitions`, `:1609`): `tcl`, the five
  ladder rows, `tk`, `jim`, `f5-irules`, `f5-iapps`, `f5-tmsh`, `f5-bigip`,
  `expect`, `spectcl`, `sslictcl` and `bpf`. The six EDA environments are
  generated from `specs/*.tclspec` into `model/bundled_environments.rs` by
  `cargo xtask gen-bundled-environments`, with `--check`.
- **The surface list** is kept again in `SOURCE_AUTHORING_ENVIRONMENT_SURFACES`
  (`model/authored_surface.rs:727`, 15 rows), in the registry's
  `shared_group!` cells and in `registry/lookup.rs`. Item 262 inverts the
  last two.
- **Two keyed-axis enums.** `VersionKey` (`library.rs:44`, 88 references) and
  `KeyedAxis` (`model/environment.rs:155`, 75 references) have the same four
  variants: `BigipVersion`, `ToolVersion`, `SdcVersion` and `UpfVersion`.
  `LibraryPin` (65 references) duplicates `PackagePlacement`.

### Alias spellings kept more than once (production code)

| Spelling | `CATALOG` (`catalog.rs`) | compiled environments (`environment.rs`) | generated EDA (`bundled_environments.rs`) | editor identities (`EditorLanguageIdentityId::CONTRIBUTED`, `environment.rs:94`) |
|---|---|---|---|---|
| `irules`, `tcl-irule` → `f5-irules` | `:464` | `:1336` | | `tcl-irule` again as identity |
| `tcl-spec`, `tclspec` → `spectcl` | `:764` | `:1521` | | both again as identities |
| `sslic-tcl`, `tls-sslictcl` → `sslictcl` | `:805` | `:1550` | | |
| `genus`, `innovus` → `cadence-eda-tcl` | `:231` | | `:35` | |
| `questa`, `modelsim` → `mentor-eda-tcl` | `:642` | | `:94` | |
| `libero` | `:701` | | `:141` | |
| `quartus` | `:556` | | `:186` | |
| `dc_shell`, `primetime` | `:843` | | `:257` | |
| `vivado` | `:1078` | | `:324` | |
| `wish` → `tk` | **absent** (`TK_PROFILE` has none, `:1185`) | `:1259` | | |
| `jimsh`, `jimtcl` → `jim` | **absent** (no Jim row) | `:1310` | | |

The copies disagree in two places, `wish` and `jim`, and agree only by
repetition elsewhere. Test files repeat the spellings as well
(`environment_ingress.rs:427`, `ingress.rs:636`, `irules_context.rs:632`,
`formatting/config.rs:491`, `unused_procs.rs:482`), all inside test modules.

The library versions per Tcl release are hand-written in
`ladder_environments` (`environment.rs:1107`).

## Design

### An environment is a selector plus placements

```rust
pub struct EnvironmentDefinition {          // tcl-dialect::model (kept, reshaped)
    pub id: EnvironmentId,
    pub presentation: Presentation,         // display_name, short_name, kind, help_terms
    pub editor: EditorIdentity,             // identity + selecting identities (fixed contributed set)
    pub dialect: Option<DialectSelector>,   // 301; None only for f5-bigip
    pub libraries: Vec<PlacedLibrary>,      // 302: id, range, Ambient | Hosted | Bundled
    pub policy: EnvironmentPolicy,          // world, fixed_ensembles, strict_ascii
    pub detection: DetectionFacts,          // unchanged
    pub provenance: Provenance,
}
pub struct AliasTable { rows: Vec<(Arc<str>, EnvironmentId, Provenance)> } // the one table
pub struct ResolvedEnvironment {
    pub definition: Arc<EnvironmentDefinition>,
    pub point: DialectPoint,                // after overrides
    pub targets: TargetSet,                 // per axis: core family and libraries
    pub libraries: LibraryWorld,            // 302's resolution without document requires
}
impl ResolvedEnvironment {
    pub fn has_library(&self, id: &str) -> bool;
    pub fn build(&self) -> Option<Build>;
    pub fn library_versions(&self, id: &str) -> Option<&VersionSet>;
}
```

- `version_ceiling` is now the upper bound of `DialectSelector::releases`.
- `targets` was the environment's own `VersionSet`. It becomes the
  selector's release set, plus each placed library's range.
- `expected_packages`/`PackagePlacement` become `libraries`/`PlacedLibrary`.
  `Placement::{Pinned, TracksBase, Keyed, Requirement}` collapse into a
  `VersionSet` range plus `Bundled`. A pin is a one-point range, `TracksBase`
  is `Bundled`, `Keyed(BigipVersion)` is a tracks-host library, and
  `Keyed(Tool/Sdc/Upf)` is the EDA library's own axis with an open range.

### The one alias table

Aliases leave the definitions, and the `AliasTable` holds every
spelling-to-environment row. It is built in this order: the compiled rows in
`tcl-catalogue`, then the generated EDA rows, then pack-declared rows (by
tier). A collision follows today's rules (`reserved_against`,
`environment.rs:1663`): a compiled spelling cannot be claimed by a pack.
Editor identities stay a separate, fixed set, because they are contributed by
the editors and are not aliases. A spelling that is both (`tcl-irule`,
`tclspec`) appears once, as an identity, and the resolver tries identities
after aliases, as today's four-tier index does (`environment.rs:900`).

Old spellings from 301 join the table: `f5-tcl` → `f5tcl`'s default
environment (`f5-tmsh`). The `f5-irules` id is unchanged.

### The compiled-in environments in `tcl-catalogue`

`tcl-catalogue` (registry split item 264) holds the compiled definitions
as data. Hand-written function bodies become one const table per crate
contribution:

| Environment | Dialect selector | Libraries |
|---|---|---|
| `tcl` (lenient) | tcl 8.4–9.1, default 9.0 | Tk, Thread hosted (open ranges); bundled per release |
| `tcl8.4` … `tcl9.1` | tcl, one line | bundled libraries at the generated versions; Tk, Itcl, Thread hosted |
| `tk` (`wish`) | tcl 8.4–9.1, default 8.6 | Tk ambient; Itcl hosted |
| `jim` (`jimsh`, `jimtcl`) | jim 0.76–0.84, `Canonical` | none |
| `jim-minimal` (new) | jim 0.82–0.84, `JimMinimal` | none |
| `f5-irules` | f5tcl lines 15.0–21.1, default 16.1, `Tmm` | iRules ambient (tracks host) |
| `f5-iapps` | f5tcl, `Scriptd` | iApps and tmsh ambient |
| `f5-tmsh` | f5tcl, `Scriptd` (301 open question) | tmsh ambient |
| `f5-bigip` | none | BIG-IP schema (tracks the BIG-IP axis) |
| `expect` | tcl 8.6 | Expect 5.45.4 ambient |
| `spectcl`, `sslictcl`, `bpf` | tcl 9.0 | their DSL library ambient, closed world |
| six EDA shells | generated from `specs/` | the vendor library ambient on its own axis |

Bundled library placements on the ladder environments are generated (302's
`bundled_libraries`) and never written by hand.

The four surface lists become one. `SOURCE_AUTHORING_ENVIRONMENT_SURFACES` is
derived from the catalogue (an environment's ambient libraries plus its
dialect surface). `shared_group!` and `lookup.rs` are gone after item 262.

### Detection is unchanged

The chain from the redesign's §5.1 is kept, in this order: directive,
shebang, `package require Tcl` guard, content signatures, filename and
extension, then the configured default. It reads `DetectionFacts` exactly as
today. The result is an `EnvironmentId`, and resolution then applies
overrides to produce a `ResolvedEnvironment`. No detection row changes, which
the verification below pins.

### `tclLsp.targets` as ranges

`tclLsp.targets` and `# tcl-lsp: supports NAME RANGE` stay as they are
(redesign §5.4, `targets_from_clauses`). The one change is where they land.
`NAME` resolves through the catalogue: a family name (`tcl`, `jim`, `f5tcl`)
or a library id or alias (`Tk`, `itcl`, `iRules`). The range intersects the
selector's release set or the library's placed range. A range on a library
the environment does not place is an error notice, not a silent drop.
`--bigip-version` and `tclLsp.bigipVersion` become a target on the f5tcl
axis.

### Removing `VersionKey` and `KeyedAxis`

| Variant | Becomes |
|---|---|
| `BigipVersion` | f5tcl's release axis (301). iRules, iApps, tmsh and the schema track the host |
| `ToolVersion` | each EDA library's own axis (`vivado`, `quartus`, …) |
| `SdcVersion` | the `sdc` library's own axis |
| `UpfVersion` | the `upf` library's own axis |

`default_version` (16.1.0) becomes the `f5-irules` selector's
`default_release`. `baseline_version` (15.0.0) becomes the lower bound of its
release set. `LibraryVersionOverrides` becomes per-axis targets.
`LibraryPin`, `LibraryVersion`, `VersionKey`, `KeyedAxis` and
`BundledVersion::Keyed` are deleted.

### Dependencies and consumers

The types stay in `tcl-dialect::model` (pure data). The resolver over
libraries is in the registry core, and the compiled rows are in
`tcl-catalogue`. Consumers: the ingress (`tcl-registry/src/model/ingress.rs`),
the LSP's dialect settings (`tcl-lsp-server/src/dialects.rs`), the editors'
generators (`gen-editor-dialects`, `gen-editor-extensions`), the CLI's
`--dialect`, MCP, and Spec Studio's environment picker.

### Decisions

1. **The environment id is the selection key, and aliases are rows.** The two
   disagreeing copies (`wish`, `jim`) prove that per-definition alias lists
   drift.
2. **Editor identities stay separate.** They are a fixed set contributed by
   the editors, and a server cannot mint one.
3. **A `jim-minimal` environment, not a setting.** Detection and editor
   identity are per environment, and a build change alters the lexer and the
   surface.
4. **f5tcl's default release is 16.1, and its baseline is 15.0.** These keep
   today's answers.

### Open questions

- **Should `f5-tmsh` default to `Scriptd`?** Today it uses `Canonical`
  (`environment.rs:1433`) while `f5-iapps` uses `F5Scriptd32`. This is settled
  by 301's tmsh wordSize probe.
- **Generated EDA aliases versus pack aliases.** Should a bundled pack be
  able to change its own alias at all? Settled by the owner. Today the
  generated table makes such a change a build edit.

## Work list

1. `tcl-dialect::model::environment`: reshape the definition as above, add
   `AliasTable`, and move aliases out of the definitions.
2. `tcl-catalogue`: a const table of compiled environments, with the bundled
   placements generated from 302's table.
3. `ingress.rs`: resolve through the one table and build a
   `ResolvedEnvironment`. Keep `DialectProfile` as a projection until 301's
   migration ends.
4. Delete `CATALOG`, `PLAIN_TCL` and `TK_PROFILE` (with 301),
   `SOURCE_AUTHORING_ENVIRONMENT_SURFACES`, `VersionKey`, `KeyedAxis`,
   `LibraryPin` and `LibraryVersionOverrides`.
5. Re-point `gen-bundled-environments`, `gen-editor-dialects` and
   `gen-editor-extensions` at the catalogue, with `--check`.
6. Settings: `tclLsp.targets` names resolve through the catalogue, and
   `bigipVersion` maps to an f5tcl target.

## Verification

- **Done when:** every name that resolves today resolves to the same
  environment id through the one table. A test enumerates the union of
  today's ids, aliases, editor identities and pack-declared names, and
  compares each against a snapshot taken before the change. `CATALOG`, the
  per-definition alias lists, `VersionKey` and `KeyedAxis` are gone (`rg
  'VersionKey|KeyedAxis|CATALOG' rust` finds nothing outside the 1.x/2.x
  shims). The detection conformance rows (directive, shebang, extension,
  filename and signature fixtures in `tcl-registry/src/model/ingress.rs`'s
  tests and the server's `dialects.rs` tests) pass unchanged.
- **Editors:** `cargo xtask gen-editor-dialects --check` and
  `gen-editor-extensions --check` produce byte-identical output.
- **Measurements:** ingress time per document, which should be unchanged
  (one hash probe). The build cone of an environment edit, which should be
  `tcl-catalogue` and the binaries only.

## Risks

- **Settings compatibility.** `f5-tcl` and the `bigipVersion` setting.
  Mitigation: aliases and a settings migration notice.
- **Pack-declared environments.** A 2.x pack's `environment` block must
  translate to placed libraries. 305's shim covers it, and the
  `environment_effect_snapshot` gate in `upgrade.rs` is the check.

## Corrections to the overview

- The overview says the alias lists are "kept twice". They are kept three
  times in production (catalogue, compiled environments, generated EDA rows),
  plus the editor identity set, and two of the copies disagree (`wish`, the
  Jim spellings).
- The overview does not name the surface list's "four copies". The ones
  found in production are `shared_group!`, `lookup.rs`,
  `SOURCE_AUTHORING_ENVIRONMENT_SURFACES` and `CATALOG`'s `base_layers`.
