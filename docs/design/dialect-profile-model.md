# The compositional `DialectProfile`

One `DialectProfile` per canonical dialect owns **both** the availability axis
(which commands and options exist) and the behaviour/runtime axis (how the
lexer, the expression grammar, and numeric literals behave). Every consumer
in the workspace derives its dialect meaning from that one profile rather
than reconstructing it.

The profile catalogue and both axes live in `rust/tcl-dialect`, a leaf crate
below `tcl-lexer`, so every layer of the stack can consume it without a
dependency cycle.

Section numbers in this document are stable: Rust doc comments across the
workspace cite them (`dialect-profile-model.md §7.2`), so renumber nothing.

---

## 0. Problem statement

Dialect is threaded through the entire stack (compiler, analyser, LSP, editors,
AI, CLI, codegen, runtime). Without a single owner, each consumer reconstructs
dialect meaning from two primitives — `DialectSet::parse(name)` and per-spec
`CommandSpec::supports_dialect` / `OptionSpec::supports_dialect` — and every
reconstruction gets one of two things wrong.

**A bare single bit is not a dialect.** `DialectSet::parse("f5-iapps")` yields
`IAPPS` alone, which does not intersect `TCL85_PLUS`, so real 8.5 core
(`dict`, `lassign`, `apply`) would be wrongly excluded under `f5-iapps`,
`expect`, and the EDA shells. The composed mask is the fix, and it has to be
composed **once**, in one place, because W123 (`unresolved.rs`), W002
(`validity.rs`), completion, semantic tokens, and the tools side
(`command_snapshot.rs`, backing `tcl registry-dump` / `command-info` /
`lookup` / `highlight --dialect`) all need the same answer.

**The behaviour/runtime axis needs the same owner.** Octal literal parsing,
the expr grammar (TIP 201/461), the mathfunc ceiling, number parsing, and the
lexer grammar are separate questions from availability, keyed off
`runtime_base` rather than `signature_base`. Split across independently-keyed
tables they drift; a profile that owns both axes cannot.

### 0.1 Scope of the consumer surface

Dialect logic is consumed across 22 crates besides `tcl-dialect` itself, plus
roughly 2000 `dialects:` data tags on registry command specs. The consuming
crates, by weight of use:

`tcl-registry`, `tcl-compiler`, `tcl-lsp-core`, `xtask`, `tcl-vm`,
`tcl-syntax`, `tcl-spec-studio`, `tcl-lexer`, `tcl-fuzz`, `tcl-cli`,
`tcl-bigip`, `tcl-vm-cli`, `tcl-mcp`, `tcl-lsp-server`, `tcl-lsp-db`,
`tcl-irules`, `tcl-irule-test`, `tcl-explorer`, `tcl-debugger`, `f5-xc`,
`f5-cli`, `bpf-tcl-ir`.

Two of those — `tcl-lexer` and `tcl-syntax` — sit *below* the registry in
the dependency order, which is why the profile catalogue cannot live in
`tcl-registry`. **Reachability from every one of these crates is a
first-class design constraint** and is what §3 resolves.

**The model:** one `DialectProfile` per canonical dialect owns **both** the
AVAILABILITY axis (A) and the BEHAVIOUR/RUNTIME axis (B); every consumer
derives from it. The deliberate signature-base vs runtime-base split is
preserved as two fields even where they are equal.

---

## 1. Settled facts (read before the model)

Four facts the model rests on. They shape everything below, so they lead.

### 1.1 iRules is pure fixed Tcl 8.4, nothing backported

iRules has `signature_base = runtime_base = V8_4`.
The F5 command surface (`HTTP::*`, `pool`, `table`, `TCP::*`, …) is a
**versioned library** keyed by BIG-IP/TMOS version (11.x–17.x), **orthogonal**
to the pinned 8.4 base — exactly as `Tk` / `Itcl` are versioned libraries over a
Tcl base. There is **no "classic vs modern 8.5/8.6" framing anywhere** in this
model: iRules never loads 8.6 signatures, at any BIG-IP version.

Concretely, at **every** BIG-IP version:
- `dict` / `lassign` (8.5), `lmap` / `throw` / `yieldto` (8.6), `zipfs` (9.0)
  are **NEVER present** — iRules embeds a genuine Tcl 8.4.6.
- What *does* change across BIG-IP versions is the F5 command library
  (`CMP::*` arrived in a specific TMOS; `HTTP2::*` later), modelled by
  `LibraryPin { package: "f5-irules-cmds", version: Keyed(BigipVersion),
  ambient: true }`.

`signature_base` and `runtime_base` stay structurally distinct (§7.1) even
though both read `V8_4` here, because the const-fold path
(`DialectProfile::const_fold_version`) and the expr-grammar path
(`expr_grammar_base`) are different projections; neither ever claims 8.6.

### 1.2 The crate-layering constraint

Dependency direction from the Cargo.toml files:

```
tcl-dialect (deps: bitflags only)        <- DialectSet, TclVersion, LexerGrammar,
   ^                                        LibraryPin, the DialectProfile catalogue
tcl-lexer   (deps: tcl-dialect, tcl-core-types, thiserror)
   ^
tcl-syntax  (deps: tcl-dialect, tcl-lexer)   <- parse_expr, format.rs, scan.rs live here
   ^
tcl-registry (deps: tcl-dialect, tcl-syntax, tcl-lexer, tcl-core-types, tcl-cmd-core,
              tcl-regex)                     <- CommandSpec, the command packs
```

A profile hosted in `tcl-registry` could not own the lexer grammar
(`LexerConfig`, in **tcl-lexer**) or the expression grammar
(`parse_expr`, in **tcl-syntax**): both crates are *below* registry, so
importing `DialectProfile` from there would be a dependency cycle. Collapsing
the parallel behaviour tables into one source is only achievable with the
behaviour-axis core below `tcl-lexer`.

That is what the foundational `tcl-dialect` crate (§3) is: `DialectSet`,
`TclVersion`, the grammar structs, and the `DialectProfile` catalogue live in
a leaf crate below `tcl-lexer`, so every layer consumes one source of
truth.

### 1.3 The unknown-dialect fallback

Every site that has to interpret a dialect string routes through
`DialectProfile::by_name`, and `by_name(unknown)` resolves to the single
`PLAIN_TCL` sink. That picks **one** answer for all of them:

