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
AI, CLI, codegen, runtime), but there is **no single owner**. Consumers
reconstruct dialect meaning independently via two moves:

1. `DialectSet::parse(name)` → a **bare single bit** (`f5-iapps` → `IAPPS` only), then
2. per-spec `CommandSpec::supports_dialect(intersects)` /
   `OptionSpec::supports_dialect(contains)`.

The bare bit is the confirmed defect: `IAPPS` does not intersect `TCL85_PLUS`,
so real 8.5/8.6 core (`dict`, `lassign`, `apply`, `lmap`, `coroutine`) is
wrongly excluded under `f5-iapps` / `expect` / EDA. This is live in **W123**
(`unresolved.rs:164`), **W002** (`validity.rs:561`), completion,
semantic-tokens, and on the **tools side** (`command_snapshot.rs` →
`tcl registry-dump/command-info/lookup/highlight --dialect f5-iapps`).

The behaviour/runtime axis has **no owner either**: it is spread across
independently-keyed tables that already disagree —
`DialectSet::expr_grammar_base_version("f5-irules")=TCL84` (`dialects.rs:970`)
while `TclVersion::from_dialect("f5-irules")=None` (`hooks.rs:420`), two
`leading_zero_is_octal` sources, and `LexerConfig::for_dialect`
(`lexer.rs:213`).

### 0.1 Scope of the consumer surface

Dialect logic is consumed across roughly 269 files in 22 crates — about
1700 query and behaviour call sites plus about 2000 `dialects:` data tags on
registry specs. Every crate in the list below reads dialect meaning:

`tcl-compiler`, `tcl-lsp-core`, `tcl-registry`, `tcl-cli`, `tcl-mcp`,
`xtask`, `tcl-lexer`, `tcl-explorer`, `tcl-lsp-db`, `tcl-diagram`,
`tcl-cli-support`, `f5-cli`, `bigip-report-gen`, `tcl-lsp-server`,
`tcl-irules`, `f5-xc`, `bigip-query-wasm`, `tcl-syntax`, `tcl-vm`,
`tcl-bigip`, `bpf-tcl-ir`, `tcl-pkg`.

Two of those — `tcl-lexer` and `tcl-syntax` — sit *below* the registry in
the dependency order, which is why the profile catalogue cannot live in
`tcl-registry`. **Reachability from every one of these crates is a
first-class design constraint** and is what §3 resolves.

**Goal:** one `DialectProfile` per canonical dialect owning **both** the
AVAILABILITY axis (A) and the BEHAVIOUR/RUNTIME axis (B); every consumer derives
from it. The subtractive-iRules case and the deliberate signature-base vs
runtime-base split are preserved exactly.

---

## 1. Settled facts (read before the model)

Four facts the model rests on. They shape everything below, so they lead.

### 1.1 iRules is pure fixed Tcl 8.4, nothing backported

iRules has `signature_base = runtime_base = V8_4`.
The F5 command surface (`HTTP::*`, `pool`, `table`, `TCP::*`, …) is a
**versioned library** keyed by BIG-IP/TMOS version (11.x–17.x), **orthogonal**
to the pinned 8.4 base — exactly as `Tk`/`tcllib` are versioned libraries over a
Tcl base. There is **no "classic vs modern 8.5/8.6" framing anywhere** in this
model. The stale "iRules loads `tcl8.6` signatures" line
(`dialects-events.md:74-78`) is **retired** by this ratification (see below).

Concretely, at **every** BIG-IP version:
- `dict` / `lassign` (8.5), `lmap` / `throw` / `yieldto` (8.6), `zipfs` (9.0)
  are **NEVER present** — iRules embeds a genuine Tcl 8.4.6.
- What *does* change across BIG-IP versions is the F5 command library
  (`CMP::*` arrived in a specific TMOS; `HTTP2::*` later), modelled by
  `LibraryPin { package: "f5-irules-cmds", version: Keyed(BigipVersion) }`.

This corrects the stale framing in `docs/design/compiler/dialects-events.md:74-78`
("iRules loads `tcl8.6` signatures for command availability"). That sentence is
**retired** by this doc: iRules never loaded 8.6 signatures conceptually; it
carried a bare `IRULES` bit and a disable list, which the model below makes
explicit. The already-verified anchor
`expr_grammar_base_version("f5-irules")=TCL84` (`dialects.rs:970`) and
`from_dialect("f5-irules")=None` (`hooks.rs:420`) both already agree with
8.4-runtime; the model keeps them structurally distinct (§7.1) but neither ever
claims 8.6.

### 1.2 The crate-layering constraint

Verified dependency direction from the Cargo.toml files:

```
tcl-lexer   (deps: thiserror only)
   ^
tcl-syntax  (deps: tcl-lexer)           <- parse_expr, format.rs, scan.rs live here
   ^
tcl-registry (deps: tcl-syntax, tcl-lexer, tcl-core-types, tcl-cmd-core)  <- DialectSet, TclVersion live here
```

A profile hosted in `tcl-registry` cannot own the lexer grammar
(`LexerConfig::for_dialect`, in **tcl-lexer**) or the expression grammar
(`parse_expr(source, dialect)`, in **tcl-syntax**): both crates are *below*
registry and importing `DialectProfile` would be a dependency cycle. That is
precisely why those two entry points are string-keyed rather than
`DialectSet`-keyed. Collapsing the parallel behaviour tables into one source
is only achievable if the behaviour-axis core sits below `tcl-lexer`.

