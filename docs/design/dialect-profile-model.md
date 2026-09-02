# The compositional `DialectProfile`

> **Status (2026-08-27): superseded in part by #1631, and still shipping.**
> The four-layer model of
> [dialect-and-package-registry-redesign.md](dialect-and-package-registry-redesign.md)
> landed through P6: environments, core profiles, surface declarations and
> realm binding knowledge are the model every consumer now resolves
> through, and the string-boundary resolvers this document described
> (`by_name`, `by_opt_name`, `resolve_known`, `availability_for_name`) are
> **deleted** — `cargo xtask retired-api-gate` fails the build if they
> reappear. What survives, and what this document remains the reference
> for, is the interned `&'static DialectProfile` catalogue itself: the seam
> still supplies `DialectProfile::find` and the named handles, and the
> lexer still builds its `LexerConfig` from a profile's grammar. The
> availability bitmask is gone: a profile states its **point** —
> `DialectProfile::surface_query` — and a spec states its **surface**, as
> `SpecSurface` rows. That residue is retirement-ledger row **C1**,
> narrowed to the interned catalogue the lexer is keyed on (redesign §11,
> D5). Read this document for how the shipping catalogue behaves; read the
> redesign for the model it answers to.

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
dialect meaning from a name and per-spec `supports_dialect` — and every
reconstruction gets one of two things wrong.

**A vendor name alone is not a dialect.** Reading `f5-iapps` as "the iapps
surface" excludes the real Tcl core it embeds, so 8.4 commands (`dict`,
`lassign`, `apply` at their own releases) would be wrongly unavailable
under `f5-iapps`, `expect`, and the EDA shells. The composed **point** is
the fix — a family at a release, plus packages — and it has to be composed
**once**, in one place, because W123 (`unresolved.rs`), W002
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
tcl-dialect (deps: bitflags only)        <- SpecSurface, TclVersion, LexerGrammar,
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

That is what the foundational `tcl-dialect` crate (§3) is: the surface
vocabulary, `TclVersion`, the grammar structs, and the `DialectProfile`
catalogue live in
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

The choice is the permissive one — the fallback's point is the whole Tcl
ladder, with a permissive behaviour axis (octal `Inert`, no version
ceiling, no expr grammar opinion) — so a typo flags nothing, which is the
highest-visibility behaviour in W123/W002. §8 is where that unification
lives. (The retired `special_vars::resolve_dialect` was the one site that
did not go through `by_name`; the `tk` ingress profile carries `Tk` in its
own point now.)

### 1.4 Availability is rows and a point, not a bitmask

A spec **states** its availability as `SpecSurface` rows — a provider
(`Core(Family)` or `Package(name)`) over version windows. A profile
**asks** at a `SurfaceQuery` point — which family at which release, with
which packages. Adding a dialect is additive: name a family or a package,
and thread the versioned-library dimension (§4). EDA shells are modelled
as a base Tcl release plus `required_package`-gated command libraries
([eda-library-packages.md](eda-library-packages.md)), so they add
nothing to the availability vocabulary at all.

---

## 2. Core model

The profile *produces* the point every availability query asks at, plus
the operator-head filter and the version guard a bare point cannot
express.

### 2.1 Two deliberately-separate base versions

- **`signature_base`** — the Tcl version whose command/subcommand/option
  *signatures* the dialect exposes (axis A). Feeds the profile's point.
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
    /// The provider this dialect's own command surface is authored under,
    /// if it has one (`Core(F5Irules)`, `Package("iapps")`, …). None for
    /// the plain Tcl-version profiles, the EDA shells, and the fallback.
    pub vendor_surface: Option<SpecProvider>,
    /// The packages this profile's own point carries — its vendor package,
    /// or `Tk` for the `tk` ingress profile. Empty for plain Tcl: Tk there
    /// needs a `package require`.
    pub surface_packages: &'static [&'static str],
    /// The command surfaces `load_surface` applies, in order. A plain Tcl
    /// version's layer carries no specs — it records which release the
    /// registry is. Empty only for the fallback profile.
    pub base_layers: &'static [SurfaceLayer],
    /// Coarse over-approximating provider list for STATIC grammars
    /// (tree-sitter / tmLanguage). Deliberately wider than the point; §10.
    pub grammar_union: &'static [SpecProvider],
    /// UPPER-BOUND version guard: the highest Tcl version whose options may
    /// appear. Distinct from the point so an option gated tcl9.0-only cannot
    /// leak into an 8.5-superset profile whose point sits inside it (§5.2).
    pub version_ceiling: Option<TclVersion>,
}
```

The point itself is derived, not stored — `DialectProfile::surface_query`
composes it from `signature_base` (or the vendor family, for a core-family
vendor surface) and `surface_packages`:

```rust
pub fn surface_query(&self) -> SurfaceQuery<'static>;
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
    pub tcloo: bool,                        // explicit; invariant-tested vs the point (§11.2)
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
    pub numbers: NumberSyntax,             // Tcl84 / Tcl85 / Tcl90 / Jim / Jim080
    pub escapes: EscapeSyntax,             // Tcl84 / Tcl86 / Tcl90 / Jim

    // ---- the five JimTcl lexical axes (§Jim below) ----
    pub word_separators: WordSeparators,   // is \v a separator? Tcl vs Jim
    pub brace_backslash_newline: BraceBackslashNewline, // {a\<nl>b} — Folds vs Literal
    pub quote_termination: QuoteTermination,  // "abc"def — Strict vs Concatenating
    pub var_syntax: VarSyntax,             // $(...), nested index parens, high bytes
    pub list_parse: ListParse,             // malformed list text — Strict vs Lenient
}