| Site | Route | Fallback behaviour |
|---|---|---|
| W123 | `DialectProfile::by_name(dialect).resolve_command(…)` (`unresolved.rs`) | `ALL_TCL` — nothing unknown |
| W002 | the analyser's resolved profile (`validity.rs`) | `ALL_TCL` |
| `command_snapshot` | `DialectProfile::by_name(dialect)` (`command_snapshot.rs`) | `ALL_TCL` (§8) |
| `LexerConfig::for_dialect` | `LexerConfig::from_grammar(by_name(dialect).grammar)` (`lexer.rs`) | `GRAMMAR_TCL9X` — modern 9.x |
| `special_vars::resolve_dialect` | `DialectProfile::find(dialect).availability_mask` (`special_vars.rs`) | `ALL_TCL` |

The choice is the permissive one — `PLAIN_TCL.availability_mask = ALL_TCL`,
with a permissive behaviour axis (octal `Inert`, no version ceiling, no expr
grammar opinion) — so a typo flags nothing, which is the highest-visibility
behaviour in W123/W002. §8 is where that unification lives.

`special_vars::resolve_dialect` is the one site that does not go through
`by_name` alone: a name that parses to a `DialectSet` bit but is not a
catalog profile — today exactly `tk` — is unioned with `ALL_TCL`, because
`tk` is a Tcl superset providing the standard globals on top of its own.

### 1.4 `DialectSet` width is u64

`bitflags struct DialectSet: u64`. Bits used today: `TCL84`, `TCL85`,
`TCL86`, `TCL90` (0–3), `IRULES`, `IAPPS`, `TK`, `EXPECT` (4–7), `BPF` (13),
`TCL91` (14), `TMSH` (15), `BIGIP` (16) = **12 bits of 64**. Bits 8–12 are
vacant — they are the slots the five EDA vendor bits would occupy, and EDA
shells are modelled as a base Tcl version plus `required_package`-gated
command libraries instead ([eda-library-packages.md](eda-library-packages.md)).
Adding a dialect is therefore purely additive: allocate a bit and thread the
versioned-library dimension (§4).

---

## 2. Core model

The profile **wraps** the existing `DialectSet` bitflags rather than replacing
them, so `supports_dialect` / `get_for_dialect` signatures stay stable and
consumers migrate incrementally. The bitflags remain the low-level membership
atom; the profile *produces* the `DialectSet` consumers already accept — plus
the disable filter and the version guard the bare bitflags cannot express.

### 2.1 Two deliberately-separate base versions

- **`signature_base`** — the Tcl version whose command/subcommand/option
  *signatures* the dialect exposes (axis A). Feeds the availability mask.
- **`runtime_base`** — the Tcl version whose *evaluation semantics* apply:
  octal, expr grammar (TIP 201/461), mathfunc ceiling, number parsing,
  const-fold, VM parity (axis B).

For every ordinary Tcl version these are equal. For iRules both are `V8_4`
(§1.1). The two fields stay **structurally distinct** even when equal, because
the const-fold path (`TclVersion::from_dialect`) and the expr-grammar path
(`expr_grammar_base_version`) key off different projections and must not be
collapsed into one scalar (§7.1).

### 2.2 Rust types — availability axis (in the foundational crate; see §3)

Both axes are flat fields of one `DialectProfile` struct
(`rust/tcl-dialect/src/profile.rs`); there is no separate behaviour-projection
type. §2.2 and §2.3 split the same struct by axis for reading.

```rust
/// One resolved dialect. `'static`, interned in a catalog, keyed by canonical name.
pub struct DialectProfile {
    // ---- identity ----
    pub name: &'static str,                 // "tcl8.6", "f5-irules", …
    pub aliases: &'static [&'static str],   // "irules" -> f5-irules, "tcl-irule" -> f5-irules

    // ---- presentation (the catalog is the editors' source of truth) ----
    /// Menu label ("Synopsys EDA Tcl"); projected into every editor's
    /// dialect list by `cargo xtask gen-editor-dialects`.
    pub display_name: &'static str,
    /// Compact label for tight UI ("Synopsys EDA", "iRules") — the
    /// compiler-explorer dropdowns, diagnostic prose ("available in: …").
    pub short_name: &'static str,
    /// The dedicated editor language id, undotted ("tcl-synopsys",
    /// "tcl84"); None = the dialect's files ride the plain `tcl` language.
    pub editor_language_id: Option<&'static str>,
    /// The extensions this dialect owns, each with a human-facing name
    /// ("xdc" / "Xilinx Design Constraints"). Drives extension→dialect
    /// detection routing (the no-packs fallback under pack-declared
    /// `file_extension` rows) and `cargo xtask gen-editor-extensions`,
    /// which generates the editors' registered extension/language lists.
    pub file_extensions: &'static [DialectFileExtension],

    // ---- AXIS A: availability ----
    /// Native tag of this dialect's own command surface, if any (IRULES,
    /// IAPPS, EXPECT, TMSH, BIGIP, BPF). None for the plain Tcl-version
    /// profiles, the EDA shells, and the permissive fallback.
    pub vendor_bit: Option<DialectSet>,
    /// Precomputed membership mask = (signature_base version bits) | vendor_bit.
    /// This is what mask-membership is tested against. For iRules it is the
    /// BARE vendor bit (§9).
    pub availability_mask: DialectSet,
    /// load_dialect() packs to apply, in order (registry_for_profile). The EDA
    /// profiles carry only their version bit and load tool packs by profile
    /// name instead (load_eda_packs). Empty only for the fallback profile.
    pub base_layers: &'static [DialectSet],
    /// Coarse over-approximating union for STATIC grammars (tree-sitter /
    /// tmLanguage). Deliberately wider than availability_mask; see §10.
    pub grammar_union: DialectSet,
    /// UPPER-BOUND version guard: the highest Tcl version whose options may
    /// appear. Distinct from the mask so an option gated tcl9.0-only cannot leak
    /// into an 8.5-superset profile whose mask happens to intersect it (§5.2).
    pub version_ceiling: Option<TclVersion>,
}
```

There is **no `disabled_commands` field.** iRules availability is fully
explicit per spec instead — see §9.

The versioned-library axis is a slice of pins:

```rust
pub struct LibraryPin {
    pub package: &'static str,
    pub version: LibraryVersion,
    /// Ambient = part of the modelled runtime, no `package require` needed
    /// (the F5 surfaces, an EDA shell's tool commands). A hosted pin (Tk /
    /// Itcl on plain Tcl) still needs its require.
    pub ambient: bool,
}
pub enum LibraryVersion {
    TracksBase,             // == signature_base (Tk). Zero new machinery.
    Pinned(&'static str),   // fixed, e.g. Itcl "3.4"
    Keyed(VersionKey),      // resolved from the version axis
}
pub enum VersionKey { BigipVersion, ToolVersion, SdcVersion }
```

### 2.3 Rust types — behaviour axis

The remaining fields of the same struct:

```rust
    // ---- AXIS B: behaviour / runtime ----
    pub signature_base: Option<TclVersion>,
    pub runtime_base:   Option<TclVersion>, // None = "not Tcl" (f5-bigip)
    pub leading_zero_is_octal: Ternary,     // Yes/No/Inert — see §11.1
    pub expr_grammar_base:     Option<TclVersion>, // = runtime_base (TIP 201/461)
    pub grammar: LexerGrammar,              // the single source LexerConfig reads
    pub operators_as_commands: bool,        // false for iRules and the 8.4 profiles
    pub tcloo: bool,                        // explicit; invariant-tested vs mask (§11.2)
    pub has_fixed_ensembles: bool,          // {f5-irules, f5-iapps, f5-bigip} only
    pub vm_runtime_version: TclVersion,     // = runtime_base; V9_0 when inert