That is what the foundational `tcl-dialect` crate (§3) is for: `DialectSet`,
`TclVersion`, the grammar structs, and the `DialectProfile` catalogue live in
a leaf crate below `tcl-lexer`, so every layer consumes one source of
truth.

### 1.3 The unknown-dialect fallback

The fallbacks for an unparseable or mistyped dialect string, per site:

| Site | Fallback | Anchor |
|---|---|---|
| W123 | `ALL_TCL` | `unresolved.rs:164` |
| W002 | `ALL_TCL` | `validity.rs:561`, `:941` |
| `command_snapshot` | `TCL86` | `command_snapshot.rs:426,433` |
| `LexerConfig::for_dialect` | `default()` = modern-9.x | `lexer.rs:209-211` |
| `special_vars::resolve_dialect` | `ALL_TCL` | `special_vars.rs:191` |

`by_name(unknown) -> PLAIN_TCL` picks **one** mask for all of them. The
choice is the permissive one — `PLAIN_TCL.availability_mask = ALL_TCL`, with
a permissive behaviour axis (octal, mathfunc, and expr grammar all
permissive) — so a typo flags nothing, which is the highest-visibility
behaviour in W123/W002. §8 is where that unification lives.

### 1.4 `DialectSet` width is u64

`bitflags struct DialectSet: u64`. Bits used today: `TCL84,TCL85,TCL86,TCL90` (0–3),
`IRULES,IAPPS,TK,EXPECT` (4–7), `SYNOPSYS,CADENCE,XILINX,QUARTUS,MENTOR` (8–12),
`BPF` (13), `TCL91` (14) = **15 bits of 64**, leaving ample headroom for a
`TMSH` bit, a `BIGIP` bit, and future vendors. Adding a dialect is therefore
purely additive: allocate a bit and thread the versioned-library dimension
(§4).

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

```rust
/// One resolved dialect. `'static`, interned in a catalog, keyed by canonical name.
pub struct DialectProfile {
    // ---- identity ----
    pub name: &'static str,                 // "tcl8.6", "f5-irules", …
    pub aliases: &'static [&'static str],   // "irules" -> f5-irules, "tcl-irule" -> f5-irules

    // ---- AXIS A: availability ----
    /// Native tag of this dialect's own layer, if any (IRULES, IAPPS, EXPECT, …).
    /// None for the plain Tcl-version profiles.
    pub vendor_bit: Option<DialectSet>,
    /// load_dialect() packs to apply, in order (registry_for_profile). Usually
    /// 0 or 1; EDA loads sdc_base + vendor. Lets f5-tmsh/f5-bigip get real arms.
    pub base_layers: &'static [DialectSet],
    /// Precomputed membership mask = (signature_base version-bits) | vendor_bit.
    /// This is what mask-membership is tested against. For SUBTRACTIVE profiles
    /// (iRules) this is the BARE vendor bit only (§9).
    pub availability_mask: DialectSet,
    /// UPPER-BOUND version guard: the effective Tcl version whose options may
    /// appear. Distinct from the mask so an option gated tcl9.0-only cannot leak
    /// into an 8.5-superset profile whose mask happens to intersect it (§5.2).
    pub version_ceiling: Option<TclVersion>,
    /// Subtractive disable list (iRules' 42 K36322151 commands). Applied AFTER
    /// the mask query by EVERY availability consumer (§9). Empty for additive dialects.
    pub disabled_commands: &'static [&'static str],
    /// Coarse over-approximating union for STATIC grammars (tree-sitter /
    /// tmLanguage). Deliberately wider than availability_mask; see §10.
    pub grammar_union: DialectSet,

    // ---- versioned libraries (reuses spec.rs Lifecycle + available_for_version) ----
    pub libraries: &'static [LibraryPin],
    pub bigip_version: Option<&'static str>, // F5 command-surface library key
    pub tool_version:  Option<&'static str>, // EDA vendor-tool library key
    pub sdc_version:   Option<&'static str>, // EDA sdc library key
}