/// Three-valued so f5-bigip (runtime_base=None, "not Tcl") is INERT, not
/// silently defaulted to octal/decimal (§11.1).
pub enum Ternary { Yes, No, Inert }
```

`JimTcl`'s parser is a reimplementation rather than a fork, so it differs from
every C Tcl release on five further lexical points, each measured on
interpreters built from the upstream tags 0.76-0.84 against tclsh 8.6 / 9.0:

* `word_separators` — Jim's script parser has no `case '\v'`
  (`JimParseSep`, jim.c:1338), so `eval "f a\vb"` passes **one** argument
  where tclsh 8.6 passes two. Command parsing only: Jim's *list* parser still
  uses `isspace()`, so `llength "a\vb"` is 2 in both.
* `brace_backslash_newline` — `{a\<newline>b}` keeps its bytes in Jim
  (`JimParseSubBrace`, jim.c:1444) rather than folding to a space, deliberately,
  to preserve line numbers. Distinct from `brace_line_continuation`, which is
  the F5 fork's next-line-`{` rule: that one decides where a *command* ends,
  this one what a *word* contains.
* `quote_termination` — Jim has no extra-characters-after-close-quote check
  anywhere, so `puts "abc"def` prints `abcdef`. The brace twin still fires:
  Jim rejects `{abc}def` exactly as C Tcl does, so that diagnostic stays
  unconditional.
* `var_syntax` — three `$` divergences present in every release: `$(...)` is
  expression substitution (its own token kind, since the body is an
  *expression* and must not be analysed as a script), index parens nest, and a
  name may hold any byte >= 0x80.
* `list_parse` — malformed list text never errors. All four `ListError`
  cases become values: an unterminated `{`/`"` runs to the end of the string
  (`llength "a {b"` is 2, and its second element is `b`), while junk after a
  closing delimiter *begins the next element* (`a {b}c` is three elements).
  `tcl_syntax::list::split_list_jim` implements this; it is distinct from the
  pre-existing `split_list_lenient`, which returns only the elements before
  the malformed tail and is a best-effort partial, not Jim's answer.

`brace_backslash_newline` moves real bindings, and the two word kinds resolve
the surviving `\<newline>` differently because only one has a `name ?default?`
level: `proc p {a b\<newline>c}` binds three parameters under Tcl and **two**
under Jim (the second element is the specifier `b c`, so the parameter is `b`
defaulting to `c`), while `foreach {a b\<newline>c}` binds a variable really
named `b c`.

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
construction — a `static` array of 18 profiles cannot run derivation code — so
the invariant tests in `profile.rs` (§7.1, §11.2) are what hold the derived
fields consistent with their bases.

### 2.4 Catalog + resolution boundary

```rust
impl DialectProfile {
    pub fn all() -> &'static [DialectProfile];             // the catalog, sorted-name order
    pub fn by_name(name: &str) -> &'static DialectProfile; // alias-normalised;
        // unknown -> PLAIN_TCL (whole-ladder point, behaviour permissive; §1.3/§8).
    pub fn by_opt_name(name: Option<&str>) -> &'static DialectProfile; // None -> PLAIN_TCL
    pub fn find(name: &str) -> Option<&'static DialectProfile>; // distinguishes "unknown"
    pub fn irules() -> &'static DialectProfile;            // explicit handle for hardcoded lookups
    pub fn plain_tcl() -> &'static DialectProfile;         // the sink itself
}
```

`all()` excludes `PLAIN_TCL`: the fallback is a resolution sink, not a
selectable dialect. `find` is the form that tells "unknown" apart from a
real profile.

The string→profile resolution happens **at ingest** (LSP `dialect_for_open`
/ CLI `effective_dialect` / `detect_dialect`) and the `&'static
DialectProfile` is threaded from there, in place of repeated per-consumer
name parsing. The dialect *name* stays the wire form for the config /
`DocumentState` round-trip (`tclLsp.selectDialect`, `folderDialects`, the
registry-dump JSON schema), so `profile.name` is the accessor those paths
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
long as it resolves through **one** place immediately — and since the
resolved point landed (§2.5) that place is `grammar_of_dialect_name`, not
the catalogue. `LexerConfig::for_dialect(name)` is
`from_grammar(grammar_of_dialect_name(Some(name)))`; `parse_expr(source,
dialect: Option<&str>)` resolves a catalogue name to its profile and any
other name to its grammar, and `parse_expr_for_profile` uses the profile it
is *handed* rather than re-finding it by name. That re-find was a bypass: it
threw away a profile projected from an environment's point (`jim`, which has
no catalogue row) and parsed under the permissive fallback — the one way
codegen and the lexer could still disagree about a document after everything
else agreed.

---

### 2.5 One resolved point: how a name becomes a grammar

The review of the Jim work found that the axes had been modelled correctly
and then **not reached**: every production ingress resolved a dialect name
through `DialectProfile::find` → `plain_tcl`, so a `jim` document — which by
design has no catalogue row — was lexed, lowered, analysed and compiled as
Tcl 9.0 end to end, and the centralised resolution was only ever asked
`"tcl"`. Worse, the two catalogues (§2.4's `CATALOG` and the environment
model) disagreed about `tk` (9.x vs an 8.6 core), and once codegen followed
one while the lexer followed the other, one document had two grammars.

The settlement is a single currency and a single producer.

**The currency: `DialectPoint`** (`tcl-dialect/src/model/point.rs`). A
`Release` — which already names its `Family` — plus the build profile, with
every lexical axis a function of it (`point.grammar()` is
`grammar(family, release)`). It exists because the two currencies layers
actually carried could not say what they needed to:

- `Option<&DialectProfile>` cannot name a dialect with no catalogue profile
  (`jim`, `tk`), so every `of_profile` answered the fallback for them.
- `Option<&str>` resolves only to an environment's *default* release, so it
  cannot tell jim 0.79 from 0.80 (`0d`), tcl8.5 from 8.6 (TIP 388 widths), or
  tcl9.0 from 9.1. `DialectPoint::of_name_and_release` keeps the release;
  `of_dialect_name` takes the environment's default and says so.

`of_dialect_name` returns `None` for an environment with no Tcl ladder at all;
among the shipped dialects that is only `f5-bigip`, the BIG-IP *config*
surface, whose grammar the catalogue row alone states.

**The resolution: `grammar_of_dialect_name`** (`tcl-dialect/src/grammar.rs`)
is the one place a name becomes a grammar — the point's grammar when there is
a point, the catalogue row's for `f5-bigip`, the default otherwise. The
per-axis constructors (`NumberSyntax` / `EscapeSyntax` / `BracedVarStyle` /
`WordValueRules::of_dialect_name`) delegate to it, so they agree by
construction rather than by four copies of one lookup.

**The producer: the ingress.** `DocumentEnvironment::point()` reads the
environment definition's core selector, and `DocumentEnvironment::grammar()`
is where a document's grammar is born: the point's grammar, else the unit
profile's. Consumers that still thread a `&'static DialectProfile` are served
by `analyser_profile()` handing out a profile **projected from the point**
(`DialectProfile::projected_from_point`, interned once per environment
identity): identity fields from the environment, grammar and version bases
from the point, every *policy* field the fallback's — so nothing downstream
gains a new opinion, only the right grammar under the right name. That last
part is load-bearing: lowering hands codegen `profile.name`, and codegen
resolves *that* through `grammar_of_dialect_name`, so a unit built for
`"jim"` now reaches Jim's grammar at the back end as well as the front.

Two cored environments are deliberately **not** projected. `tk` keeps the
anonymous fallback as its analyser profile (the cache-key and help-filter
reasons on `analyser_profile`), and its grammar reaches the analyser through
`State::grammar()`, which prefers the ingress-resolved grammar over
`profile.grammar`. The lenient `tcl` sink — where the bare name, the empty
name and every unknown name land — **is** the fallback, whose identity
`is_fallback()` tests by pointer (§8); its point is 9.0, the fallback's own
grammar, so a projection would change nothing but that identity.

**`tk` is 8.6.** `TK_PROFILE.grammar` is now `GRAMMAR_TCL86`, matching the
`tk` environment's core, its library pins (`LIBS_TCL86_PLUS`) and the version
gating that already read that core. It said 9.x while everything else about
the row said 8.6; the disagreement predates the Jim work and was exposed by
it. Reversing the decision — a 9.0 core instead — is a one-line change to the
environment, and the agreement tests then enforce whichever is chosen.

**Codegen carries one grammar.** `CodegenCtx::expr_grammar` is the same
`LexerGrammar` its `numbers`, `escapes`, `braced_var` and `word_rules` were
taken from, and `parse_compile_expr` re-parses `expr` bodies under it.
Before, a named compile emitted numerals under a grammar resolved from the
name and re-parsed `expr` under `ctx.dialect`'s profile — for `tk`, `010` was
8 on one path and 10 on the other inside one compile.

**The word-value owner: `WordValueRules`** (`tcl-syntax/src/word_rules.rs`)
carries the two axes every site that splits a word-shaped list needs
(`brace_backslash_newline`, `list_parse`) with the algorithms keyed by them
(`collapse_braced_word`, `split_list`, `split_word_names`). Lowering, codegen's
verbatim-literal path and the taint walker ask it; none reaches for
`split_list` or `collapse_brace_continuations_str` and decides for itself.

**What enforces it.** `every_route_to_a_documents_grammar_agrees`
(`tcl-registry/src/model/ingress.rs`) asserts, for every compiled environment
with a ladder, that `env.grammar()`, `grammar_of_dialect_name(id)`,
`unit_profile().grammar` and (bar `tk`) `analyser_profile().grammar` are one
value, and that the profile carries the environment's own name.
`a_jim_unit_is_built_as_jim_end_to_end` (`tcl-compiler/src/compilation_unit.rs`)
builds a unit for the *string* `"jim"` and asserts the name codegen receives.
`the_two_catalogues_agree_wherever_both_know_a_name` (`grammar.rs`) includes
`TK_PROFILE`, the row `all()` excludes and the one that drifted.
`from_grammar_carries_every_lexer_axis` (`tcl-lexer`) pins each `LexerConfig`
field against a grammar in which every axis is non-default — the
`..Self::default()` tail is where `list_parse` was once declared and carried
by nothing.

**Deliberately not done here**, each a follow-up rather than a quiet
omission: the VM's list conversions (`Value::as_list`, `cmd_prefix`,
`interp`, `expr`) and `signature_scan::parse_param_list` are public APIs with
no dialect parameter and still split strictly — the VM is Tcl-only and should
say so; the ~30 `Lexer::new` sites outside `tcl-lexer` re-lex bodies under the
default config regardless of the document's dialect (wrong for F5 today, not
only for Jim); the `Option<&DialectProfile>` currency in taint, GVN and the
optimiser cannot name `jim`/`tk` and should become a `DialectPoint`;
`ExprSugar` (`$(…)`) is reconstructed by the segmenter, formatter and
minifier but not yet lowered to an expression evaluation; and a
`dialect-drift` xtask gate, modelled on `number-drift`, should ban the
bypass spellings (`DialectProfile::find(…).grammar`, bare `split_list` /
`collapse_brace_continuations_str` outside their owners, new
`Option<&DialectProfile>` signatures) so the next axis cannot repeat this.

---

## 3. Crate layering — the `tcl-dialect` foundational crate

The behaviour axis (octal / expr grammar / lexer grammar) is consumed **below**
registry (`tcl-lexer`, `tcl-syntax`) and across leaf crates that path-dep
`tcl-lexer` directly. A registry-hosted `DialectProfile` would therefore be
unimportable from exactly the crates that need it most (§1.2).

### The `tcl-dialect` crate

`SpecSurface` / `SurfaceQuery`, `TclVersion`, the grammar structs
(`LexerGrammar`, `BracedVarStyle`), `LibraryPin` / `LibraryVersion` /
`VersionKey`, and the `DialectProfile` catalogue live in
`rust/tcl-dialect`, a leaf crate whose only dependency is `bitflags`:

```
tcl-dialect (deps: bitflags)  <- SpecSurface, TclVersion, LexerGrammar, the catalogue
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

The cost is a real crate boundary: the surface vocabulary and `TclVersion`
are imported from `tcl_dialect`, not `tcl_registry`, at every site. That is the price of
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

## 4. Adding a dialect is additive

Availability is rows and a point (§1.4), so there is no width to exhaust
and nothing to renumber:

- **A new family** — `Family` gains a variant with its release ladder,
  and specs say `available {family WINDOW}`.
- **A new vendor package** — nothing in the availability vocabulary
  changes at all; the environment places the package and specs name it.
- **Serialisation**: `command_snapshot.rs` / `registry-dump` project rows
  onto the catalogue's dialect ids through the one projection
  (`tcl_registry::model::surface::dialect_names_for_rows`), so a new name
  regenerates that golden and nothing else.

A family lands with the profile that needs it, never as a standalone
change. `f5-tmsh` and `f5-bigip` are the worked examples: they exist
because they are first-class profiles (D8), each with a precise point
rather than a collapse to plain Tcl.

---

## 5. Resolution APIs every consumer calls

One method set, dispatching to the *correct* per-entity semantics (the
`intersects`-vs-`contains` distinction is load-bearing). The availability
queries need `CommandSpec`, which lives above the foundational crate, so they
are a **trait** — `tcl_registry::ProfileQueries`, implemented for
`DialectProfile` — rather than inherent methods. The behaviour queries (§5.4)
are inherent on `DialectProfile` itself.

### 5.1 Availability (axis A)

Asked of the document's `ResolvedContext`; the profile-keyed forms below
are the same questions at a profile's own point.

| API | Semantics |
|---|---|
| `is_available(&CommandSpec) -> bool` | `spec.supports_dialect(point)` **AND** `operator_heads_are_commands() \|\| !spec.traits.contains(OPERATOR_COMMAND)` **AND** `required_package_available(spec.required_package)` |
| `resolve_spec(&reg, name) -> Option<&CommandSpec>` | the single availability primitive W123 / W002, completion, and the CLI snapshot share: `reg.get_for_surface(name, point)` filtered by `spec_available` (§5.3) |
| `subcommand_available(spec, sub)` | `sub.surface.or(spec.surface)` admits the point; a `None` gate on both means no restriction |
| `p.available_subcommands(spec)` | the filtered subcommand list, in declaration order |
| `p.is_option_available(opt, parent_gate)` | **profile-aware — see §5.2** |
| `p.available_option_names(spec)` / `p.available_option_specs(spec)` | the profile-aware `switch_names` / option table, declaration order, duplicates removed |
| `available_sub_option_names(spec, sub)` / `available_sub_option_specs(spec, sub)` | the same for a subcommand's options, which inherit `sub.surface.or(spec.surface)` as their parent gate |
| `p.find_option(spec, name, package_version)` | option lookup by canonical name or alias, honouring §5.2's gate and the resolved package version |
| `p.vendor_surface(&reg) -> Option<VendorSurface>` | this profile's own vendor commands, grouped by `NS::` prefix, sorted by descending size then name. `None` for a profile with no vendor surface. Feeds generated consumers (the AI prompt's F5-surface summary) so prose cannot drift from data |
| `p.keyed_version_range(spec)` | the declared introduction and removal releases of `spec` on this profile's keyed library axis, or the axis baseline when none is recorded. `None` for a spec outside a keyed pin |
| `p.keyed_pin_for(spec)` | the ambient `Keyed` `LibraryPin` `spec` belongs to under this profile |