    // ---- AXIS C: versioned libraries (§7.1) ----
    pub libraries: &'static [LibraryPin],

    // ---- AXIS D: out-of-registry vendor knowledge ----
    /// Lower-case substring terms selecting this dialect's KCS help entries
    /// (`tcl help --dialect`). Empty = no filtering.
    pub help_terms: &'static [&'static str],
}

pub struct LexerGrammar {
    pub expand_syntax: bool,               // {*} — 8.5+
    pub irules_brace_separator: bool,      // }{ — iRules
    pub braced_var: BracedVarStyle,        // Tcl9Nesting vs FirstClose
    pub script_skips_leading_bom: bool,    // a whole-file source's BOM prologue
    pub expr_comments: ExprCommentStyle,   // `#` inside [expr] — TIP 582, 9.0
    pub numbers: NumberSyntax,             // Tcl84 / Tcl85 / Tcl90
    pub escapes: EscapeSyntax,             // Tcl84 / Tcl86 / Tcl90
}

/// Three-valued so f5-bigip (runtime_base=None, "not Tcl") is INERT, not
/// silently defaulted to octal/decimal (§11.1).
pub enum Ternary { Yes, No, Inert }
```

`escapes` is the axis that forces the 8.x grammar constant apart: 8.5 and 8.6
agree on every other field, but TIP 388 (8.6) capped `\x` at two hex digits,
added `\UHHHHHHHH`, and stopped an octal escape taking a third digit once the
first two reached `0x20`, so the catalog carries `GRAMMAR_TCL85` and
`GRAMMAR_TCL86` separately. 9.0 keeps 8.6's widths and raises `TCL_UTF_MAX` to
4, so a decoded scalar above U+FFFF stops degrading to U+FFFD. The decoder
(`tcl_lexer::backslash_subst_in`) and its extent rule
(`tcl_lexer::backslash_escape_end_in`) are one scan, so a release can never
give an escape one width and another value.

There is **no `mathfunc_ceiling` field.** The mathfunc tier is still derived
per call by `tcl_expr_eval::math_func_ceiling_for_dialect` in `tcl-compiler`,
keyed on the dialect name rather than read off the profile. That is the one
behaviour-axis table §5.4 names that has not moved onto the profile.

`is_irules` is a **method**, not a field (`DialectProfile::is_irules`), as are
`is_fallback` and `const_fold_version`.

Every field is written out literally in the catalog rather than computed at
construction — a `static` array of 16 profiles cannot run derivation code — so
the invariant tests in `profile.rs` (§7.1, §11.2) are what hold the derived
fields consistent with their bases.

### 2.4 Catalog + resolution boundary

```rust
impl DialectProfile {
    pub fn all() -> &'static [DialectProfile];             // the catalog, sorted-name order
    pub fn by_name(name: &str) -> &'static DialectProfile; // alias-normalised;
        // unknown -> PLAIN_TCL (availability_mask = ALL_TCL, behaviour permissive; §1.3/§8).
    pub fn by_opt_name(name: Option<&str>) -> &'static DialectProfile; // None -> PLAIN_TCL
    pub fn find(name: &str) -> Option<&'static DialectProfile>; // distinguishes "unknown"
    pub fn irules() -> &'static DialectProfile;            // explicit handle for hardcoded lookups
    pub fn plain_tcl() -> &'static DialectProfile;         // the sink itself
}
```

`all()` excludes `PLAIN_TCL`: the fallback is a resolution sink, not a
selectable dialect. `find` is the form that tells "unknown" apart from a real
profile, which is why `special_vars::resolve_dialect` uses it (§1.3).

The string→profile resolution happens **at ingest** (LSP `dialect_for_open` /
CLI `effective_dialect` / `detect_dialect`) and the `&'static DialectProfile`
is threaded from there, in place of repeated `DialectSet::parse` calls. The
dialect *name* stays the wire form for the config / `DocumentState` round-trip
(`tclLsp.selectDialect`, `folderDialects`, the registry-dump JSON schema,
`DialectSet::canonical_name`), so `profile.name` is the accessor those paths
read.