pub struct LibraryPin { pub package: &'static str, pub version: LibVersion }
pub enum LibVersion {
    TracksBase,             // == signature_base (tcllib/Tk/Itcl). Zero new machinery.
    Pinned(&'static str),   // fixed, e.g. Itcl "3.4"
    Keyed(VersionAxis),     // resolved from a profile field
}
pub enum VersionAxis { BigipVersion, ToolVersion, SdcVersion }
```

### 2.3 Rust types — behaviour axis

```rust
pub struct DialectBehaviour {              // the axis-B projection of a profile
    pub signature_base: Option<TclVersion>,
    pub runtime_base:   Option<TclVersion>, // None = "not Tcl" (f5-bigip)
    pub leading_zero_is_octal: Ternary,     // Yes/No/Inert — see §11 for None paths
    pub expr_grammar_base:     Option<TclVersion>, // = runtime_base (TIP 201/461)
    pub mathfunc_ceiling:      Option<MathFuncSince>,
    pub grammar: LexerGrammar,              // lifted from LexerConfig::for_dialect
    pub operators_as_commands: bool,        // false for iRules AND tk
    pub tcloo: bool,                        // explicit; invariant-tested vs mask (§11.2)
    pub has_fixed_ensembles: bool,          // {f5-irules,f5-iapps,f5-bigip} only
    pub is_irules: bool,                    // canonical iRules-behaviour predicate
    pub vm_runtime_version: TclVersion,     // = runtime_base; V9_0 only when inert
}

pub struct LexerGrammar {                  // verbatim from LexerConfig::for_dialect
    pub expand_syntax: bool,               // {*} — 8.5+
    pub irules_brace_separator: bool,      // }{ — iRules
    pub braced_var: BracedVarStyle,        // Tcl9Nesting vs FirstClose
}

/// Three-valued so f5-bigip (runtime_base=None, "not Tcl") is INERT, not
/// silently defaulted to octal/decimal (§11.1).
pub enum Ternary { Yes, No, Inert }
```

`availability_mask`, `version_ceiling`, `leading_zero_is_octal`,
`expr_grammar_base`, and `grammar_union` are **derived once** at
catalog-construction time from `signature_base`/`runtime_base`/`vendor_bit`/
`disabled_commands` — not recomputed per call. This collapses the parallel
tables into one source (subject to §3's layering resolution).

### 2.4 Catalog + resolution boundary

```rust
impl DialectProfile {
    pub fn all() -> &'static [&'static DialectProfile];    // = the catalog
    pub fn by_name(name: &str) -> &'static DialectProfile; // alias-normalized;
        // unknown -> PLAIN_TCL (availability_mask = ALL_TCL, behaviour permissive; §1.3/§8).
    pub fn irules() -> &'static DialectProfile;            // explicit handle for hardcoded lookups
}
```

The string→profile resolution happens **once at ingest** (LSP
`dialect_for_open` / CLI `effective_dialect` / `detect_dialect`) and the
`&'static DialectProfile` is threaded, replacing the repeated
`DialectSet::parse(&self.dialect)` calls. `state.dialect: String` becomes
`state.profile: &'static DialectProfile` with a `profile.name` accessor kept for
the config / DocumentStore string round-trip (`tclLsp.selectDialect`,
`folderDialects`, the registry-dump JSON schema, `canonical_name` all stay
stable).

**Alias normalization is load-bearing** (missed touchpoint). `by_name` MUST
canonicalize `irules` and `tcl-irule` → `f5-irules`, because today
`has_fixed_ensembles(Some("irules"))=false` (`dialects.rs:1101-1102` asserts
this) and `expr_grammar_base_version` has **no `"irules"` arm** (`dialects.rs:970`
only matches `"f5-irules"`), while `is_irules_dialect(Some("irules"))=true`
(`dialects.rs:848`). Without canonicalization the profile predicates silently
disagree with the legacy `dialect == "f5-irules"` equality checks in
`lsp-server/lib.rs`. `IRULES_DIALECT` const and `dialect_from_language_id`
(`tcl-irule` → `f5-irules`) route through `by_name`.

---

## 3. Crate layering — the `tcl-dialect` foundational crate

The behaviour axis (octal / expr grammar / lexer grammar) is consumed **below**
registry (tcl-lexer, tcl-syntax) and across leaf crates (f5-xc, tcl-cli,
bigip-query-wasm) that path-dep tcl-lexer directly. Because `DialectSet` and
`TclVersion` live in tcl-registry today, those lower crates **cannot** import a
registry-hosted `DialectProfile` without a dependency cycle (§1.2) — which is
why `LexerConfig::for_dialect` and `parse_expr` are string-keyed and duplicate
the behaviour tables.

**DECIDED — Option A: introduce a foundational `tcl-dialect` crate below
tcl-lexer.** This is the chosen architecture; the whole model below assumes it.
(Option B — keeping the behaviour tables string-keyed — is recorded as a
rejected alternative in §3.1.)

### The `tcl-dialect` crate

Move `DialectSet`, `TclVersion`, the grammar structs (`LexerGrammar`,
`BracedVarStyle`), and the **`DialectProfile` catalog** into a new leaf crate
`tcl-dialect` that depends on **nothing** (like `tcl-core-types`). Then:

```
tcl-dialect (deps: none)  <- DialectSet, TclVersion, LexerGrammar, DialectProfile catalog
   ^          ^        ^
tcl-lexer  tcl-syntax  tcl-registry  ...  every layer consumes the profile
```

- `LexerConfig::for_dialect(name)` becomes `LexerConfig::from(profile.grammar)`;
  the ~10 cross-crate `for_dialect` callers (`f5-xc/translator.rs:1588`,
  `tcl-cli/commands/{transform,compile,diff,graphs,lookup,registry,diagram}.rs`,
  `tcl-cli-support/highlight.rs`, `f5-cli/commands/*`, `bigip-query-wasm`,
  `bigip-report-gen`) take a profile handle from the catalog.
- `parse_expr(source, dialect_str)` (`tcl-syntax/expr/parser.rs`) becomes
  `parse_expr(source, profile)` and reads `profile.expr_grammar_base` instead of
  its own string-keyed arm.
- registry keeps its `CommandSpec` metadata but re-exports the catalog.

**Cost (accepted):** a genuine crate split — moving `DialectSet`/`TclVersion`
out of registry touches every `use tcl_registry::…DialectSet`/`TclVersion`
import (hundreds of sites, mechanical), the `(source, dialect-string)` expr
cache key (`expr/parser.rs`) changes to a profile-identity key, and
`dialects.rs`'s detection heuristics either move too or stay in registry
re-exporting the enum. This is the **only** option that delivers the stated
single-source-of-truth for the behaviour axis, which is why it was chosen. The
split lands in its own milestone (§12, Milestone 1) ahead of any behaviour
routing, and it **unblocks** the behaviour-axis unification (§12, Milestone 3)
that a registry-hosted profile could not reach.

### 3.1 Alternatives considered — Option B (rejected)

*Rejected.* Option B kept `DialectProfile` in registry owning only the
availability axis plus the registry-level behaviour bits, while
`LexerConfig::for_dialect` and `parse_expr` **kept their own string-keyed
tables**, reconciled by a cross-crate consistency test. It was cheaper (no crate
move) but **accepted a documented split**: the behaviour axis would have two
owners kept in sync by test rather than by construction. That fails the
single-source-of-truth goal for octal / expr-grammar / lexer-grammar, so it is
not the chosen path.

---

## 4. `DialectSet` width (u64)

**The width is already u64** (`dialects.rs:34`, commit `0655f8a`); the
u16-exhaustion risk is gone (§1.4). The old "widen the backing integer"
prerequisite is **DONE**. What remains under this heading is additive:

- **New bits**: `TMSH` and a `BIGIP` bit (allocate at 15, 16…), plus headroom
  for future vendors — trivially available in u64.
- **Serialization/literals** for any *new* bit: `command_snapshot.rs` /
  `registry-dump` encode bit values in `dialects_json`; adding a bit regenerates
  that golden. The existing u64 widening already carried the width change through
  these sites, so only the new-bit encodings move.
- The combinator constants (`ALL_TCL`, `TCL85_PLUS`, `NON_IRULES_OPERATORS`, …)
  are `.bits()` unions and are width-agnostic — untouched.

New bits land with the profiles that need them (§12, Milestone 6:
`TMSH`/`BIGIP` for first-class f5-tmsh/f5-bigip), not as a standalone width
change.

---

## 5. Resolution APIs every consumer calls

One method set on `DialectProfile`, dispatching to the *correct* per-entity
semantics (the `intersects`-vs-`contains` distinction is load-bearing).

### 5.1 Availability (axis A)

| API | Replaces | Semantics |
|---|---|---|
| `p.is_available(&CommandSpec) -> bool` | `spec.supports_dialect(parse(name))` | `spec.supports_dialect(p.availability_mask)` **AND** `!p.disabled_commands.contains(name)` **AND** `spec.available_for_version(p.library_version(spec.owning_package()))` |
| `p.is_command_known(name) -> bool` | W123 `get_for_dialect` filter | mask query + disable filter over registry names |
| `p.resolve_command(&reg, name) -> Option<&CommandSpec>` | `get_for_dialect` / `resolve_spec` | the single primitive (§5.3) |
| `p.is_subcommand_available(spec, sub)` | `sub.dialects.or(spec.dialects).intersects(bit)` | intersects `p.availability_mask` |
| `p.available_subcommands(spec)` | GAP (`completion.rs` had none) | filtered iterator |
| `p.is_option_available(opt, parent_gate)` | `OptionSpec::supports_dialect` | **profile-aware — see §5.2** |
| `p.available_options(spec[/sub])` | `switch_names` / `option_specs` | membership + version guard + `library_version` |
| `p.is_var_available(spec)` / `p.available_keys(spec)` | `special_var().available_in` | intersects `p.availability_mask` |
| `p.availability_hint(spec/sub)` | `dialect_availability_suffix` | W002 hint text |
| `p.library_version(package) -> Option<&str>` | new resolver | `TracksBase`→`signature_base`; `Keyed(BigipVersion)`→`p.bigip_version`; `Keyed(ToolVersion)`→`p.tool_version`. Feeds `available_for_version` (`hover.rs:452`) |

### 5.2 Option-gating

Verified: `OptionSpec::supports_dialect(dialect, parent_dialects)` uses
`own.contains(active)` when the option sets `dialects`, else
`parent.contains(active)` — an unset option inherits the **command's** dialect
as parent (`hover.rs:422-437`). Confirmed against `expect_after`
(`commands/expect/expect_after.rs`): command `dialects = Some(EXPECT)`, options
`-re/-ex/-gl/-nocase/-i/-info` all `dialects: None`. Today `active =
parse("expect") = EXPECT` and `EXPECT.contains(EXPECT) = true`, so options
resolve.

The draft's fix ("keep `contains`, just pass one bit") is **wrong**: passing
`signature_base` as a single bit gives `TCL86` for expect, and
`EXPECT.contains(TCL86) = false`, so **every inherited option on every vendor
command silently drops**. Conversely, a core option gated `TCL85_PLUS`
(confirmed real: `switch -nocase`, `switch_.rs:138`) needs a **version** bit,
and `TCL85_PLUS.contains(IAPPS) = false`. **No single bit satisfies `contains`
for a composed `(version|vendor)` dialect.**

The resolution is a genuine **semantics change** to option availability:

```rust
// p.is_option_available(opt, parent_gate):
//   membership:  gate.intersects(p.availability_mask)   // NOT contains
//   upper-bound: opt.min_dialect_version() <= p.version_ceiling  // no 9.0-opt leak
// where `gate` = opt.dialects.or(parent_gate) (unchanged inheritance),
// and min_dialect_version() derives the lowest TclVersion bit in the gate,
// so a TCL85_PLUS option resolves under an 8.5-or-later ceiling but a
// TCL90-only option does NOT resolve in an 8.5 superset.
```

Switching `contains`→`intersects` for options is exactly why the draft's version
guard is *also* needed: `intersects` alone would leak a `TCL90`-only option into
an 8.5 profile whose mask contains… nothing 9.0, so in practice the
`version_ceiling` guard is what prevents the leak the review flagged. This
change moves the **switch / regsub / option-arity** goldens (options that were
`contains`-gated flip visibility under vendor masks). Budget those golden
regens in the milestone that lands it (§12, Milestone 4) — **not** Milestone 2.

### 5.3 The single spec-selection strategy

Two divergent rules exist: `get_for_dialect` = **last-match**
(`registry.rs:405`, `.iter().rev().find`); `command_snapshot::resolve_spec` =
**most-specific / min dialect-set size** (`command_snapshot.rs:364-380`, backs
the golden `tcl registry-dump` snapshots that caught the dialect-membership
and version-arm gaps).
`p.resolve_command` must pick **one**.

**Decision: adopt `resolve_spec`'s most-specific rule** as the single
`resolve_command` primitive (it is strictly more principled — "best spec for
this profile" — and is already golden-tested), and route `get_for_dialect`
callers through it. This regenerates registry-dump goldens; do it in its own
milestone (§12, Milestone 5) with review, **not** folded into the W123 fix.

### 5.4 Behaviour (axis B)

| API | Replaces |
|---|---|
| `p.leading_zero_is_octal()` | `registry.leading_zero_is_octal()` (`registry.rs:349`) **and** the duplicate `tcl_expr_eval::leading_zero_is_octal` |
| `p.expr_op_available(op)` | `w003_gates` via `expr_grammar_base_version` (TIP 201 `in`/`ni` ≥8.5; TIP 461 `lt/le/gt/ge` ≥9.0) |
| `p.mathfunc_available(name)` / `p.mathfunc_ceiling()` | `math_func_ceiling_for_dialect` (single source for const-fold + W002) |
| `p.runtime_version() -> Option<TclVersion>` | `TclVersion::from_dialect` (`hooks.rs:420`) |
| `p.grammar()` | `LexerConfig::for_dialect` (`lexer.rs:213`) |
| `p.has_fixed_ensembles()` / `p.is_irules()` / `p.operators_as_commands()` / `p.tcloo()` | the open-coded `matches!(dialect, Some("irules"|"f5-irules"))` copies (side_effects, manager, taint) and `minify.rs:2164` |
| `p.effective_tcl_version()` | the version the argument-DSL validators consult (§6) |
| `p.vm_runtime_version` | runtime execution release; matches `runtime_base`, or `V9_0` for an inert profile |

---

## 6. The granularity ladder — the argument-DSL rung

Dialect gating today reaches **command → subcommand → option** depth only.
Verified: `version_gate.rs` (W135/W136/W139/W144) records a `Lifecycle` at the command
head and at each option token, checked against the resolved `package require`
**floor** (`version_gate.rs:19-116`). It never descends into an argument's
mini-language.

But dialect/version differences reach **into argument mini-languages**, and that
rung is a **GAP**:

- **`binary scan` / `binary format` / `format` / `scan` format strings**: the
  `u` (unsigned) modifier is **8.5+** and invalid on 8.4; conversion specifiers
  differ across versions. Verified the spec parsers are version-**agnostic** and
  live **below registry**: `tcl-syntax/src/format.rs::parse_spec(fmt, i) ->
  Option<Spec>` and `tcl-syntax/src/scan.rs::parse_conversion` take **no**
  dialect/version — their own docstrings say "version-aware const-folding …
  stays with each consumer." Nothing gates `u` by version.
- **`string is` classes**: class set is version-dependent.
- **`regexp`/`subst` flags**, **`clock format` specifiers**, **`expr`
  operators/functions** (already partly gated by W003 via
  `expr_grammar_base_version`).

### 6.1 The ladder

```
command            e.g.  lmap            gated by availability_mask   (W123/W002)
  subcommand       e.g.  dict getwithdefault  gated by mask         (W002)
    option         e.g.  switch -nocase  gated by mask + version_ceiling (§5.2, W136/W004)
      argument-DSL e.g.  format %u       gated by p.effective_tcl_version()  <-- GAP (new)
```

The profile resolves the **effective Tcl version** the DSL validators consult:

```rust
impl DialectProfile {
    /// The Tcl version an argument mini-language (format/scan/string is/…) must
    /// validate against — the runtime_base, raised to any package floor the
    /// caller supplies. Permissive (None) for PLAIN_TCL and non-Tcl profiles.
    pub fn effective_tcl_version(&self, package_floor: Option<TclVersion>) -> Option<TclVersion> { … }
}
```

### 6.2 Scoping

The DSL validators are their **own milestone** (§12, Milestone 7), not part of
the availability fix. They are new diagnostics (a `format %u on 8.4` warning, a
`string is` class warning) built on `p.effective_tcl_version()`. Because
`format.rs`/`scan.rs` live in **tcl-syntax below registry**, they can only
consume the profile once the DECIDED `tcl-dialect` crate (§3, Milestone 1) puts
the catalog **below** tcl-syntax — after Milestone 1 the DSL validators read
`p.effective_tcl_version()` directly from tcl-syntax with no cycle. This is why
Milestone 7 is scheduled after Milestone 1, not before.

---

## 7. Per-dialect profile table

`sig`=signature_base, `rt`=runtime_base, `oct`=leading_zero_is_octal,
`ens`=has_fixed_ensembles, `ops`=operators_as_commands,
`mask`=availability_mask (precise), `ceil`=version_ceiling. Libraries reuse
`spec.rs` `Lifecycle` + `available_for_version` — **no parallel version
machinery**.

| Profile | sig | rt | oct | tcloo | ens | ops | mask (precise) | ceil | disabled | Libraries |
|---|---|---|---|---|---|---|---|---|---|---|
| `tcl8.4` | V8_4 | V8_4 | ✓ | ✗ | ✗ | ✓ | `TCL84` | V8_4 | — | tcllib/Tk `TracksBase`, Itcl `Pinned(3.4)` |
| `tcl8.5` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | `TCL85` | V8_5 | — | tcllib/Tk `TracksBase`, Itcl 3.4 |
| `tcl8.6` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | `TCL86` | V8_6 | — | tcllib/Tk `TracksBase`, Itcl `Pinned(4.x)` |
| `tcl9.0` | V9_0 | V9_0 | **✗** | ✓ | ✗ | ✓ | `TCL90` | V9_0 | — | tcllib/Tk `TracksBase`; zipfs |
| `tcl9.1` | V9_1 | V9_1 | ✗ | ✓ | ✗ | ✓ | `TCL91` (inherits 9.0) | V9_1 | — | as 9.0 |
| **`f5-irules`** | **V8_4** | **V8_4** | ✓ | ✗ | **✓** | **✗** | **`IRULES` (bare!)** | V8_4 | **42 (K36322151)** | `f5-irules-cmds` `Keyed(BigipVersion)`; **8.4 pinned forever — dict/lassign(8.5), lmap/throw(8.6), zipfs(9.0) NEVER present at ANY BIG-IP version** |
| **`f5-iapps`** | V8_5 | V8_5 | ✓ | ✗ | ✓ | ✓ | **`TCL85\|IAPPS`** (W123 fix) | V8_5 | **none** (host Tcl 8.5.13; exec/file/socket allowed) | `f5-iapps-cmds` `Keyed(BigipVersion)`, tcllib `TracksBase`. Has dict/lassign; NO lmap/8.6 |
| `f5-tmsh` | V8_5 | V8_5 | ✓ | ✗ | **✗** | ✓ | `TCL85\|TMSH`\* | V8_5 | none | `f5-tmsh-cmds` `Keyed(BigipVersion)`, tcllib `TracksBase` |
| `f5-bigip` | **None** | **None** | **Inert** | ✗ | ✓ | ✗ | (config parser — no Tcl surface) | None | — | `f5-bigip-schema` `Keyed(BigipVersion)` |
| `expect` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | **`TCL86\|EXPECT`** | V8_6 | none | Expect `Pinned(5.45.4)`, tcllib/Tk `TracksBase` |
| `synopsys-eda-tcl` | V8_6 | V8_6 | ✓ | ✓ | ✗ | ✓ | **`TCL86\|SYNOPSYS`** | V8_6 | none | sdc `Keyed(SdcVersion)`, synopsys-dc `Keyed(ToolVersion)`, tcllib `TracksBase` |
| `cadence-eda-tcl` | V8_6 | V8_6 | ✓ | ✓ | ✗ | ✓ | **`TCL86\|CADENCE`** | V8_6 | none | sdc `Keyed(SdcVersion)`, cadence-genus `Keyed(ToolVersion)` |
| `xilinx-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | **`TCL85\|XILINX`** | V8_5 | none | sdc `Keyed(SdcVersion)`, vivado `Keyed(ToolVersion)` |
| `intel-quartus-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | **`TCL85\|QUARTUS`** | V8_5 | none | sdc, quartus `Keyed(ToolVersion)` |
| `mentor-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | **`TCL85\|MENTOR`** | V8_5 | none | sdc, questa `Keyed(ToolVersion)` |
| `bpf` | **V9_0** | **V9_0** | **✗** | ✓ | ✗ | ✓ | **`TCL90\|BPF`** | **V9_0** | — | bpf-tcl `Pinned` |
| `PLAIN_TCL` (unknown) | permissive | permissive | **Inert** | permissive | ✗ | ✓ | **`ALL_TCL`** (§1.3/§8) | None | — | — |

\* `f5-tmsh` and `f5-bigip` have **no `DialectSet::parse` bit today** (they
collapse to plain Tcl — verified: `parse` has no arm for them, `dialects.rs:866`).
Giving them real profiles requires a new `TMSH` bit (u64 has headroom, §4) or an
explicit `base_layers` arm in `load_dialect` (currently no arm). This is a
**user-visible, bidirectionally-regressive** change (§7.2) — gate behind
Milestone 6.

### 7.1 Derivation rules

- `leading_zero_is_octal = if runtime_base is None { Inert } else { runtime_base < V9_0 }`
  — the `Inert` branch (f5-bigip, and any non-Tcl profile) is explicit, **not**
  a silent `false`/`true` default (hole, §11.1).
- `expr_grammar_base = runtime_base` (None → the validators return only the
  dialect-invariant subset, matching `from_dialect`'s None contract,
  `hooks.rs:416-418`).
- `tcloo` is **explicit per profile**, invariant-tested against the mask
  (§11.2): iApps 8.5 (OFF), expect 8.6 (ON), iRules OFF.
- `operators_as_commands` false for **both** `f5-irules` and `tk`
  (`dialects.rs:99-111` `NON_IRULES_OPERATORS` encodes exactly this exclusion).
- `has_fixed_ensembles` is exactly `{f5-irules, f5-iapps, f5-bigip}` — **NOT
  f5-tmsh** (drives `minify.rs:2164` prefix-shortening; a wrong `true`
  mis-minifies). Matches `DialectSet::has_fixed_ensembles` (`dialects.rs:860`).
- Default `bigip_version` / `tool_version` / `sdc_version` = **the OLDEST
  supported version** (owner-final, D5 — the *conservative* choice, reversing the
  draft's "latest" recommendation). By default only **floor-version** commands
  are offered; newer F5/EDA commands (a `CMP::*`/`HTTP2::*` introduced in a later
  TMOS, a newer synthesis-tool subcommand) are **hidden until the file pins a
  newer version**. Rationale: a default of "latest" would silently mark
  genuinely-unavailable commands as known on older targets; "oldest" never
  over-reports availability. A file must explicitly pin a newer BIG-IP/tool
  version (via `package require`, a config directive, or the `--bigip-version`
  CLI flag) to surface newer commands.
- `leading_zero_is_octal` for `bpf` = **No**: bpf's `runtime_base = V9_0` and
  Tcl 9.0 dropped bare-leading-`0` octal (TIP 114/472), so `0NN` is decimal
  under bpf — consistent with `runtime_base < V9_0` being false.

### 7.2 The f5-tmsh / bpf reverse-regression

Today `parse("f5-tmsh")`/`parse("bpf")` → `None` → W123/W002
`unwrap_or(ALL_TCL)`, so **all** Tcl commands are "known" in those files. Giving
`f5-tmsh` the precise mask `TCL85|TMSH` and `bpf` a base makes **8.6/9.0 core
commands newly draw W123** in tmsh/bpf files — a false-positive surface on
*general Tcl commands*, not just "tmsh:: stops drawing W123". This is a real
regression to budget in Milestone 6's golden/test set, not a footnote.

**Not dialects — model as `LibraryPin`, not profiles:** `tk` (express as
`Tk TracksBase` on a Tcl profile — `wish` = Tcl base + Tk; the standalone `TK`
bit stays only for the grammar layer), `itcl`, `tcllib`, `argparse`,
`ticklecharts` (all already libraries via `required_package`).

---

## 8. Unified unknown-dialect fallback

`PLAIN_TCL` is the single sink for every unparseable/typo string.
`by_name(unknown) -> &PLAIN_TCL` with:

- `availability_mask = ALL_TCL` (preserves W123/W002 "typo flags nothing").
- `version_ceiling = None`, `leading_zero_is_octal = Inert`, mathfunc
  permissive, `expr_grammar_base = None`, grammar = modern-9.x
  (`LexerConfig::default()`).

Migration note (goldens): **W123/W002 don't move** (they already used
`ALL_TCL`). **`command_snapshot` moves** — it used `unwrap_or(TCL86)`
(`command_snapshot.rs:426,433`), so an unknown-dialect dump changes from a TCL86
view to an ALL_TCL view; regenerate that golden and confirm it is the intended
unification (or, if a snapshot for an *unknown* dialect is nonsensical, make
`command_snapshot` reject unknown names up front — owner's call, but the
fallback must be **one** value, documented here).

---

## 9. Reconciling `dialects: Option<DialectSet>` + the iRules subtractive trap

**Decision: KEEP the per-command `dialects: Option<DialectSet>` field as the
intrinsic native-version/native-layer tag.** The profile computes the query
mask; the data is not migrated wholesale. `None` = universal 8.4 core;
`supports_dialect(intersects)` against a profile-supplied mask composes cleanly.

**One targeted data migration — the `NON_IRULES_OPERATORS` split:**

Today the 42 K36322151 iRules-disabled commands **and** the math-operator
command heads are both encoded by tagging them `NON_IRULES_OPERATORS` (=
`ALL_TCL | IAPPS|EXPECT|SYNOPSYS|CADENCE|XILINX|QUARTUS|MENTOR`, which **contains
`TCL84`**; `dialects.rs:107-111`). This conflates two facts and is the root of
the subtractive trap:

1. **The 42 disabled commands** (`exec`, `file`, `socket`, `open`, `glob`,
   `source`, `cd`, `pwd`, `fconfigure`, `fcopy`, `gets`, `read`, `close`,
   `exit`, `vwait`, …): ordinary universal 8.4 core. Retag `dialects` → `None`
   and move the iRules exclusion into `f5-irules.disabled_commands`. Then
   `p.is_available` for iRules is `signature_base(8.4) MINUS disabled_commands`
   — never `TCL84|IRULES`.
2. **The math-operator heads** (`+`, `eq`, `tcl::mathop::*`): excluded from
   iRules *and* tk because operators there live only inside `expr`. Model as
   `operators_as_commands: bool` (false for `f5-irules`, `tk`); drop
   `NON_IRULES_OPERATORS` from their tag. `commands_for_event` and the tk
   surface read the toggle.

### 9.1 Why the general widen-fix is WRONG for iRules

`TCL84|IRULES` re-includes `exec`/`file`/`socket` because those disables carry
`NON_IRULES_OPERATORS` (which contains `TCL84`), so `intersects(TCL84)` matches
them. The widen-to-`(base|vendor)` fix that repairs iApps/expect/EDA is exactly
wrong for iRules. Therefore `f5-irules.availability_mask` is the **bare `IRULES`
bit** plus an explicit `disabled_commands` list; `p.is_available` applies the
disable filter after the mask query. **The shipped bare-IRULES highlight fix is
correct and final and is preserved by this model** (it is literally
`f5-irules.availability_mask = IRULES`).

### 9.2 The re-inclusion hazard and its gate

After step 1 retags the 42 commands to `None`, **only** `disabled_commands`
removes them; the bare `IRULES` mask now **matches** them (`None ⇒
supports_dialect true`). **Any** availability path that queries the mask without
applying the disable filter re-admits `exec`/`file`/`socket` under iRules.

Verified un-migrated hazard: the CLI snapshot `command_names()` filters solely
by `get_for_dialect` (`command_snapshot.rs:411-414`) and is **not** in the LSP
migration set; several `get_for_dialect(IRULES)` callers exist.

**Pre-retag gate (blocking Milestone 5):** *"No consumer resolves iRules
availability via a bare mask query."* Before the retag lands, enumerate and
migrate **every** `get_for_dialect` / `supports_dialect(IRULES)` caller to apply
`disabled_commands`:

- W123 (`unresolved.rs:164`), W002 (`validity.rs:561,941`),
- completion, semantic-tokens, hover, `resolve_command`,
- **the CLI snapshot** `command_names` + `resolve_spec`
  (`command_snapshot.rs:411,427,437`),
- every other `get_for_dialect(…, IRULES)` / `resolve_dialect("f5-irules")`
  call across the 22 crates (enumerate from the complete inventory in §15,
  filtered to the IRULES/`get_for_dialect`/`resolve_dialect` columns).

Until this gate is green the retag **must not** land — an un-migrated consumer
would silently allow the banned commands.

---

## 10. Precise vs coarse masks (static grammars)

Tree-sitter / tmLanguage queries are static-per-filetype; over-approximation is
intentional (`gen_zed_queries` iApps uses `ALL_TCL|IAPPS`, pulling in 8.6/9.0
the real 8.5.13 base lacks, because precise per-version correctness is the LSP
semantic-token layer's job). The profile exposes **two** projections:

- `availability_mask` — precise (CLI, LSP, diagnostics, completion). iApps =
  `TCL85|IAPPS` exactly.
- `grammar_union` — coarse over-approx (static grammars only). iApps =
  `ALL_TCL|IAPPS`, preserving first-paint highlighting of 9.0 commands.

`gen_zed_queries::targets()` stops composing four literal unions and instead
**names profiles** (`Target{ profile: "tcl" | "f5-irules" | "f5-iapps" |
"expect" }`), taking `profile.grammar_union` for the static buckets and
`profile.registry()` for the layered command list.

---

## 11. Behaviour-axis None paths and the tcloo invariant

### 11.1 None paths are explicit, not defaulted

`leading_zero_is_octal = runtime_base < V9_0` and `expr_grammar_base =
runtime_base` are **ill-defined** for `f5-bigip` (`runtime_base = None`) and for
`tk`/`bpf`. The `Ternary::Inert` variant (§2.3) makes the non-Tcl case inert:
octal/expr validators short-circuit to "no opinion" rather than silently reading
octal or decimal. §7.1 states the None branch explicitly.

### 11.2 tcloo bool must be invariant-tested against the mask

The profile sets `tcloo` per-dialect, but hover/completion/oo-handler **also**
resolve `oo::*` specs through the mask (gated ~`TCL86_PLUS`). For the table
values they agree (iApps `TCL85` excludes `oo::`; expect `TCL86` includes it),
but nothing enforces it — a future profile with `tcloo` inconsistent with its
mask yields contradictory oo behaviour vs hover. **Add an invariant test:** for
every profile, `p.tcloo == p.availability_mask.intersects(TCL86_PLUS)` (or the
documented exception list), so the hand-filled field can never drift from the
mask-resolved `oo::*` availability.