Special-variable availability asks the same point through
`SpecialVarSpec::available_in`.

The library-version resolvers are inherent on `DialectProfile`:
`library_pin(package)`, `is_ambient_package(package)`, `library_floor(…)`,
`library_floor_default(package)`, and `hosts_tk()`.

### 5.2 Option-gating

An option inherits its parent's gate when it declares none: `gate =
opt.surface.or(parent_gate)`, where `parent_gate` is `spec.surface` for a
command option and `sub.surface.or(spec.surface)` for a subcommand option.
`expect_after` (`commands/expect/expect_after.rs`) is the worked case: the
command names the `expect` package and its `-re` / `-ex` / `-gl` /
`-nocase` / `-i` / `-info` options state no surface of their own.

`option_available` uses two tests, not one:

```rust
// option_available(opt, parent_gate), with gate = opt.surface.or(parent_gate):
//   admits:      surface_admits(gate, point)
//   upper-bound: core_tcl_floor(gate) <= version_ceiling   // no 9.0-opt leak
// A `None` gate on both the option and its parent means "no restriction".
```

The upper bound is not optional decoration: admission alone would let a
9.0-only option into an 8.5-superset profile whose point sits inside the
gate. `core_tcl_floor` reads the lowest Tcl release the gate's rows name,
so an 8.5-and-later option resolves under an 8.5-or-later ceiling while a
9.0-only one does not. A profile with no ceiling (the permissive fallback)
accepts every release, and a gate that names no Tcl release at all (a pure
vendor gate) passes the bound unconditionally.

### 5.3 The single spec-selection strategy

`resolve_command` needs one rule for picking among several specs registered
under the same command name. The rule is **most-specific**, implemented once
in `CommandRegistry::best_visible` and reached through `get_for_surface`:

```text
among the specs visible at the query point, take the maximum of
  (spec.surface.is_some(),                    // an explicit surface beats an open one
   Reverse(surface_breadth(spec.surface)),    // then the narrowest surface
   index)                                     // then the last declared