**Alias normalisation is load-bearing.** `by_name` canonicalises `irules` and
`tcl-irule` → `f5-irules`. Without it a profile predicate answered for the
alias spelling would silently disagree with the `dialect == "f5-irules"`
equality checks in `lsp-server/lib.rs` (`IRULES_DIALECT`), and with
`dialect_from_language_id`, which maps editor language ids — including the
undotted `tcl84` … `tcl91` spellings VS Code needs because a language id
containing a `.` cannot carry a `configurationDefaults` override (issue
#1122) — onto canonical dialect names.

A string-keyed public signature is not by itself a second source of truth, as
long as it resolves through the catalog immediately: `parse_expr(source,
dialect: Option<&str>)` still takes a name, but its first act is
`DialectProfile::by_opt_name(dialect)`, and it threads `profile.name`
downwards so the expr lexer's grammar branch and the parse cache key cannot
disagree about what a spelling means. `LexerConfig::for_dialect(name)` is the
same shape — a thin wrapper over `LexerConfig::from_grammar(by_name(name).grammar)`.

---

## 3. Crate layering — the `tcl-dialect` foundational crate

The behaviour axis (octal / expr grammar / lexer grammar) is consumed **below**
registry (`tcl-lexer`, `tcl-syntax`) and across leaf crates that path-dep
`tcl-lexer` directly. A registry-hosted `DialectProfile` would therefore be
unimportable from exactly the crates that need it most (§1.2).

### The `tcl-dialect` crate

`DialectSet`, `TclVersion`, the grammar structs (`LexerGrammar`,
`BracedVarStyle`), `LibraryPin` / `LibraryVersion` / `VersionKey`, and the
`DialectProfile` catalogue live in `rust/tcl-dialect`, a leaf crate whose only
dependency is `bitflags`:

```
tcl-dialect (deps: bitflags)  <- DialectSet, TclVersion, LexerGrammar, the catalogue
   ^          ^        ^
tcl-lexer  tcl-syntax  tcl-registry  ...  every layer consumes the profile
```

- `LexerConfig::for_dialect(name)` is
  `LexerConfig::from_grammar(DialectProfile::by_name(name).grammar)`, so the
  cross-crate `for_dialect` callers all read one grammar table.
- `parse_expr(source, dialect)` resolves through `by_opt_name` and threads the
  canonical `profile.name`, so the expr lexer's grammar branch and the parse
  cache key agree by construction (§2.4).
- `tcl-registry` keeps its `CommandSpec` metadata and re-exports the catalogue.

The cost is a real crate boundary: `DialectSet` and `TclVersion` are imported
from `tcl_dialect`, not `tcl_registry`, at every site. That is the price of
having one owner for the behaviour axis, which a registry-hosted profile
could not reach.

### 3.1 Alternatives considered — Option B (rejected)

*Rejected.* Option B keeps `DialectProfile` in registry owning only the
availability axis plus the registry-level behaviour bits, while
`LexerConfig::for_dialect` and `parse_expr` keep **their own string-keyed
tables**, reconciled by a cross-crate consistency test. It is cheaper (no crate
move) but accepts a documented split: the behaviour axis would have two
owners kept in sync by test rather than by construction. That fails the
single-source-of-truth goal for octal / expr-grammar / lexer-grammar, so it is
not the chosen path.

---

## 4. `DialectSet` width (u64)

`DialectSet` is backed by `u64` with 12 bits in use (§1.4), so bit exhaustion
is not a constraint and adding a dialect is purely additive:

- **New bits**: allocate the next free index — bits 8–12 and 17 upward are all
  available.
- **Serialisation** for a new bit: `command_snapshot.rs` / `registry-dump`
  encode bit values in `dialects_json`, so adding a bit regenerates that
  golden.
- The combinator constants (`ALL_TCL`, `TCL85_PLUS`, `TCL86_PLUS`, `TCL8X`,
  `TCL90_PLUS`, `TK_AND_TCL`) are `.bits()` unions and are width-agnostic.

A bit lands with the profile that needs it, never as a standalone width
change. `TMSH` (15) and `BIGIP` (16) are the worked examples: they exist
because `f5-tmsh` and `f5-bigip` are first-class profiles (D8), each with a
precise mask rather than a collapse to plain Tcl.

---

## 5. Resolution APIs every consumer calls

One method set, dispatching to the *correct* per-entity semantics (the
`intersects`-vs-`contains` distinction is load-bearing). The availability
queries need `CommandSpec`, which lives above the foundational crate, so they
are a **trait** — `tcl_registry::ProfileQueries`, implemented for
`DialectProfile` — rather than inherent methods. The behaviour queries (§5.4)
are inherent on `DialectProfile` itself.

### 5.1 Availability (axis A) — `ProfileQueries`

| API | Semantics |
|---|---|
| `p.is_available(&CommandSpec) -> bool` | `spec.supports_dialect(p.availability_mask)` **AND** `p.operators_as_commands \|\| !spec.traits.contains(OPERATOR_COMMAND)` **AND** `package_available(p, spec.required_package)` |
| `p.resolve_command(&reg, name) -> Option<&CommandSpec>` | the single availability primitive W123 / W002, completion, and the CLI snapshot share: `reg.get_for_dialect(name, p.availability_mask)` filtered by `is_available` (§5.3) |
| `p.is_subcommand_available(spec, sub)` | `sub.dialects.or(spec.dialects)` intersects `p.availability_mask`; a `None` gate on both means no restriction |
| `p.available_subcommands(spec)` | the filtered subcommand list, in declaration order |
| `p.is_option_available(opt, parent_gate)` | **profile-aware — see §5.2** |
| `p.available_option_names(spec)` / `p.available_option_specs(spec)` | the profile-aware `switch_names` / option table, declaration order, duplicates removed |
| `p.available_sub_option_names(spec, sub)` / `p.available_sub_option_specs(spec, sub)` | the same for a subcommand's options, which inherit `sub.dialects.or(spec.dialects)` as their parent gate |
| `p.find_option(spec, name, package_version)` | option lookup by canonical name or alias, honouring §5.2's gate and the resolved package version |
| `p.vendor_surface(&reg) -> Option<VendorSurface>` | this profile's own vendor commands, grouped by `NS::` prefix, sorted by descending size then name. `None` for a profile with no vendor surface. Feeds generated consumers (the AI prompt's F5-surface summary) so prose cannot drift from data |
| `p.keyed_version_range(spec)` | the declared introduction and removal releases of `spec` on this profile's keyed library axis, or the axis baseline when none is recorded. `None` for a spec outside a keyed pin |
| `p.keyed_pin_for(spec)` | the ambient `Keyed` `LibraryPin` `spec` belongs to under this profile |

Special-variable availability does not go through the trait: `special_vars`
tests `SpecialVarSpec::available_in(mask)` against
`special_vars::resolve_dialect(name)`, which is the profile's
`availability_mask` (§1.3).

The library-version resolvers are inherent on `DialectProfile`:
`library_pin(package)`, `is_ambient_package(package)`, `library_floor(…)`,
`library_floor_default(package)`, and `hosts_tk()`.

### 5.2 Option-gating

An option inherits its parent's gate when it declares none: `gate =
opt.dialects.or(parent_gate)`, where `parent_gate` is `spec.dialects` for a
command option and `sub.dialects.or(spec.dialects)` for a subcommand option.
`expect_after` (`commands/expect/expect_after.rs`) is the worked case: the
command is `dialects = Some(EXPECT)` and its `-re` / `-ex` / `-gl` /
`-nocase` / `-i` / `-info` options are all `dialects: None`.

Testing that gate with `contains` against a single bit cannot work. Passing
`signature_base` gives `TCL86` for expect, and `EXPECT.contains(TCL86) =
false`, so **every inherited option on every vendor command would silently
drop**. Conversely a core option gated `TCL85_PLUS` (real: `switch -nocase`)
needs a **version** bit, and `TCL85_PLUS.contains(IAPPS) = false`. **No
single bit satisfies `contains` for a composed `(version|vendor)` dialect.**

`is_option_available` therefore uses two tests, not one:

```rust
// p.is_option_available(opt, parent_gate), with gate = opt.dialects.or(parent_gate):
//   membership:  gate.intersects(p.availability_mask)      // NOT contains
//   upper-bound: gate.min_version() <= p.version_ceiling   // no 9.0-opt leak
// A `None` gate on both the option and its parent means "no restriction".
```

The upper bound is not optional decoration: `intersects` alone would admit a
`TCL90`-only option into an 8.5-superset profile whose mask happens to
intersect the gate. `DialectSet::min_version` derives the lowest Tcl-version
bit in the gate, so a `TCL85_PLUS` option resolves under an 8.5-or-later
ceiling while a `TCL90`-only option does not. A profile with no ceiling (the
permissive fallback) accepts every version, and a gate with no version floor
(a pure vendor gate) passes the bound unconditionally.

### 5.3 The single spec-selection strategy

`resolve_command` needs one rule for picking among several specs registered
under the same command name. The rule is **most-specific**, implemented once
in `CommandRegistry::best_visible` and reached through `get_for_dialect`:

```text
among the specs visible under the query mask, take the maximum of
  (spec.dialects.is_some(),                       // an explicit gate beats an open one
   Reverse(spec.dialects.bits().count_ones()),    // then the tightest gate
   index)                                         // then the last declared
```

"Tightest gate wins" is the principled reading — the best spec *for this
profile* — and it is what the golden `tcl registry-dump` snapshots are
written against. The declaration-index tiebreak only decides between two
equally-specific specs.

`spec_visible` is the visibility predicate `best_visible` filters on, and it
applies the same trio as `ProfileQueries::is_available` so a mask query on a
profile-stamped registry and a profile-side query can never disagree. It
short-circuits to the bare `supports_dialect` result when the query mask does
not intersect the registry's own profile mask, because such a query is asking
about some *other* dialect's availability and this profile's exclusions do not
apply to it.

### 5.4 Behaviour (axis B)

These are inherent on `DialectProfile`, and most are plain field reads — the
profile *is* the table.

| API | What it owns |
|---|---|
| `p.leading_zero_is_octal` | the octal rule, three-valued (§11.1). `CommandRegistry::octal_fold_policy` answers from it when the registry carries a profile |
| `p.expr_grammar_base` | TIP 201 (`in` / `ni`, 8.5+) and TIP 461 (`lt` / `le` / `gt` / `ge`, 9.0+) gating |
| `p.runtime_base` | the evaluation-semantics version |
| `p.const_fold_version()` | the version const-folding evaluates at |
| `p.grammar` | the lexing grammar `LexerConfig::for_dialect` reads |
| `p.has_fixed_ensembles` / `p.is_irules()` / `p.operators_as_commands` / `p.tcloo` | the predicates that replace open-coded `matches!(dialect, Some("irules" \| "f5-irules"))` copies and the minifier's prefix-shortening gate |
| `p.effective_tcl_version(package_floor)` | the version the argument-DSL validators consult (§6) |
| `p.vm_runtime_version` | the Tcl release the bytecode VM emulates; matches `runtime_base`, or `V9_0` for an inert profile |
| `p.help_terms` | the KCS help-index filter for `tcl help --dialect` |

The **mathfunc ceiling is the exception**: it is still derived per call by
`tcl_expr_eval::math_func_ceiling_for_dialect` in `tcl-compiler`, keyed on
the dialect name rather than read off the profile. Folding it onto the
profile alongside `expr_grammar_base` is unfinished work.

---

## 6. The granularity ladder — the argument-DSL rung

Dialect gating reaches four rungs deep, not three. The first three are
mask-driven: `version_gate.rs` records a `Lifecycle` at the command head and
at each option token, checked against the resolved `package require` floor
(W135 / W136 / W139 / W144). The fourth rung descends **into an argument's
mini-language**, because dialect and version differences reach in there too:

- **`format` / `scan` conversion strings** — conversion specifiers and size
  modifiers differ across releases. Gated by **W138**.
- **`string is` classes** — the class set is version-dependent (`wideinteger`
  is 8.5+, `entier` 8.6+, `dict` 9.0+). Gated by **W137**.
- **`regexp` / `subst` flags** and **`clock format` specifiers** are *not*
  modelled: no version-gated conversion table exists for them, so they are
  deliberately left alone rather than guessed at. **`expr` operators and
  functions** are gated separately, by W003 off `expr_grammar_base`.

### 6.1 The ladder

```
command            e.g.  lmap            gated by availability_mask   (W123/W002)
  subcommand       e.g.  dict getwithdefault  gated by mask         (W002)
    option         e.g.  switch -nocase  gated by mask + version_ceiling (§5.2, W136)
      argument-DSL e.g.  format %llu     gated by effective_tcl_version()  (W137/W138)