```

"Tightest gate wins" is the principled reading — the best spec *for this
profile* — and it is what the golden `tcl registry-dump` snapshots are
written against. The declaration-index tiebreak only decides between two
equally-specific specs.

`spec_visible` is the visibility predicate `best_visible` filters on, and it
applies the same trio the context's own availability answer does, so a point query on a
profile-stamped registry and a profile-side query can never disagree. It
short-circuits to the bare `supports_dialect` result when the query is not
about the registry's own profile, because such a query is asking
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
| `p.grammar` | the lexing grammar — the point's, for a profile projected from one (§2.5); `LexerConfig::for_dialect` no longer reads it, resolving through `grammar_of_dialect_name` instead |
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
surface-driven: `version_gate.rs` records a `Lifecycle` at the command head and
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
command            e.g.  lmap            gated by the point          (W123/W002)
  subcommand       e.g.  dict getwithdefault  gated by the point      (W002)
    option         e.g.  switch -nocase  gated by point + version_ceiling (§5.2, W136)
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
`point`=`surface_query` (precise), `ceil`=version_ceiling. Libraries reuse
`spec.rs` `Lifecycle` + `available_for_version` — **no parallel version
machinery**.

| Profile | sig | rt | oct | tcloo | ens | ops | point (precise) | ceil | Libraries (all ambient unless noted) |
|---|---|---|---|---|---|---|---|---|---|
| `tcl8.4` | V8_4 | V8_4 | ✓ | ✗ | ✗ | **✗** | tcl 8.4 | V8_4 | Tk `TracksBase`, Itcl `Pinned(3.4)` — both **hosted** |
| `tcl8.5` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | tcl 8.5 | V8_5 | Tk `TracksBase`, Itcl `Pinned(3.4)` — both hosted |
| `tcl8.6` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | tcl 8.6 | V8_6 | Tk `TracksBase`, Itcl `Pinned(4.2)` — both hosted |
| `tcl9.0` | V9_0 | V9_0 | **✗** | ✓ | ✗ | ✓ | tcl 9.0 | V9_0 | Tk `TracksBase`, Itcl `Pinned(4.2)` — both hosted |
| `tcl9.1` | V9_1 | V9_1 | ✗ | ✓ | ✗ | ✓ | tcl 9.1 (inherits 9.0) | V9_1 | as 9.0 |
| **`f5-irules`** | **V8_4** | **V8_4** | ✓ | ✗ | **✓** | **✗** | **f5-irules, no release** | V8_4 | `f5-irules-cmds` `Keyed(BigipVersion)`. **8.4 pinned forever — dict/lassign (8.5), lmap/throw (8.6), zipfs (9.0) NEVER present at ANY BIG-IP version** |
| **`f5-iapps`** | **V8_4** | **V8_4** | ✓ | ✗ | ✓ | **✗** | tcl 8.4 + `iapps` | V8_4 | `f5-iapps-cmds` `Keyed(BigipVersion)`. Rides the `f5-tcl` trunk (fork of Tcl at 8.4.6 — measured, `bigip-irule-parser-measurements.md` §4a; the 8.5 hypothesis is falsified) |
| `f5-tmsh` | **V8_4** | **V8_4** | ✓ | ✗ | **✗** | **✗** | tcl 8.4 + `tmsh` | V8_4 | `f5-tmsh-cmds` `Keyed(BigipVersion)`. Same trunk and same measurement as iApps |
| `f5-bigip` | **None** | **None** | **Inert** | ✗ | ✓ | **✗** | `bigip` only (config surface, no Tcl command surface) | None | `f5-bigip-schema` `Keyed(BigipVersion)` |
| `expect` | V8_6 | V8_6 | ✓ | **✓** | ✗ | ✓ | tcl 8.6 + `expect` | V8_6 | Expect `Pinned(5.45.4)` |
| `synopsys-eda-tcl` | V8_6 | V8_6 | ✓ | ✓ | ✗ | ✓ | tcl 8.6 | V8_6 | sdc `Keyed(SdcVersion)` + 5 tool packs `Keyed(ToolVersion)` |
| `cadence-eda-tcl` | **V8_4** | **V8_4** | ✓ | **✗** | ✗ | **✗** | tcl 8.4 | V8_4 | sdc + 4 tool packs |
| `xilinx-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | tcl 8.5 | V8_5 | sdc + `vivado` |
| `intel-quartus-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | tcl 8.5 | V8_5 | sdc + 7 `quartus-*` packs |
| `microchip-libero-eda-tcl` | V8_5 | V8_5 | ✓ | ✗ | ✗ | ✓ | tcl 8.5 | V8_5 | sdc + the Libero tool packs |
| `mentor-eda-tcl` | **V8_6** | **V8_6** | ✓ | **✓** | ✗ | ✓ | tcl 8.6 | V8_6 | sdc + `questa`, `questa-formal`, `calibre` |
| `spectcl` | **V9_0** | **V9_0** | **✗** | ✓ | ✗ | ✓ | tcl 9.0 + `spectcl` | **V9_0** | — |
| `bpf` | **V9_0** | **V9_0** | **✗** | ✓ | ✗ | ✓ | tcl 9.0 + `bpf` | **V9_0** | — |
| `PLAIN_TCL` (unknown) | **None** | **None** | **Inert** | ✓ | ✗ | ✓ | the whole Tcl ladder (§1.3/§8) | None | — |

The five EDA profiles carry **no vendor surface**: their points are the bare base
version, and their tool commands are gated by `required_package` against the
ambient library pins instead
([eda-library-packages.md](eda-library-packages.md)). `f5-bigip` is the other
profile whose point names no Tcl release — its own surface alone, because
it is a configuration surface with no Tcl command surface, which is also
why its `grammar_union` names only that surface.

`grammar_union` names `Core(Tcl)` plus the profile's own vendor provider
for every profile except the two whose static grammar is deliberately
scoped tight: `f5-irules` and `f5-bigip`, which name only their own. See
§10.

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
- `tcloo` is **explicit per profile**, invariant-tested against the point
  (§11.2): it must equal what `SpecSurface::TCL86_PLUS` answers there, with
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

Giving a profile a precise point instead of letting it collapse to the whole ladder
is not a free win. `f5-tmsh` at Tcl 8.4 plus `tmsh`, and `bpf` at Tcl 9.0
plus `bpf`, means
8.5+ core commands correctly draw W123 in a tmsh file and 8.x-only relics
correctly draw it in a bpf file — but those are *general* Tcl commands, and
any modelling error in the base version shows up as a false positive across
the whole file rather than on the vendor surface alone. Precision on the base
version is therefore load-bearing for a vendor profile in a way it is not for
a plain Tcl profile.

**Not dialects — modelled as `LibraryPin`, not profiles:** `tk` (a `Tk`
`TracksBase` pin on a Tcl profile — `wish` is a Tcl base plus Tk; the
standalone `Tk` package sits in the `tk` ingress profile's own point), `itcl`, `tcllib`,
`argparse`, `ticklecharts`. `DialectProfile::hosts_tk()` is the predicate for
"can this profile `package require Tk`": true for the plain Tcl versions and
the permissive fallback, false for every closed-world vendor shell, which is
what consumers ask now that the EDA profiles carry no vendor surface.

---

## 8. Unified unknown-dialect fallback

`PLAIN_TCL` is the single sink for every unparseable or mistyped dialect
string. `by_name(unknown) -> &PLAIN_TCL`, which is deliberately permissive on
both axes:

- its point is the whole Tcl ladder and `base_layers = &[]` — nothing is
  unknown, and no surface is loaded.
- `version_ceiling = None`, `signature_base = None`, `runtime_base = None`,
  `leading_zero_is_octal = Inert`, `expr_grammar_base = None`,
  `grammar = GRAMMAR_TCL9X` (modern 9.x lexing), `help_terms = &[]` (no
  filtering).

A typo therefore flags nothing, which is the highest-visibility behaviour in
W123 / W002. `command_snapshot` resolves through `by_name` like everything
else, so an unknown-dialect `registry-dump` renders the whole-ladder view
rather than an ad-hoc one.

`PLAIN_TCL` is not in `DialectProfile::all()` — it is a resolution sink, not
a selectable dialect — so a caller that must enumerate real dialects gets the
18 catalogue entries, and a caller that must include the fallback reaches it
through `DialectProfile::plain_tcl()`. `is_fallback()` is the predicate for
"did this resolve to the sink", which is how `hosts_tk()` treats an unlabelled
`wish` shell as Tk-capable — and is why the sink is one of the two cored
environments the ingress never projects a profile for (§2.5): a projection
would carry the same 9.0 grammar under a different pointer, and every
`is_fallback()` reader would silently stop recognising the fallback.

---

## 9. `surface: Option<&[SpecSurface]>` and the iRules subtractive trap

The per-command `surface` field states where a command exists. The profile
supplies the point; `supports_dialect` asks whether any row admits it.

**iRules availability is subtractive in appearance only.** F5's TMM
interpreter removes about fifty commands from iRules — the K36322151 sandbox
bans (`exec`, `file`, `socket`, `open`, `glob`, `source`, `cd`, `pwd`,
`fconfigure`, `fcopy`, `gets`, `vwait`, …) plus the project-modelled
iRules-excluded internals. There is **no ban list** anywhere in the model.
Instead:

1. **Each banned command carries an explicit surface that does not name
   iRules** — typically `ALL_TCL`. The spec still exists, so the LSP can
   distinguish "exists, but not in iRules" from "unknown"; it simply does
   not admit the iRules point. Universal `surface: None` was eliminated
   registry-wide precisely so this works: with no universal tag left, the
   absence of an iRules row is a positive statement, not an accident.
2. **The math-operator heads** (`+`, `eq`, `tcl::mathop::*`) are excluded by
   dialect *shape* rather than by tag: a spec carries
   `Traits::OPERATOR_COMMAND` iff it is a `tcl::mathop` spelling, and
   `is_available` drops it when `operators_as_commands` is false. That is a
   separate fact from the sandbox bans and is modelled separately.

Both directions are contract-tested in
`rust/tcl-registry/tests/dialect_profile.rs`: every banned name must resolve
to registered spec data *and* must not name iRules, and
`OPERATOR_COMMAND` must mark exactly the `tcl::mathop` spellings.

### 9.1 Why the general widen-fix is wrong for iRules

A release plus a package is the right point for `f5-iapps`, `expect`, and
the EDA shells, and exactly wrong for iRules: a point naming Tcl 8.4 would
re-admit every sandbox-banned command, because a banned command's surface
names a Tcl release. The iRules point therefore names **the family and no
release** — the same thing its `grammar_union` says — and a command is in
iRules iff its own surface names iRules.

### 9.2 Why "no ban list" is the safer shape

A subtractive list has a standing hazard: any availability path that queries
the point without also applying the list re-admits `exec` / `file` / `socket`
under iRules, and there are many such paths (`get_for_surface` callers,
`resolve_dialect("f5-irules")` callers, the CLI snapshot's `command_names`)
across the consuming crates.

Encoding the exclusion in the spec's own surface removes the hazard by
construction: the point query *is* the whole test, so a consumer that
forgets the profile-side filter still gets the right answer. The one
remaining profile-level exclusion is the operator-command one, and
`CommandRegistry::spec_visible` applies it inside the registry so that
point queries and the context's own availability answer cannot
disagree.

---

## 10. Precise point vs coarse providers (static grammars)

Tree-sitter / tmLanguage queries are static per filetype, so
over-approximation is intentional: `f5-iapps` highlights against the whole
Tcl ladder plus its own package, pulling in 8.6/9.0 words the real 8.4 base
lacks, because precise per-release correctness is the LSP semantic-token
layer's job — it sees the file, the static query does not. The profile
therefore exposes **two** projections:

- `surface_query` — the precise point (CLI, LSP, diagnostics, completion).
  iApps is Tcl 8.4 plus `iapps` exactly.
- `grammar_union` — a coarse provider list, static grammars only. iApps
  names `Core(Tcl)` and `Package("iapps")`, preserving first-paint
  highlighting of 9.0 commands.

`f5-irules` and `f5-bigip` are the exceptions where the coarse projection
is *not* wider: both name only their own family. Widening iRules would
paint 8.5+ core words that genuinely do not exist there, which is worse
than under-painting.

`gen_zed_queries::targets()` names profiles rather than composing literal
lists — `plain_tcl()`, `f5-irules`, `f5-iapps`, `expect` — and takes each
target's `grammar_union` for the static buckets and its resolved context
for the command list, so the projection applies the same visibility rules
(§9) the LSP does.

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
it, derives from the surfaces it has loaded instead.

### 11.2 The tcloo bool is invariant-tested against the point

The profile sets `tcloo` per dialect, but hover, completion, and the `oo`
handler **also** resolve `oo::*` specs at the point (8.6 and later).
Nothing about the struct forces those two to agree, and a profile whose
`tcloo` contradicted its point would give contradictory `oo` behaviour and
hover text.

`tcloo_agrees_with_what_the_point_resolves` in `profile.rs` enforces it
over every catalog profile plus the fallback:
`p.tcloo == surface_admits(SpecSurface::TCL86_PLUS, Some(&p.surface_query()))`. `f5-bigip` is the one
documented exception — it has no Tcl surface at all, so `!p.tcloo` is asserted
directly rather than derived from a point that names no Tcl release.