```

The profile resolves the **effective Tcl version** the DSL validators consult:

```rust
impl DialectProfile {
    /// The Tcl version an argument mini-language (format/scan/string is/…) must
    /// validate against — the runtime_base, raised to any package floor the
    /// caller supplies. Permissive (None) for PLAIN_TCL and non-Tcl profiles.
    pub fn effective_tcl_version(&self, package_floor: Option<TclVersion>) -> Option<TclVersion>;
}
```

`Analyser::effective_dsl_version` supplies that floor: the highest version
named by an **unconditional** `package require Tcl` in the file. A `None`
result — the permissive fallback and the non-Tcl profiles — makes every DSL
check abstain rather than guess.

### 6.2 How the rung is wired

The version knowledge lives with the mini-language parser and the gating
lives with the analyser:

- `tcl_syntax::format::version_gated_uses(fmt)` and
  `tcl_syntax::scan::version_gated_uses(fmt)` return each version-gated
  feature in a literal conversion string with its minimum Tcl version.
  `parse_spec` / `parse_conversion` themselves stay version-agnostic — they
  answer "what is this specifier", not "is it allowed here".
- The analyser **buffers** sites as `DslGateSite` during the walk
  (`record_dsl_format_sites`) and flushes them once at the end
  (`flush_dsl_gate_diagnostics`), because the effective version depends on
  `package require` lines that may appear anywhere in the file.
- Which argument is a format string comes from the registry
  (`CommandRegistry::format_string_args` for compatibility callers and
  `format_string_args_words_for_dialect` for source-aware consumers), never
  from matching a command name. The source-aware query receives each word's
  literal/dynamic/expanded shape and the profiled option table: an unresolved
  leading `$mode` must not be projected into a positional `regsub`
  replacement or a pattern role. The same proof boundary owns regex/glob
  pattern queries. The `FormatType` family check is load-bearing: `clock`'s
  field string, `binary`'s cursor spec, and `regsub`'s backreference template
  all sit at `ArgRole::FormatString` / `ArgRole::ScanFormat` positions too,
  and running the sprintf table over `clock format $t -format {%b}` would
  report a Tcl 8.6 requirement for a conversion that has nothing to do with
  `format`. Only `FormatType::Sprintf` words are gated.
- A word whose token is a `Var` or `Cmd` substitution is skipped: its text is
  not the literal %-string.

Because `format.rs` / `scan.rs` live in `tcl-syntax`, *below* registry, they
can only see the version model at all because §3 puts the catalogue below
`tcl-syntax` too.


---

## 7. Per-dialect profile table

`sig`=signature_base, `rt`=runtime_base, `oct`=leading_zero_is_octal,
`ens`=has_fixed_ensembles, `ops`=operators_as_commands,
`mask`=availability_mask (precise), `ceil`=version_ceiling. Libraries reuse
`spec.rs` `Lifecycle` + `available_for_version` — **no parallel version
machinery**.

| Profile | sig | rt | oct | tcloo | ens | ops | mask (precise) | ceil | Libraries (all ambient unless noted) |
|---|---|---|---|---|---|---|---|---|---|
| `tcl8.4` | V8_4 | V8_4 | ✓ | ✗ | ✗ | **✗** | `TCL84` | V8_4 | Tk `TracksBase`, Itcl `Pinned(3.4)` — both **hosted** |
| `tcl8.5` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | `TCL85` | V8_5 | Tk `TracksBase`, Itcl `Pinned(3.4)` — both hosted |
| `tcl8.6` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | `TCL86` | V8_6 | Tk `TracksBase`, Itcl `Pinned(4.2)` — both hosted |
| `tcl9.0` | V9_0 | V9_0 | **✗** | ✓ | ✗ | ✓ | `TCL90` | V9_0 | Tk `TracksBase`, Itcl `Pinned(4.2)` — both hosted |
| `tcl9.1` | V9_1 | V9_1 | ✗ | ✓ | ✗ | ✓ | `TCL91` (inherits 9.0) | V9_1 | as 9.0 |
| **`f5-irules`** | **V8_4** | **V8_4** | ✓ | ✗ | **✓** | **✗** | **`IRULES` (bare!)** | V8_4 | `f5-irules-cmds` `Keyed(BigipVersion)`. **8.4 pinned forever — dict/lassign (8.5), lmap/throw (8.6), zipfs (9.0) NEVER present at ANY BIG-IP version** |
| **`f5-iapps`** | V8_5 | V8_5 | ✓ | ✗ | ✓ | ✓ | **`TCL85\|IAPPS`** | V8_5 | `f5-iapps-cmds` `Keyed(BigipVersion)`. Host Tcl 8.5.13: has dict/lassign, no lmap/8.6 |
| `f5-tmsh` | V8_5 | V8_5 | ✓ | ✗ | **✗** | ✓ | **`TCL85\|TMSH`** | V8_5 | `f5-tmsh-cmds` `Keyed(BigipVersion)` |
| `f5-bigip` | **None** | **None** | **Inert** | ✗ | ✓ | **✗** | **`BIGIP`** (config surface, no Tcl command surface) | None | `f5-bigip-schema` `Keyed(BigipVersion)` |
| `expect` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | **`TCL86\|EXPECT`** | V8_6 | Expect `Pinned(5.45.4)` |
| `synopsys-eda-tcl` | V8_6 | V8_6 | ✓ | ✓ | ✗ | ✓ | **`TCL86`** | V8_6 | sdc `Keyed(SdcVersion)` + 5 tool packs `Keyed(ToolVersion)` |
| `cadence-eda-tcl` | **V8_4** | **V8_4** | ✓ | **✗** | ✗ | **✗** | **`TCL84`** | V8_4 | sdc + 4 tool packs |
| `xilinx-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | **`TCL85`** | V8_5 | sdc + `vivado` |
| `intel-quartus-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | **`TCL85`** | V8_5 | sdc + 7 `quartus-*` packs |
| `mentor-eda-tcl` | **V8_6** | **V8_6** | ✓ | **✓** | ✗ | ✓ | **`TCL86`** | V8_6 | sdc + `questa`, `questa-formal`, `calibre` |
| `bpf` | **V9_0** | **V9_0** | **✗** | ✓ | ✗ | ✓ | **`TCL90\|BPF`** | **V9_0** | — |
| `PLAIN_TCL` (unknown) | **None** | **None** | **Inert** | ✓ | ✗ | ✓ | **`ALL_TCL`** (§1.3/§8) | None | — |

The five EDA profiles carry **no vendor bit**: their masks are the bare base
version, and their tool commands are gated by `required_package` against the
ambient library pins instead
([eda-library-packages.md](eda-library-packages.md)). `f5-bigip` is the other
profile whose mask is not a version union — `BIGIP` alone, because it is a
configuration surface with no Tcl command surface, which is also why its
`grammar_union` is the bare `BIGIP` bit rather than `ALL_TCL`.

`grammar_union` is `ALL_TCL | vendor_bit` for every profile except the two
whose static grammar is deliberately scoped tight: `f5-irules` (bare `IRULES`)
and `f5-bigip` (bare `BIGIP`). See §10.

`operators_as_commands` is false for the 8.4-based profiles (`tcl8.4`,
`cadence-eda-tcl`) as well as for `f5-irules` and `f5-bigip`: `::tcl::mathop`
arrived in 8.5, so on an 8.4 core the operators genuinely are not command
heads. Under iRules they are absent for a different reason — operators there
live only inside `expr`.

### 7.1 Derivation rules

The catalog is a `static` array, so these are consistency rules the profile
values must satisfy, enforced by the invariant tests in `profile.rs` rather
than by construction code.

- `leading_zero_is_octal = if runtime_base is None { Inert } else { runtime_base < V9_0 }`
  — the `Inert` branch (`f5-bigip` and the permissive fallback) is explicit,
  **not** a silent `false`/`true` default (§11.1). For `bpf` this reads `No`:
  its `runtime_base` is `V9_0`, and Tcl 9.0 dropped bare-leading-`0` octal
  (TIP 114/472), so `0NN` is decimal there.
- `expr_grammar_base = runtime_base`. `None` means the validators return only
  the dialect-invariant subset.
- `vm_runtime_version = runtime_base`, falling back to `V9_0` for a profile
  with no runtime base at all.
- `tcloo` is **explicit per profile**, invariant-tested against the mask
  (§11.2): it must equal `availability_mask.intersects(TCL86_PLUS)`, with
  `f5-bigip` the one documented exception (no Tcl surface at all, so `false`
  is asserted directly).
- `operators_as_commands` is false where `::tcl::mathop` heads do not exist:
  the 8.4-based profiles (`tcl8.4`, `cadence-eda-tcl`), `f5-irules` (operators
  live only inside `expr`), and `f5-bigip`.
- `has_fixed_ensembles` is exactly `{f5-irules, f5-iapps, f5-bigip}` — **NOT
  `f5-tmsh`**. It drives the minifier's subcommand prefix-shortening, so a
  wrong `true` mis-minifies. Covered by its own invariant test.
- The keyed library axes (`BigipVersion`, `ToolVersion`, `SdcVersion`) default
  to the **oldest supported version**, the conservative choice: by default
  only floor-version commands are offered, and newer ones (a `CMP::*` /
  `HTTP2::*` introduced in a later TMOS, a newer synthesis-tool subcommand)
  stay hidden until the file pins a newer version. A default of "latest" would
  silently mark genuinely-unavailable commands as known on older targets;
  "oldest" never over-reports availability. `VersionKey::default_version` is
  where those floors live — `BigipVersion` is `16.1.0`, while `ToolVersion`
  and `SdcVersion` are `None` (permissive) because no registry pack carries
  keyed tool or SDC introduction data yet. It is deliberately distinct from
  `VersionKey::baseline_version` (`15.0.0` for `BigipVersion`), which says
  what the *data* claims — an item with no explicit `min_version` is asserted
  present since 15.0 — rather than what the user targets when they pin
  nothing.

### 7.2 Precision costs false positives, and that is the trade

Giving a profile a precise mask instead of letting it collapse to `ALL_TCL`
is not a free win. `f5-tmsh` at `TCL85|TMSH` and `bpf` at `TCL90|BPF` means
8.6/9.0 core commands correctly draw W123 in a tmsh file and 8.x-only relics
correctly draw it in a bpf file — but those are *general* Tcl commands, and
any modelling error in the base version shows up as a false positive across
the whole file rather than on the vendor surface alone. Precision on the base
version is therefore load-bearing for a vendor profile in a way it is not for
a plain Tcl profile.

**Not dialects — modelled as `LibraryPin`, not profiles:** `tk` (a `Tk`
`TracksBase` pin on a Tcl profile — `wish` is a Tcl base plus Tk; the
standalone `TK` bit exists only for the grammar layer), `itcl`, `tcllib`,
`argparse`, `ticklecharts`. `DialectProfile::hosts_tk()` is the predicate for
"can this profile `package require Tk`": true for the plain Tcl versions and
the permissive fallback, false for every closed-world vendor shell, which is
what consumers ask now that the EDA profiles carry no vendor bit.

---

## 8. Unified unknown-dialect fallback

`PLAIN_TCL` is the single sink for every unparseable or mistyped dialect
string. `by_name(unknown) -> &PLAIN_TCL`, which is deliberately permissive on
both axes:

- `availability_mask = ALL_TCL` and `base_layers = &[]` — nothing is unknown,
  and no pack is loaded.
- `version_ceiling = None`, `signature_base = None`, `runtime_base = None`,
  `leading_zero_is_octal = Inert`, `expr_grammar_base = None`,
  `grammar = GRAMMAR_TCL9X` (modern 9.x lexing), `help_terms = &[]` (no
  filtering).

A typo therefore flags nothing, which is the highest-visibility behaviour in
W123 / W002. `command_snapshot` resolves through `by_name` like everything
else, so an unknown-dialect `registry-dump` renders the `ALL_TCL` view rather
than an ad-hoc one.

`PLAIN_TCL` is not in `DialectProfile::all()` — it is a resolution sink, not
a selectable dialect — so a caller that must enumerate real dialects gets the
16 catalog entries, and a caller that must include the fallback reaches it
through `DialectProfile::plain_tcl()`. `is_fallback()` is the predicate for
"did this resolve to the sink", which is how `hosts_tk()` treats an unlabelled
`wish` shell as Tk-capable.

---

## 9. `dialects: Option<DialectSet>` and the iRules subtractive trap

The per-command `dialects: Option<DialectSet>` field stays as the intrinsic
native-version / native-layer tag. The profile supplies the query mask; the
data is not migrated wholesale, and `supports_dialect(intersects)` against a
profile-supplied mask composes cleanly.

**iRules availability is subtractive in appearance only.** F5's TMM
interpreter removes about fifty commands from iRules — the K36322151 sandbox
bans (`exec`, `file`, `socket`, `open`, `glob`, `source`, `cd`, `pwd`,
`fconfigure`, `fcopy`, `gets`, `vwait`, …) plus the project-modelled
iRules-excluded internals. There is **no ban list** anywhere in the model.
Instead:

1. **Each banned command carries an explicit `dialects` group without the
   `IRULES` bit** — typically `ALL_TCL`. The spec still exists, so the LSP can
   distinguish "exists, but not in iRules" from "unknown"; it simply does not
   intersect the bare `IRULES` mask. Universal `dialects: None` was eliminated
   registry-wide precisely so this works: with no universal tag left, absence
   of the `IRULES` bit is a positive statement, not an accident.
2. **The math-operator heads** (`+`, `eq`, `tcl::mathop::*`) are excluded by
   dialect *shape* rather than by tag: a spec carries
   `Traits::OPERATOR_COMMAND` iff it is a `tcl::mathop` spelling, and
   `is_available` drops it when `operators_as_commands` is false. That is a
   separate fact from the sandbox bans and is modelled separately.

Both directions are contract-tested in
`rust/tcl-registry/tests/dialect_profile.rs`: every banned name must resolve
to registered spec data *and* must lack the `IRULES` bit, and
`OPERATOR_COMMAND` must mark exactly the `tcl::mathop` spellings.

### 9.1 Why the general widen-fix is wrong for iRules

Composing `(version | vendor)` is the right mask for `f5-iapps`, `expect`, and
the EDA shells, and exactly wrong for iRules: `TCL84|IRULES` would re-admit
every sandbox-banned command, because a banned command's gate contains a Tcl
version bit and `intersects` would match on it. `f5-irules.availability_mask`
is therefore the **bare `IRULES` bit** — the same value its `grammar_union`
carries — and a command is in iRules iff its own gate says so.

### 9.2 Why "no ban list" is the safer shape

A subtractive list has a standing hazard: any availability path that queries
the mask without also applying the list re-admits `exec` / `file` / `socket`
under iRules, and there are many such paths (`get_for_dialect` callers,
`resolve_dialect("f5-irules")` callers, the CLI snapshot's `command_names`)
across the consuming crates.

Encoding the exclusion in the spec's own `dialects` group removes the hazard
by construction: a mask query *is* the whole test, so a consumer that forgets
the profile-side filter still gets the right answer. The one remaining
profile-level exclusion is the operator-command one, and
`CommandRegistry::spec_visible` applies it inside the registry so that mask
queries and `ProfileQueries::is_available` cannot disagree.

---

## 10. Precise vs coarse masks (static grammars)

Tree-sitter / tmLanguage queries are static per filetype, so
over-approximation is intentional: `f5-iapps` highlights against
`ALL_TCL|IAPPS`, pulling in 8.6/9.0 words the real 8.5.13 base lacks, because
precise per-version correctness is the LSP semantic-token layer's job — it
sees the file, the static query does not. The profile therefore exposes
**two** projections:

- `availability_mask` — precise (CLI, LSP, diagnostics, completion). iApps is
  `TCL85|IAPPS` exactly.
- `grammar_union` — coarse over-approximation, static grammars only. iApps is
  `ALL_TCL|IAPPS`, preserving first-paint highlighting of 9.0 commands.

`f5-irules` and `f5-bigip` are the exceptions where the coarse projection is
*not* wider: both `grammar_union`s are the bare vendor bit. Widening iRules
would paint 8.5+ core words that genuinely do not exist there, which is worse
than under-painting.

`gen_zed_queries::targets()` names profiles rather than composing literal
unions — `plain_tcl()`, `f5-irules`, `f5-iapps`, `expect` — and takes each
target's `grammar_union` for the static buckets and
`registry_for_profile(profile)` for the command list, so the mask query
applies the same visibility rules (§9) the LSP does.

The static projection covers only the **ambient** command surface: a command
with a `required_package` (Tk, tcllib, the stdlib packages, itcl,
ticklecharts) needs a `package require` the static query cannot see, so it is
deliberately left to the semantic-token layer. The F5 and EDA surfaces are
ambient in their runtimes, so they stay. Namespaced ambient commands are kept
too — the F5 surface (`HTTP::uri`, `LB::server`) is entirely namespaced, and
the tree-sitter Tcl grammar parses `ns::cmd` as a single `simple_word`.

---

## 11. Behaviour-axis None paths and the tcloo invariant

### 11.1 None paths are explicit, not defaulted

`leading_zero_is_octal = runtime_base < V9_0` and `expr_grammar_base =
runtime_base` are **ill-defined** where there is no `runtime_base` at all —
`f5-bigip`, a configuration surface, and `PLAIN_TCL`, which has no opinion by
design. The `Ternary::Inert` variant (§2.3) makes that case inert: the octal
and expr validators short-circuit to "no opinion" rather than silently reading
octal or decimal. §7.1 states the `None` branch explicitly.

`CommandRegistry::octal_fold_policy` carries the same three-valued answer
outward: `Some(true)` for an 8.x profile, `Some(false)` for a 9.x one, and
`None` to abstain. A registry assembled by hand, with no profile stamped on
it, falls back to deriving from `loaded_dialects` instead.

### 11.2 The tcloo bool is invariant-tested against the mask

The profile sets `tcloo` per dialect, but hover, completion, and the `oo`
handler **also** resolve `oo::*` specs through the mask (gated `TCL86_PLUS`).
Nothing about the struct forces those two to agree, and a profile whose
`tcloo` contradicted its mask would give contradictory `oo` behaviour and
hover text.

`tcloo_is_invariant_with_the_availability_mask` in `profile.rs` enforces it
over every catalog profile plus the fallback:
`p.tcloo == p.availability_mask.intersects(TCL86_PLUS)`. `f5-bigip` is the one
documented exception — it has no Tcl surface at all, so `!p.tcloo` is asserted
directly rather than derived from a mask that carries no version bit.
