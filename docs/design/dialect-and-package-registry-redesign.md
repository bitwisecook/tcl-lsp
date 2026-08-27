# Dialects, packages, and environments — the registry redesign (issue #1631)

> **Status: revision 2, IMPLEMENTED through P6 (2026-08-27).** This is the
> design for the post-release architecture directed in issue #1631,
> researched on branch `claude/tcl-dialect-registry-design-lrzbsn`
> (2026-08-25). Revision 2 (2026-08-26) incorporates the
> [adversarial review](dialect-and-package-registry-redesign-adversarial-review.md)
> of revision 1: all thirteen blocking findings are **accepted** and the model
> is corrected accordingly — §0.1 records the disposition.
>
> **This document is no longer a proposal.** The owner directed full
> implementation on this branch and phases P0–P6 landed over 2026-08-26/27;
> the marked ▸ recommendations on Q1–Q25 were adopted as working rulings
> and are now shipped behaviour unless §10 says otherwise. Every phase
> entry in §8 carries its final status, §9 records what became of each
> research defect, §10 records what became of each owner question, and
> **§11 is the single ledger of everything still open** — the one place to
> look for what this programme did not finish and what each remainder waits
> on. Where the prose still describes intent rather than code it says so
> explicitly; nothing else here should be read as future tense.
>
> Where this document and [dialect-profile-model.md](dialect-profile-model.md)
> disagree, this document describes the model as built and that one
> describes the pre-#1631 `DialectProfile` catalogue, which survives as the
> interned seam ledger row C1 retires.

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
Rust-side compatibility shims**: the tk triangle,
`availability_for_name`'s union, `LanguageDialect::Set` and
`registry_for_dialect_profile` are **deleted, not wrapped** — and held
there by `cargo xtask retired-api-gate`. `TK_PROFILE` and the `DialectSet`
bits survive, not as shims but as the interned catalogue the lexer and the
executable-IR bundle are still keyed on; that is retirement-ledger row
**C1**, §11's D1, the largest remaining item of the programme.

## 0.05 Standing design principles (owner, 2026-08-27)

These govern every lane of this programme and every lane after it. They
were stated by the owner while ruling on the §11 ledger and are recorded
here because they decide questions the ledger rows only ask.

**P-A — SpecTcl declares; Rust executes.** SpecTcl is an extension that
declares command specs, and those declarations are **compiled to a
structure Rust uses very efficiently**. The tclvm is entered *only* to
execute small hooks, and only where a hook is genuinely needed (option
constraints that no declarative relation can express, literal-driven
typing, and their kin). This is the frozen-snapshot model of §6 stated as
a principle: the snapshot is the artefact, evaluation is a build step,
and analysis reads the compiled structure.

**P-B — performance in the general case, and hooks are the exception.**
The design must minimise how often a hook actually executes. Scripts must
not run on every edit: every relation the declarative vocabulary can
express must be checked natively with no VM entry, hooks must declare
their inputs so results are shape-cached, and an edit that does not change
a hook's inputs must reuse the cached verdict. When a declarative
vocabulary and a hook can both express something, the declarative form
wins — coverage of the common patterns is worth more than hook
expressiveness.

**P-C — retire unused surface.** Declared-and-unpopulated model surface is
deleted rather than carried, because a word no data uses invites packs to
guess at semantics the engine never implements (§11.1 O2, ruled
2026-08-27: delete). Anything genuinely needed later returns *with* its
consumer.

**P-D — prove it with an experiment.** Where a question is empirical,
answer it by measuring rather than by reasoning: build the interpreter,
run the probe, record the transcript, and turn it into a hermetic vector.
The F5 conformance corpus and the five `jimsh` builds are the pattern —
both found real model defects on their first run.

**P-E — follow ordinary Tcl design and patterns.** Where a surface needs
expressive power, reach for the shapes a Tcl programmer already knows
(`if`, `switch`, `foreach`, ordinary command syntax) before inventing a
mini-language.

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

## 0.2 F5 evidence review disposition

The
[BIG-IP evidence review](dialect-and-package-registry-redesign-bigip-evidence-review.md)
(2026-08-26) reviews the residual F5 assumptions that survived revision 2.
**All eight findings are accepted**, and its nine required changes gate
the F5 half of the migration:

| Finding | Disposition |
|---|---|
| F1 six language contexts | accepted — `BigIpExecutionContext` { TmmIRule, TmshCliScript, IAppImplementation, IAppPresentationApl, IAppPresentationTclCallback, HostShellTcl } keys every F5 grammar/command/variable/package/policy/evidence record; APL's `tcl` clauses get a typed embedded-range descriptor, never a whole-document language id; `f5-bigip` stops being a catch-all Tcl identity. **Landed (evidence lane, #15)**: `BigIpExecutionContext` is a real typed key in `rust/tcl-registry/src/f5/execution_context.rs` — six variants, each carrying its family, build profile, environment name and core profile, with the two APL contexts answering `None` on every one of them and `promotes_facts_to` refusing every cross-context substitution. APL's own `is_tcl()` is `false`, so an APL range can never route into the Tcl registry. Still open in F1: the typed **embedded-range descriptor** for APL's `tcl` clauses — a document/range concern for the compiler and server rather than a registry one — and the `f5-bigip` identity split |
| F2 unprovenanced releases | accepted, with the owner-attested fork fact now measured: **iRules branched off Tcl 8.4.6 and evolved independently from that point** (owner ruling, 2026-08-26), and the iApps/tmsh "8.5"/"8.5.13" claims are **measured and falsified** — both report 8.4.6 and both carry the fork grammar ([measurements](bigip-irule-parser-measurements.md) §4a), so the fork is keyed to the *appliance*, not to the iRules family (the 8.5.13 on-box binary is the unrelated host `/usr/bin/tclsh`). The F5 tree is therefore (owner rulings, 2026-08-26): `tcl@8.4.6` → family `f5-tcl` (the shared trunk, ladder keyed by TMOS release) → family `f5-irules` (a dialect offshoot of the trunk carrying its own parse-level fingerprint), with `f5-iapps` and `f5-tmsh` as environments riding the trunk directly and `f5-irules` the environment riding the offshoot (§2); post-fork deltas per TMOS release come from the evidence corpus. `EmbeddedRuntimeEvidence` records are the truth; unmeasured BIG-IP releases resolve to `Unknown` or an explicitly labelled nearest-known **assistance** profile; the two APL contexts stay `Unknown`. **Landed (evidence lane, #15)**: `rust/tcl-registry/src/f5/evidence.rs` carries `(context, build, fact, provenance)` records seeded from the 21.1.0.1 run — patchlevel per context (including `tcl_patchLevel` **unset** in `TmshCliScript`), the three-way `tcl_platform` split, the 16 discriminators in the two contexts §4 probed them in (four in TMM, and *only* the four the parity list exercised), §4b's 31 command-class rows, and the traffic lab's priority policy. The semantic door (`measured_fact`) is exact-build-only; the assistance door (`assistance_fact`) returns a labelled `NearestKnownAssistance` carrying the build it was really measured on, and neither door crosses a context boundary |
| F3 `}{` overclaim | **answered by live measurement** ([measurements](bigip-irule-parser-measurements.md) §3): the six-row matrix ran on TMM 21.1.0.1 with same-host stock controls. The separator is generic (`list {a}{b}` and `set x {a}{b}` both split), lexical (stays lexer data, no per-command grammar, no BIG-IP lifecycle), and gated on the word having *started* with `{` or `"` (`if{1}{…}` stays one bare word). `{*}` must not be implemented in the iRules dialect — the separator wins and expansion does not exist. The same run discovered the independent brace-line continuation divergence (N-rules) the matrix did not cover. The probe corpus is checked in at `scripts/dev/bigip-probes/`; §3 **and §4a** of the measurements are E4-grade (§4a is a single clean four-context run with a cleanup proof per object and zero residual objects), the rest is a strong non-conforming transcript pending a mechanical E4 re-run — §11's V4 |
| F4 tmsh role policy | accepted — a command-visibility overlay distinct from the Tcl baseline and the ambient `tmsh::*` surface; completion may show policy-disabled commands with the reason, semantic hooks must not treat them as callable |
| F5 `tcl_platform` effects | accepted — context overlays may refine a core variable binding's effects: the iRules overlay distinguishes the CMP-demoting global from the `static::` alias, carries the F5 source, and is lifecycle-keyed |
| F6 tmsh syntax axis | accepted — a distinct typed version axis (never comparable with Tcl axes) plus a registry-declared state transition for `tmsh::modify cli version active`: constant argument updates subsequent tmsh resolution, dynamic widens to `Unknown`; realm scope assigned only after the probe establishes it. **Landed (evidence lane, #15)**: `VersionAxisId::tmsh_syntax()` (and its sibling `VersionAxisId::big_ip()`) are typed axes in `tcl-dialect`, so every binary operation against a Tcl core or package axis is `AxisMismatch` rather than a coincidence of dotted numbers; `rust/tcl-registry/src/f5/tmsh_syntax.rs` resolves the `tmsh::modify cli version active` form through the ordinary `InvocationArguments` view (literal → `Selected`, non-literal → `Unknown`) and carries `scope_is_measured: false` until the realm-scope probe of §12 is run |
| F7 iApp action metadata | accepted — `requires-bigip-version-min/max`, `role-acl`, `run-as` parse into typed document overlays intersecting the configured targets; appliance security settings (e.g. `systemauth.disablebash`) are a separate live-policy overlay; unknown principals widen authorisation-sensitive analysis. **Landed (evidence lane, #15)**: `rust/tcl-registry/src/f5/iapp_metadata.rs` parses the four properties (already present in the BIG-IP object schema, and pinned against it by test) into `IAppActionOverlay` — the declared interval on the BIG-IP axis, `effective_targets` intersecting it with the configured targets, `role-acl` as `None`/empty/roles, and an omitted `run-as` reading as the **calling user**, which widens authorisation analysis rather than inheriting an administrator surface. Appliance security settings stay out of this document overlay |
| F8 conformance corpus | accepted — a checked-in F5 corpus (per-build, per-context manifests; exact scripts; normalised privacy-safe outputs; explicit `unknown` cells) with a transcript validator that never runs in CI; registry rows generate from the corpus and a drift gate fails when prose, rows, or tests disagree with it. **Partially landed (evidence lane, #15)**: `rust/tcl-registry/src/f5/corpus.rs` holds 205 hermetic vectors derived from the checked-in transcripts — 21 §4a parity cases, 9 §4a environment rows, 16 discriminators, 31 §4b command classes, 120 event-context cells, 8 priority facts — each citing its measurements section and each asserted against the model. Rows carry a two-sided `ModelExpectation`: `Agrees` must keep agreeing, `Diverges` must keep diverging *exactly where recorded* (so closing a gap is a deliberate edit), `NotComparable` names why there is no model answer. **Defects fixed (#27)**: the corpus immediately caught sixteen real model defects and P4 closed every one by moving the model to the measurement — eight over-permissive event cells (`HTTP::uri` in `HTTP_RESPONSE`, `HTTP::status` in `HTTP_REQUEST`, `SSL::cipher` in `RULE_INIT`/`CLIENT_ACCEPTED`/`CLIENT_DATA`/`SERVER_CONNECTED`/`CLIENT_CLOSED`, `LB::server` in `RULE_INIT`), seven over-strict ones (`HTTP::uri`/`HTTP::status`/`HTTP::collect` in `LB_SELECTED`, `IP::server_addr` in the four client-side events), and the missing bare `matches` word operator. The event-context divergence count fell 21 → 6, and the six that remain are the deliberate `RULE_INIT` compile-acceptance rows §8 of the measurements says must not be read as "valid to use". Still open: registry rows are not yet *generated* from the corpus, and the transcript-schema validator is not written |

**Migration hold**: no F5 row moves into the new source of truth until the
review's acceptance matrix has its required coverage; until then the
compiled seeds carry today's shipping claims explicitly marked as
translation-of-shipping-hypotheses. The hold is now *checkable* rather
than declarative: the corpus (`rust/tcl-registry/src/f5/corpus.rs`)
covers **one build and four contexts**, and its own test says so — the
17.x column, the older-build column, the restricted-role tmsh column and
the two APL contexts remain empty, and the two APL contexts resolve
`Unknown` through both evidence doors.

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
| `{*}` expansion (TIP 157) | off in 8.4 and iRules; on 8.5+. **iRules is measured, not merely off**: on TMM the separator wins — `{*}$l` lexes as a literal `*` word plus the unexpanded list, silently ([measurements](bigip-irule-parser-measurements.md) §1). The iRules lexer must produce exactly that (two words, no error, no expansion); implementing 8.5 expansion here would silently disagree with the appliance |
| iRules implicit word break (`}{`) | the `f5-tcl` trunk (all three F5 catalogue rows select it since P1-G: `f5-irules`, `f5-iapps`, `f5-tmsh`) — **measured on BIG-IP 21.1.0.1** ([measurements](bigip-irule-parser-measurements.md) §1, §3): the six-row F3 matrix is answered and the separator is **generic and lexical**, not `if` grammar. Normative rules R1–R7: fires only when the word *started* with `{` or `"` (bare words, `$v{b}`, `${v}b`, `[cmd]{b}` are untouched — 23 diverging vs 11 identical cases, split exact), applies repeatedly and in every word position including the command name (`{set}zz 7` runs `set zz 7`), restarts word parsing from scratch (`}else{log` becomes the bare word `else{log`), emits no diagnostic, and leaves the `expr` sub-parser unmodified. The zero-width `SEP` token in `Lexer::parse_brace` (`rust/tcl-lexer/src/lexer.rs:1242`) matches the appliance; it stays lexer data with no BIG-IP lifecycle |
| iRules brace-line continuation (N-rules) | the `f5-tcl` trunk (all three F5 catalogue rows select it since P1-G) — the **second, independent measured divergence** ([measurements](bigip-irule-parser-measurements.md) §2): a newline does not terminate a command when the next line's first non-whitespace character is `{` (K&R brace style is legal). Unconditional (not arity- or command-dependent: `list a b` ⏎ `{c}` → 3 elements), any nesting depth; a blank, whitespace-only, or comment line terminates normally; `else`/`elseif` are a separate one-newline lookahead by `if` that does not cross a blank line. Implemented as the `BraceLineContinuation` axis, honoured by the lexer (`Lexer` consults `brace_line_continuation` at newline) and set to `Continues` by the shared `GRAMMAR_F5_TCL` value all three F5 rows carry |
| `${…}` close rule | `FirstClose` (8.x) vs `Tcl9Nesting` (9.x) |
| Leading-BOM skip on `source` | 9.x only |
| `#` comments in `[expr]` (TIP 582) | 9.x only |
| Numeral grammar | `Tcl84` / `Tcl85` (`0b`/`0o`) / `Tcl90` (`0d`, `_` separators, leading-zero decimal) |
| Escape grammar (TIP 388, `TCL_UTF_MAX`) | `Tcl84` (=8.5) / `Tcl86` / `Tcl90` |
| expr word-operator lexemes | `eq`/`ne` ≥8.4, `in`/`ni` ≥8.5, `lt`/`le`/`gt`/`ge` ≥9.0, plus **ten** F5 word operators on the `f5-tcl` trunk (`contains`, `starts_with`, …). **Corrected twice by measurement**: they are trunk facts, not iRules-only — a tmsh `cli script` and an iApp implementation answer them byte-identically to TMM ([measurements](bigip-irule-parser-measurements.md) §4a) — and the tenth, the bare `matches`, was missing from the trunk table until the conformance corpus found it (#27). Its precedence and semantics are **inferred, not measured** (§11's V5): it takes its siblings' equality class, answers as string equality in the VM, and is deliberately excluded from constant folding |

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
— parse-level facts that keep `f5-irules` a **dialect offshoot** even now
that the underlying parser is the shared trunk (§2, owner rulings
2026-08-26: the bans themselves stay environment policy per B12; the
top-level form and the rule compiler's load-time strictness stay
dialect — the word operators proved to be trunk grammar, §4a). **`f5-iapps` and
`f5-tmsh` do *not* use a plain-Tcl core release's grammar verbatim**:
measurement ([measurements](bigip-irule-parser-measurements.md) §4a)
shows all three BIG-IP contexts share one parser, so the fork grammar is
a property of the **`f5-tcl` trunk family**, not of the `f5-irules`
family alone, and both environments select the trunk — while `f5-irules`
selects the offshoot that inherits from it. `expect`, `spectcl`, `bpf`,
`tk`, and all six EDA entries do use a core release's grammar
**verbatim**; `f5-bigip` is not Tcl at all (its own tokeniser and
tree-sitter grammar; the profile's grammar field is inert). **The old
catalogue rows now agree (P1-G):** `f5-iapps` and `f5-tmsh` select the
shared `GRAMMAR_F5_TCL` trunk grammar (R-rules, N-rules, inert `{*}`,
8.4 numerals) with an 8.4 base (`TCL84|vendor` mask, 8.4 version
ceiling/signature/runtime/expr bases, `operators_as_commands: false`),
so live lexing and availability match the measurement instead of the
falsified `GRAMMAR_TCL85`/8.5 hypothesis.

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
| `tcl8.4` … `tcl9.1` | dialect (family `tcl`, releases 8.4–9.1) | 9.1 has no grammar delta vs 9.0 but is a core release; releases are the family's version ladder, not separate catalogue entries. **Owner ruling (2026-08-26): the ladder stays exactly these per-release identities.** A per-major family split (`tcl8`/`tcl9` as families) was considered and rejected — succession is not a fork, and the single total order is load-bearing for `ItemHistory` windows spanning the 8/9 boundary, for the `vsatisfies`-oracle-pinned `VersionSet` algebra (where `tcl 8.5-` matches 9.0, as in Tcl itself), and for cross-major range targets like `tcl 8.5-9.0`. `tcl8`/`tcl9` named-range sugar was also declined: requirements spell explicit releases and ranges. Family status is earned by independent evolution (the `f5-tcl`/`jim` criterion), never by version numerals |
| `f5-tcl` | **dialect (family `f5-tcl`, fork of Tcl 8.4.6, ladder keyed by TMOS release)** — owner-adopted (2026-08-26), measurement-backed | The shared BIG-IP fork trunk. **Measured identical in all three execution contexts** ([measurements](bigip-irule-parser-measurements.md) §4a, §4b): the implicit word break (R-rules), the brace-line continuation (N-rules), the inert `{*}`, 8.4 numerals, full ordinary `proc` semantics, and the `expr` word operators — which are **not** iRules-only. `f5-tmsh` and `f5-iapps` are environments *over this dialect*, differing only in ambient packages and host facts, with no grammar delta between them; iRules rides it as a further dialect offshoot |
| `f5-irules` | dialect (family `f5-irules`, **offshoot of `f5-tcl`** — a fork of a fork, owner ruling 2026-08-26) + environment `f5-irules` | Inherits the trunk grammar whole (R-rules, N-rules, inert `{*}`, word operators — all `f5-tcl` facts per §4a). What it adds *over `f5-tcl`* is not lexical grammar but **load-time language rules**, measured live: the declaration-only top level — only `when`, `proc`, `priority`, `timing` at the root of a rule (bare `set` is `"set" unknown property` from the config layer; `IrulesExecutionContext`, IRULE5006/5007; top-level `proc` reachable only via `call`, an iRules-only command) — closed-world command resolution at rule load (unaffected by `catch`), the event model with compile-enforced event-context validity, and `expr` math-function validation at load. §4c sharpens how those load-time rules work: they are **lexical scans of braced script literals** — `eval {proc …}` and `uplevel #0 {proc …}` are rejected identically to a bare `proc`, but script text held in a variable escapes the scan entirely (both the definition *and* the literal call head must be hidden), so our analyser must mirror exactly that: recurse the load-time checks through braced literals under `eval`/`uplevel`, and abstain — widening to `May` bindings — on variable-held scripts. A runtime-defined proc is then a **persistent per-TMM global** (survives across events and connections, definable mid-request), while `when` is not a runtime command in any scope — the event table is fixed at load. And one severity is pinned by measurement: unbraced `if $var` is both the construct F5 itself warns about and the load-bearing primitive of the only user-space JIT the dialect permits (KaiWilke's `static::`-cached compiled-expression idiom), so it must surface as a **warning, never an error**, ideally with the cached-expression idiom recognised. Riding on that dialect, the `f5-irules` **environment** carries what was always policy (review B12): the 31-command disabled list, which §4b splits into **two mechanisms** — **16 absent from TMM's interpreter** (`exec`, `open`, `socket`, `source`, `file`, `glob`, `cd`, `exit`, `load`, `pwd`, `fconfigure`, `unknown`, the four `auto_*`) and **15 present but refused by the rule compiler** (`namespace`, `time`, `rename`, `interp`, `package`, `gets`, `eof`, `seek`, `tell`, `flush`, `fblocked`, `fcopy`, `pid`, `update`, `vwait`), the latter reachable via `eval` at runtime, so a policy warning about rule *source*, not a language fact — plus 8.3-era-only `trace`, static-head-identity, and fabricated `tcl_platform` (`os BIG-IP`, `wordSize 8`) |
| `jim` (was `jim0.76`–`jim0.84` on the branch) | dialect (family `jim`, releases 0.76–0.84) + **one** environment `jim` (aliases `jimsh`, `jimtcl`) | measured grammar deltas per release (`NumberSyntax::Jim`/`Jim080`, `EscapeSyntax::Jim`, expr comments ≥0.81, special-float set; since extended with the five lexical axes and the expr precedence/operator/mathfunc/arity divergences — §1, §3.1). **Landed (P6)**: the nine profiles are one environment plus a ladder, the releases are targets on the `jim` core axis, and the family carries a `Lineage::Reimplementation` ancestry edge to Tcl 8.6 that inherits the core command surface instead of re-authoring it — see §8's P6 status |
| `f5-iapps` | environment `f5-iapps` = **dialect `f5-tcl`** (32-bit build profile) + iapps pack (ambient, BIG-IP-keyed) + policy (fixed ensembles, W108 strict ASCII, no hosted tcllib) | **CORRECTED by measurement** ([measurements](bigip-irule-parser-measurements.md) §4a): the 8.5 baseline hypothesis is falsified. `IAppImplementation` reports `info patchlevel` **8.4.6**, fails every 8.5 discriminator (`dict`, `lassign`, `apply`, `0b101`), and its grammar is **not** `GRAMMAR_TCL85` verbatim — it carries the full `f5-tcl` trunk grammar: R-rules, N-rules, inert `{*}`, and the expr word operators, all byte-identical to TMM. Environment deltas are real but *non-grammatical*: `exec` works here and not in TMM, `package names` is large, `tcl_platform` is real-Linux with `wordSize 4` — a 32-bit build of the family, the CoreProfile build-profile axis earning its place again. APL container routing remains a language-id fact; the two APL contexts are still `Unknown`. **Catalogue corrected (P1-G)**: the shipping `f5-iapps` row now selects `GRAMMAR_F5_TCL` with the 8.4 base fields and `operators_as_commands: false` |
| `f5-tmsh` | environment `f5-tmsh` = **dialect `f5-tcl`** + tmsh pack (ambient, BIG-IP-keyed) + tmsh syntax-version axis | **CORRECTED by measurement** ([measurements](bigip-irule-parser-measurements.md) §4a): both claims in the previous cell are falsified. `TmshCliScript` reports **8.4.6**, not 8.5.13, and a tmsh lexing mode **is** required — a `cli script` reproduces the entire trunk grammar (R-rules, N-rules, inert `{*}`, expr word operators) identically to TMM. The `AGENTS.md` owner-map claim was right after all: the mode selects the `f5-tcl` trunk grammar. Environment deltas: `exec` works, `tcl_platform` is **empty**, `tcl_patchLevel` does not exist at all, and a non-standard `info vartype` subcommand exists. The `IAPPS\|TMSH` spec files still split into two packs sharing sources. **Catalogue corrected (P1-G)**: the shipping `f5-tmsh` row now selects `GRAMMAR_F5_TCL` with the 8.4 base fields and `operators_as_commands: false` |
| `tk` (off-catalogue) | package `Tk` + environment `tk` (alias: "wish") = tcl@base + Tk ambient | erases the tk triangle. **Landed (P3)**: the `tk` environment places `Tk` ambient on Tk's own axis and every plain-Tcl environment places it hosted, so one package answers both — see §8's P3 status |
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
pub enum Family { Tcl, F5Tcl, F5Irules, Jim /*, SslicTcl? */ }
// F5Tcl forks from Tcl@8.4.6; F5Irules forks from F5Tcl (fork of a
// fork, §2): grammar resolution walks the fork edges, so an axis the
// offshoot does not override answers from the trunk, and the trunk
// from tcl@8.4.6.

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
`package require` model can recover it. P6 adds a second, sharper proof
from the same source: `auto.def` **flipped its default at 0.82**, so a
bare `./configure` gives neither utf8 nor math through 0.81 and both from
0.82. "The canonical build" therefore names two different capability
records on one ladder, which is why `CapabilitySet::canonical` is keyed by
release rather than by family. Tcl's own history has the same
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

- **Precedence is not a per-token fact — and, P6 proved, not purely a
  per-family one either.** The original claim here was "a per-family fact";
  the upstream sources falsified the second half of it, and the field is
  release-keyed accordingly. C Tcl merges
  the comparison operators into two levels (`tclCompExpr.c`):
  `== != eq ne` at one, `< > <= >= lt le gt ge in ni` at the other. Jim
  splits the same operators across four-plus (`jim.c:9252-9285`, `OPRINIT`
  precedences, stable across every modelled release *for the comparison
  block* — **but not for the whole table. P6 found `**` at 250 and
  left-associative on 0.76, and at 120 and right-associative from 0.77:
  the row moved its binding power *and* its associativity, so the table is
  release-keyed after all and a "which operators exist" model cannot
  substitute for it. `expr {-2 ** 2}` is −4 on 0.76 and 4 from 0.77 (the
  unary minus, at 150, overtakes it); `expr {2 ** 3 ** 2}` is 64 at 0.76
  and 512 at 0.79. And `lt`/`gt`/`le`/`ge` are not present throughout as
  this section originally claimed — they are absent from
  `Jim_ExprOperators` at 0.76–0.79 and arrive at 0.80, so offering them
  under `jim 0.78` would offer a syntax error**): `in ni` 55,
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
  every modelled Jim release. **Confirmed and refined by P6**: 26 from
  0.77 (23 at 0.76 — `atan2`, `hypot` and `fmod` arrive at 0.77), and the
  set splits on the build axis *per function*, not wholesale: seven rows
  (`int`, `wide`, `abs`, `double`, `round`, `rand`, `srand`) sit outside
  `#ifdef JIM_MATH_FUNCTIONS`, so a `--minimal` build still evaluates
  `int(4)` while rejecting `sqrt(4)`. Today's `TclVersion`-floor keying in
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
  arity windows are already representable. (P6 correction: the flip is
  **compat-gated** — both halves sit behind `JIM_COMPAT` in the C, so a
  `--compat` build still concatenates at 0.84. `--compat` is off unless
  asked for, so the ladder carries the default build's value and the
  compat column is a recorded probe, not a second ladder row.) The *parse* behaviour —
  whether a multi-word `expr` concatenates its words with spaces before
  parsing — is the `arity` field here, because the analyser needs it
  before any spec is resolved.
- **The lexical axes are Jim's own, not Tcl 9's** — a P6 correction to the
  interim scaffold rather than to this list, recorded here because this is
  where the family's axis values are pinned. Jim's `${…}` close rule is
  `FirstClose` (the 8.x rule) and it skips **no** leading BOM; the interim
  value copied Tcl 9's grammar wholesale and was wrong on both. The five
  further lexical axes the jim branch measured (word separators, brace
  continuation, quote termination, `$(…)` variable syntax, list parse)
  remain new `LexerGrammar` fields, unlanded.

`RuntimeExprSurface` (today: release floor ∧ dialect-set intersection)
re-derives from `ExprGrammar` plus provider availability; nothing keeps
a second operator table. **Landed for F5**: `f5_core_expr_grammar()` maps
every `F5Tcl`-cored profile to the model `ExprGrammar`, the expr lexer's
duplicate operator list is deleted in favour of reading that table, and
W003 keys on the family grammar instead of `is_irules` — so tmsh and iApp
documents stopped drawing "operator not available" for operators they
measurably accept. The `TclVersion`-keyed `RuntimeExprSurface::for_tcl_version`
survives for the plain-Tcl ladder (ledger B6/C12).

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
- The F5 grammar lives here as a two-level tree (owner rulings
  2026-08-26): the implicit word break (R-rules), the brace-line
  continuation (N-rules), the inert `{*}`, and the expr word
  operators are `Family::F5Tcl` trunk facts — live-measured in all
  three F5 contexts
  ([measurements](bigip-irule-parser-measurements.md) §4a); the
  declaration-only top-level form and the rule compiler's load-time
  strictness are `Family::F5Irules` offshoot facts (fork of
  `f5-tcl`); the command
  bans and closed-world guarantee are environment/realm policy (§2).
  Grammar resolution walks the fork edge: an axis the offshoot does
  not override answers from the trunk, and the trunk from `tcl@8.4.6`.
  The measured context split (three-way `tcl_platform` fabrication,
  scriptd's 32-bit build profile, the two APL contexts still
  `Unknown`) keys the F1 execution contexts and environments, not new
  families.

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
  **Landed for the compiled environments (P3)**: `Tk` is placed both ways
  at once — ambient under `tk`, hosted under every plain-Tcl environment
  — and that pairing is what forces the model to distinguish a *library
  with an ambient host* from a *closed-world vendor runtime*. The
  distinction is derived from the placement data itself (ambient
  somewhere ∧ hosted nowhere ⇒ closed-world), never a name list, so a
  pack that later places a library ambient in its own environment gets
  the same answer for free.
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

> **Status (P1a, single-realm slice landed).** `BindingKnowledge` gained
> its target type (`Spec` — the I4 hook licence — vs `Document` for
> user-established bindings) and is produced by two integrated sources:
> the compiler's document realm scan (`tcl_compiler::realm` — the
> offset-keyed top-level command-binding state, retired-in from
> `head_identity.rs`, ledger C4) and the analyser's one `exists` oracle
> (`command_existence_oracle`/`command_binding_knowledge`, R-c), whose
> document-wide widenings type the package/provider transitions above
> and whose `Absent` verdict is exactly W123. The consumer contract's
> I4 half is enforced at the model's selection primitives (see §8's I4
> gate and the centralisation ledger's C3/C5/C7 rows). Still open here:
> the multi-realm `AnalysisWorld` map (child/safe interpreters — Q21's
> confirmed increment), `PackageTransition` feeding a live
> `PackageStateMap` (today the require/provider widenings are consumed
> as oracle state), and `StateTransition`'s package variant (C8).

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
#1626).

**Landed (P2-H, P3):** pack-declared detection facts are live —
`registration::extension_routes` is a pure function of the loaded pack set
(explicit `file_extension … -dialect D` rows first, then each `environment`
block's own detection rows, first claim winning), published at the pack
*merge* so an identical reload cannot lose it, and the server advertises
the same extensions to the client so an editor opens the file as Tcl at
all. A document whose extension a pack-declared environment claims routes
to it through the ordinary ingress, with no setting and nothing in the
file, and stops routing when the pack is deleted.

**Intent, not shipped:** the `# tcl-dialect:` directive should accept
environment names and aliases, making `# tcl-dialect: tk` (used in e2e
tests today) coherent. `detect_dialect_directive` still gates on
`KNOWN_DIALECTS`, which has no `tk` row, so the directive still abstains
and falls through to the next tier — see §9's defect 5 and §11's E8/D15,
where it is a user-visible payload change rather than a local fix.

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
- `closed` (`f5-irules`, and equally `bpf`/`spectcl`): only the ambient
  closure exists; hosted packs and `package require` are not part of the
  language (require itself is not an iRules command).

**Landed (P3)** at package granularity, with one honest qualification. All
three `WorldPolicy` values exist and `ResolvedContext::package_provider_active`
— the **carrier** question, "does something here actually provide this?" —
distinguishes all three. `ResolvedContext::package_active` — the
**availability** question — currently distinguishes only `Closed` from
not-`Closed`, so `ambient-only-plus-require` resolves hosted packages as
leniently as `open` does. The consequence is real and named in §11 (E2):
the tcllib-excluded-from-iApps rule is *still* the subtractive
`DialectSet::all().difference(IRULES | IAPPS)` in
`commands/tcllib/mod.rs`, not the policy, and it retires with ledger C1.
The Tk pilot is what exercises the rest from one package — ambient under
`tk`, leniently visible under every `open` plain-Tcl environment, refused
under `closed`. The `open`
leniency is deliberately a statement about packages **no environment owns
as its own runtime**: a closed-world vendor surface stays invisible
outside its own environment, which is the old `vendor_ambient_packages`
subtraction restated positively.
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

**Status (P1b — declared ranges drive diagnostics end-to-end).** The
first slice of this section ships: a document/project that *declares* a
target range gets range-compatibility warnings; an undeclared document is
byte-identical to before (pinned by
`a_single_target_matching_the_primary_changes_nothing`).

- **Model** (`tcl-registry::model::context`): `ResolvedContext` carries
  **declared** target sets separately from the environment floors —
  `declare_targets` / `declared_targets` / `declared_target_sets` — so
  the lenient environments' full-ladder targets never switch range mode
  on and `primary` (every assistance answer) is untouched. Two remainder
  queries answer the checks: `targets_outside_window(axis, introduced,
  retired)` for lifecycle-spelled items and `targets_uncovered_by_gate`
  for `DialectSet`-mask-spelled ones (via `surface::tcl_core_set`), with
  `requirement_spelling` / `ladder_releases_in` naming the failing
  targets in messages.
- **Targets grammar** (working ruling, R6's directive half): clauses are
  space-separated and union; a bare `V` names **that release line only**
  (the §6.2 `available` rule, not the vsatisfies next-major window);
  `MIN-` is open-ended (clamped to the modelled ladder on a core axis);
  `MIN-MAX` runs through the **whole line of `MAX`** — `tcl 8.5-9.0`
  includes 9.0.x, because the strict vsatisfies exclusive-max reading
  would silently drop the release the declaration names. On package
  axes (no ladder) a line is `[V, V+ε)` with the last dotted component
  bumped. Implemented as `targets_from_clauses`.
- **Ingress**: `tclLsp.targets` (settings → `AnalyserConfig::targets` →
  `Analyser::with_declared_targets`; VS Code exposes the object setting)
  and the top-of-file `# tcl-lsp: supports NAME RANGE` comment directive
  (R6's resolver-invisible declaration, parsed beside the `disable`
  directives; the directive wins per provider over settings). `tcl`
  names the core axis (honoured only on a Tcl-family core); any other
  `NAME` is a package axis — the "same for libraries" ruling
  (`supports Tk 8.5-8.6`). Malformed declarations are dropped, never
  guessed at. The `tclpkg.tcl` manifest half of R6 (the multi-clause
  `tcl` constraint grammar) is **not** ingested yet.
- **Diagnostics**: **W150** — item not available across the whole
  declared range: the version-gate flush evaluates every lifecycle site
  against the declared window, the mask-gated command/subcommand/option
  sites against gate coverage, and the argument-DSL sites (`string is`
  classes, format/scan conversions, binary modifiers) against their
  minimum release; messages name the failing targets ("`'lmap'` requires
  Tcl 8.6 but the declared targets include 8.5"; "`'case'` … missing at
  9.0"). **W151** — the numeral grammar delta, the motivating case:
  whole-word literals are read under every `NumberSyntax` era the
  declared range spans; a value divergence (`expr {010}`: 8 under 8.x,
  10 under 9.0) or validity divergence (`0b`/`0o` before 8.5, `0d`/`_`
  before 9.0) warns. Both are token-local detectors in the §3.1 sense —
  the numeral axis provably diverges only token-locally.
- **Assistance/semantic split**: a failure at the **primary** keeps
  today's semantic-floor diagnostic (W002/W135/W136/W139/W144, unchanged
  spans and messages) and always outranks the range warning at the same
  word; W150/W151 fire only for items clean at the primary, at Warning
  severity — declared non-primary targets are assistance facts.

**P3 addendum — the package half is exercised.** §3.2's last bullet
("packages take range targets exactly like cores") needed no further
code: `supports Tk 8.5-8.6` declares a set on the `Tk` package axis, the
version gate already routes a Tk item's lifecycle to that axis through
`lifecycle_axis`, and the W150 remainder names the failing Tk releases.
The pilot pins it on the acceptance case — an item Tk only grew at 8.6
under a declared `Tk 8.5-8.6` — plus the two controls that keep
invariant I2 honest (a `tcl` declaration must not gate the `Tk` axis) and
the `wish` variant, where the range applies with no `package require` at
all because the environment places Tk ambient.

Deferred from this slice: numerals *inside* compound `expr` bodies (the
whole-word walk catches `expr {010}` and plain arguments; a braced
`{010 + 5}` needs the expr-token walk), the other grammar axes (escapes,
`${a{b}c}`, expr comments/operators — the reference multi-profile
evaluation), differential constant folding at the endpoints, the
`package require`-satisfiability-per-target interplay, fix-its
(`0o10`/decimal rewrites), Q15's explicit `primary` selection for
multi-target projects, per-folder `tclLsp.targets` overrides, and the
R6 manifest grammar.

## 6. SpecTcl 2.0 (`speclib … 2.0`)

> **Authoring-surface decision (owner, 2026-08-26): design E,
> executable registration** — a pack file is a Tcl program evaluated
> in the sandboxed tclvm whose registration calls produce a frozen
> registry snapshot — per
> [the six-design comparison](spectcl-syntax-alternatives.md) and
> [the design-E deep dive](spectcl-design-e-deep-dive.md), whose §1
> execution model (frozen snapshots keyed by content hash ×
> vocabulary, the determinism contract, `-available` rows over
> control flow, trust enforced at the registration call) is adopted
> with it, rulings E-R1–E-R9 ratified. The declarative vocabulary in
> this section is unchanged by the decision: those words become the
> registration commands of the evaluator, and existing 1.x/2.0 packs
> — straight-line registration calls — evaluate to byte-identical
> snapshots (the old-vs-new loader equivalence gate).

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

**Status (P2-H part 1).** `VocabularyClass { Presentation, Assistance,
Semantic }` classifies the loader's unknown-word path. An unknown word is
classified by the scope it appears in (every unknown word inside a
`dialect` or `environment` block is semantic by construction, and rejects
the block) and otherwise by a closed name-marker table. A presentation
unknown warns and drops as before; an assistance unknown marks the loaded
`PackCommand` `degraded`; a semantic unknown excludes the command from the
pack entirely, with a distinct notice class.

The escalation past `Presentation` applies **only in the forward
direction** — a pack declaring a vocabulary this build postdates. An
unknown word in a pack whose vocabulary the build knows in full is an
author's typo, not a meaning being dropped, and keeps the warn-and-drop
treatment exactly; that is what keeps every 1.x pack in the frozen corpus
loading unchanged.

An unsupported **major** now fails the whole pack closed: nothing loads,
`Pack::load_error` is `Some(LoadError::UnsupportedMajor)`, and one notice
explains why. An unknown *minor* within a supported major keeps loading
maximally.

### 6.2 New vocabulary (the additive core of 2.0)

**Status (P2-H part 1).** The loader speaks vocabulary `2.0`
(`KNOWN_VOCABULARY_VERSIONS`, `NEWEST_VOCABULARY_VERSION`), and
`VOCABULARY_VERSION` — the compiled-cache key — bumped once to `2`, per
§6.1. Landed from the table below:

- **`available {PROVIDER SPEC…}`** at every scope `dialects` / `-dialects`
  is accepted at today: pack `default`, `command`, `subcommand`,
  `sub_subcommand`, `option`, object-class method, `form`, `side_effect`,
  `option_conflict`. Providers are `tcl RANGE`, `f5-irules`, `jim RANGE`,
  and `package NAME ?RANGE?`, with `RANGE` in Tcl requirement syntax
  (`8.6-`, `8.4-9.0`, or a bare `8.5` naming that release line only).
  Translation is **new → old**: a row is projected onto the same
  `DialectSet` + `required_package` the legacy word feeds, so a body
  spelled either way loads to a byte-equal `CommandSpec`
  (`available_and_dialects_load_byte_equal_specs`). `f5-bigip` in an
  `available` row is an error (Q3). Legacy `dialects` is untouched.
- **`environment NAME { … }`** — `core`, `ambient`, `hosted`, `alias`,
  `editor_identity`, `file_extension`, `filename`, `signature`,
  `display_name`, `policy`. Parsed, validated, and carried on the pack as
  `PackEnvironment`, with `to_definition` converting to an
  `EnvironmentDefinition` at the declaring tier's `Provenance`. Compiled
  canonical names and aliases are reserved (§3.3): a block claiming one is
  rejected with a notice. **Live in production**: the one pack publish
  point (`bundled::set_active`, which the LSP server calls on every reload
  and the CLI once per process) registers the whole loaded set through
  `registration::publish_pack_set`, so a pack's environments resolve
  through the ordinary ingress, a document whose extension one claims
  routes to it, and a pack that has left the workspace has its
  environments retired.
- **`dialect NAME { … }`** — `release R ?-build P?` ladder rows and
  `axis NAME VALUE` rows against a closed axis vocabulary
  (`expand_syntax`, `braced_var`, `expr_comments`, `numbers`, `escapes`,
  `irules_brace_separator`, `bom_skip`). An unknown axis or value is §6.1's
  semantic class: the whole block is rejected, naming the axis. The §2
  classification gate rejects a block whose axes reproduce a compiled
  family release and names the environment it should have been.
  **Converted (P3)**: a validated block becomes a
  `tcl_dialect::model::DynamicFamily` — a namespaced `PACK/DIALECT` id, one
  `LexerGrammar` per declared release, the declaring tier's provenance,
  compiled family names reserved — and an `environment … { core DIALECT
  RELEASE }` row naming it registers as a `DynamicCore` binding whose
  grammar `dynamic_core_grammar` answers with. The remaining step is the
  lexer's, not the conversion's: `Family` is a closed enum and
  `LexerConfig` is built from a `&'static DialectProfile` out of a compiled
  table, so a pack-declared core cannot yet be *lexed* with. That waits on
  the `DialectProfile` re-type (ledger C1).

**Status (P2-H remainder).** `provides` (with the fallback-provider
default), `co_provides` (parsed and carried as data; the alias mechanics
that consume it are P3+), `dynamic_surface` / `unknown_members` (mapping
to the open-surface facts on commands and object classes), `include`
(pack-file inclusion under the determinism contract), and the additive
`environment NAME -extend { … }` form are landed at the shared
row-reader seam of both loaders — see the centralisation document's §6
status note for the details and the registration seam that consumes the
environment blocks.

**Status (P3).** Six of the seven ratified-but-unimplemented words now have
loader readers at that same shared seam, each riding the export gates and
loading identically through both front ends: `result_stability` (command
*and* subcommand scope, including the `{ReadsVersionedWorld {D …}}`
payload form), `event_requirement_form` (with its nested `event_requires`
block), `data_collection -native ID`, `body_scope` (inline block, pack
`descriptor`, or a shipped environment by name), `side_switch_target`, and
`event_handler_priority`. They are documented vocabulary rather than
2.0 additions, so they draw no per-site version notice. `bpf_op` is
deferred and needs a **model** change first: `CommandSpec::bpf_op` is
`Option<&'static BpfOpSpec>` and every shipped value is a private
per-command `static OP` under `tcl-registry/src/commands/bpf/`, so
`bpf_op -native ID` has no id catalogue to resolve — the missing piece is a
named `id → &'static BpfOpSpec` table in `tcl_registry::bpf_op`. Its
downgrade class is already correct (`bpf` is a semantic marker: an older
build that drops the word must exclude the command, not load it without a
lowering). Still not landed: the invocation-refinement descriptor (Q12),
which is a new descriptor type rather than a reader over an existing
field.


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
sweeps).

**Status (2026-08-27): this is the end-state, and the tree is most of the
way to it.** What actually went: `resolve_known`, `availability_for_name`,
`by_name`, `by_opt_name`, `hosts_tk`'s consumers, `registry_for_dialect`,
`registry_for_dialect_profile`, `profile_for_dialect`'s hop,
`LanguageDialect::{Profile,Set}`, `head_identity.rs`, the six divergent
validators, and `side_effects.rs`'s hand-rolled selection — all deleted
and held at zero by `cargo xtask retired-api-gate`. What is still standing
in this list: `DialectSet` itself (bits and combinators, behind the
semantic-facts bundle), `DialectProfile` with `PLAIN_TCL` and `TK_PROFILE`
as interned statics the seam consumes, `KNOWN_DIALECTS` as the directive
and CLI vocabulary, and `ProfileQueries` narrowed to `pub(crate)`. All of
it is ledger row **C1** plus the tooling payload rows — §11's D1 and D15.
The per-editor and per-CLI *ingress* work is done; the per-editor and
per-CLI *payloads* are not.

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

**Final status of the programme (2026-08-27).** Every phase below carries
its own status block. In summary: **P0, P1, P1-E/F/G, P1a, P1b, P2 (H, I,
J), P3, P4, P5, P6 and P8 all landed**; **P7 stayed deliberately deferred**
(Q5); and the named remainders — the C1 executable-IR re-key, the
invocation-refinement descriptor, `bpf_op`'s id catalogue, the multi-realm
`AnalysisWorld`, corpus-*generated* F5 rows, and the appliance coverage the
acceptance matrix still wants — are collected in **§11**, not left implied
in phase prose. What did *not* happen in any phase is a surface-to-pack
conversion: P3 (Tk) and P5 (tcllib) both proved the *model* move on the
real surface while the **declarations stayed compiled Rust**, which is the
split each phase set out to demonstrate is possible, and §11 records what a
conversion still needs.

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

  **Status: done.** The contracts ratified as revision 2 (§0.1, §0.2) and
  the oracle programme's binary half landed as P0-B — `ensure-test-deps.sh`
  builds all five reference interpreters (8.4.20 / 8.5.19 / 8.6.16 / 9.0.4 /
  9.1b0), `audit_option_dialects` fails loudly rather than degrading an
  unbuilt tree's column, `tcltest_sweep`'s hardcoded reference path is gone,
  and the five Tk trees are fetched (companion §7.1). The one `AGENTS.md`
  owner-map correction §9 listed turned out **not** to be a correction: the
  "tmsh mode per dialect" claim was *vindicated* by measurement (§9 item 2).
  Still open from P0: upgrading the single-binary probes to the five-version
  matrix, and the vector-file domains the companion's §7.2–§7.5 enumerate
  (§11).
- **P1 — core/environment model only.** `Family`/`Release`/`CoreProfile`
  (with build profiles), `EnvironmentDefinition`/`EnvironmentOverlay`,
  and central ingress land in `tcl-dialect`/`tcl-registry` with today's
  data expressed in the new model — existing native package specs stay in
  place. The four validators collapse to `Environment::resolve`. Editor
  catalogues regenerate (names unchanged ⇒ small diffs). The tk triangle,
  `TK_PROFILE`, and `LanguageDialect::Set` die here. Gates: I1, I2, I8.

  **Status: done, in five sub-waves.** *P1-E* built the model beside the
  old code (`tcl_registry::model::{surface,context,assembly,binding}`) and
  proved it: 65,646 per-spec visibility checks and 38,713 resolved names
  reproduce `ProfileQueries` / `registry_for_profile` with **zero**
  divergences and an empty deliberate-divergence allowlist. *P1-F* moved
  every production consumer onto the one shared ingress seam
  (`tcl_registry::model::ingress`) in four waves — compiler, then the LSP
  trio, then the engines / runtime / VM hosts, then CLI, MCP, studio,
  `tcl-spectcl` and xtask, with a fourth-b wave for f5-cli and
  tcl-explorer. *P1-G* deleted the retired APIs outright
  (`DialectProfile::by_name` / `by_opt_name` / `resolve_known` /
  `availability_for_name`, `tcl_registry::registry_for_dialect` /
  `registry_handle_for_dialect`), ported the ~1,600 remaining fixture call
  sites, and installed `cargo xtask retired-api-gate` in `make xtask-check`
  so those spellings cannot reappear. The four validators are one
  `Environment::resolve` (ledger C2), `LanguageDialect::Set` is gone
  (F3), and `profile_for_dialect` + `registry_for_dialect_profile` are thin
  faces of the seam (F2).

  **Two claims in the bullet above did not come true in P1, and one not at
  all yet.** The **tk triangle died in P3**, not here — erasing it needed
  the placement model, so the name survived P1 by design. And **`TK_PROFILE`
  is not deleted**: `DialectProfile::tk()` still returns an interned static,
  because the `tk` environment's `unit_profile()` must carry the `tk`
  identity (name, label, Tk library pins) while `analyser_profile()` sinks
  to the permissive fallback — the analyser-vs-unit asymmetry P3 ruled
  **permanent and narrowed to catalogue identity alone**. It retires with
  the interned `DialectProfile`, ledger C1. Two further things P1 did not
  finish, both in §11: the `DialectSet` residue behind the semantic-facts
  bundle (C1 again) and the *payload* halves of the tooling rows
  (T1/T3/T4/T6/T7 — the user-visible enumerations and enums).
- **P1a — realm state.** Integrate `state_transition.rs` with provider
  candidates: package transitions, safe interpreters, import/alias/rename
  effects, and the one shared name resolver produce `BindingKnowledge`;
  the assistance and semantic query APIs split. Gates: I3, I4, I5.
  **Status: the single-realm slice is landed — Q21's confirmed increment,
  exactly.** The document realm scan
  and the analyser's one `exists` oracle produce `BindingKnowledge`
  (§4.2 status note), invariant I4 gates the model's three selection
  primitives (hook/type-fact/hint selection requires the environment
  binding proof; `Absent` ⇒ no hook; documented widening queries are the
  only context-less readers), and the C4/C5 retirements are complete.
  C7's open question was decided **test-first**: a sweep measured the
  shipped hint walk against proved-single-winner selection and found nine
  real divergences (`next` under `bpf`; `exit`/`send`/`close` under
  `expect`; `option` and four subcommands under `spectcl`), so the walk
  stays inside the primitive behind the head proof — with a test built to
  fail the day the count reaches zero — and the head-proof gate was
  measured to widen **zero** hint selections across 64,787 catalogue
  checks. Two behavioural deltas were enumerated and pinned: settlement
  and W113/const-dispatch no longer believe in environment-disabled
  commands, and selection declines unproved heads (`try` under `tcl8.4`
  selects no hook; under `tcl9.0` it keeps one).

  The multi-realm map, the `state_transition.rs` re-homing of the
  analyser's fact tables (C6 tail), and the `CommandTableEffect`
  vocabulary fold (C8) remain this phase's open tail — carried in §11.
- **P1b — range targeting, reference first.** Typed `VersionSet` targets
  on contexts, the `targets ⊆ applicable` diagnostic family (core and
  package providers uniformly), and the **per-distinct-profile reference
  evaluator**; detector/parse optimisations (numerals first — the octal
  case) land only after the differential gate proves each pair against
  the reference. Ships behind the targets setting; single-target projects
  are unaffected. **Status: the declared-range slice is landed** — the
  full status block, the diagnostics it added (W150, W151), and the
  explicit list of what it deferred are in §5.4. The reference
  multi-profile evaluator itself was **not** built: the two shipped
  detectors are the token-local ones (lifecycle windows and numerals),
  which is the only pair §5.4 licenses without the reference (§11).
- **P2 — durable SpecTcl foundation.** Registry generations (I7),
  trust-aware provenance (I6), the fail-closed vocabulary classes (I9),
  shared `InvocationSpec` capabilities, the loader-direction gate, 2.0
  words + legacy translation + `spec upgrade`, spec-studio parity, and
  closure or explicit `dynamic_surface` abstention for the external
  census's `[STRUCT]` gaps; `spec-author` skill refresh (its vocabulary
  section is already stale at 1.1).

  **Status: the vocabulary, the loader, the upgrade tool, the evaluator
  and the studio/AI rework all landed — in waves H, I and J.**

  - **P2-H** landed the 2.0 vocabulary: `available` at all nine legacy
    `dialects` scopes (proven byte-equal to the legacy spelling),
    `environment` and `dialect` blocks with closed axis vocabularies, the
    §6.1 fail-closed classes (`Presentation` / `Assistance` / `Semantic`,
    escalating only in the forward direction), `provides`, `co_provides`,
    `dynamic_surface` / `unknown_members`, `include` under the full
    determinism contract, the additive `environment … -extend` form, and
    live pack-declared environment registration through one publish point.
    `tcl spec upgrade` implements **U0–U10 in full**: over the eight
    bundled packs `--verify` reports 1,168 rows translating, zero TODOs,
    and 8/8 byte-identical registry snapshots.
  - **P2-I** landed the design-E **evaluation loader**: a pack file
    evaluated as a sandboxed Tcl program whose registration calls produce
    the frozen snapshot, with the determinism contract, typed budgets, and
    transactional registration. The equivalence gate is the phase's proof —
    all 24 shipped packs (1,515 commands) load **byte-identically through
    both loaders**, hooks, clause grammars, degraded flags and declaration
    lines included, with all 45 notices identical to the line number.
  - **P2-J** landed the canonical renderer and the tooling rework: `tcl
    spec export`, the `spectcl_expand` MCP verb, `spectcl_check` on the
    evaluator, the studio store reading through the eval loader,
    `render_spectcl` emitting canonical 2.0 with the `DSL_VERSION` pin
    lifted, StudioOverride patch-pack editing (E-R12), and the wasm32
    eval-loader build proven by a node smoke test.

  **What P2 did not do**, all in §11: registry **generations** (invariant
  I7's arena/`Arc` re-home and its 1,000-reload allocator test) — the
  loader still leaks per load, and no `RegistryGeneration` type exists; the
  shared `InvocationSpec` capability model (review B6); `LOADER_BUILD`
  deriving from a build hash; the §6.3 loader-direction round-trip gate
  that would make a ratified-but-unread word fail CI — `bpf_op` is still
  ratified and unread, and nothing fails because of it; `tcl spec build
  --emit rust` (ruling R7); and the
  `spec-author` skill refresh (ledger T9) — its vocabulary section is
  **still at 1.1**, two majors behind the loader.
- **P3 — Tk pilot.** Invocation-refinement descriptor first (Tk's 53
  `subcommand_forms` sites are its migration test), then
  `specs/tk.tclspec` generated from the native specs, gated on
  representation parity **and** behavioural parity (I10) — including
  Tcl/Tk version independence and the lowercase/uppercase loader
  semantics (B11) — then the native `commands/tk` deleted. The `tk`
  environment ships beside it. The Tk semantics epic (#1710) continues
  against the pack form.

  **Status: the model move is landed; the surface stays compiled.**
  What moved:

  - **Placement.** The `tk` environment places `Tk` **ambient** on Tk's
    own version axis (`Requirement Tk 8.4-` — never `tracks-base`, B11);
    every plain-Tcl environment places it **hosted**, the release-pinned
    ones under B11's named host exemption (`tclsh8.6` ships Tk 8.6, so
    the point on the *Tk* axis is derived from the pinned core release)
    and the unpinned `tcl` sink under a bare requirement. Identity,
    aliases (`wish`), and detection facts were already environment data
    and stay there.
  - **Activation.** Every `required_package: Some("Tk")` spec (68 of
    them) now carries the `RequiresPackage("Tk")` conjunct, so the Tk
    surface resolves through **one** query,
    `ResolvedContext::package_active("Tk")`, under §5.3's policy: the
    ambient closure always, the lenient hosted world under `open`,
    nothing under `closed`. The surviving `Core(Tcl)` row on those specs
    is specificity data only — `specificity_breadth` counts it to
    reproduce the coexisting `get_for_dialect` mask popcount exactly —
    and goes with the mask under ledger C1.
  - **Classification.** `is_closed_world_package` gained the conjunct Tk
    forces: a package that is ambient *somewhere* and hosted *elsewhere*
    is a library with an ambient host, not a vendor runtime. Reading
    ambience alone would have made Tk closed-world the instant the pilot
    placed it and erased Tk from plain Tcl.
  - **Holds resolved (ledger F4).** `DocumentEnvironment::is_tk` is
    private with two identity-only callers; `can_host_package` moved onto
    the context as a pure placement query (no lenient special case — the
    `tcl` sink declares its own Tk placement);
    `ResolvedContext::with_authoring_mask` and the second leaked
    document-context are deleted, because the ambient placement *derives*
    the `TK` bit that used to be injected. The analyser-vs-unit `tk`
    asymmetry is ruled **permanent** and narrowed to catalogue identity
    (see the centralisation companion).
  - **The W120 ruling.** W120's suppression rule was already "the package
    is ambient here", so the ruling is the placement: silent under `tk` /
    `wish` (the interpreter loaded Tk before the first byte — there is no
    `package require Tk` to write), unchanged under every plain-Tcl
    environment. Pinned by
    `w120_is_silent_under_the_tk_environment_and_nags_under_plain_tcl`
    and `tk_checks_activate_on_the_ambient_placement_not_the_environment_name`.
  - **Range targeting.** `supports Tk 8.5-8.6` gates Tk items on the Tk
    axis through the P1b machinery unchanged — the package half of §5.4
    needed no new code, only the acceptance case
    (`a_tk_range_warns_on_an_item_the_older_tk_lacks` and its two axis /
    ambient controls).

  **One enumerated behavioural delta**, pinned by
  `tk_is_closed_out_of_closed_worlds` and allowlisted in the P1-E
  acceptance sweeps: a **closed** world (`bpf`, `spectcl`) no longer
  resolves the Tk surface. `package require` is not part of either
  language, so `wm` was never callable there; the old profile mask
  admitted it only because `TK_AND_TCL` unions the whole Tcl ladder.
  Every open world — the five plain-Tcl releases, the lenient sink, the
  F5 shells, the EDA shells, `expect` — answers exactly as before.

  **What a surface-to-pack conversion still needs** (the remainder of
  this phase): the invocation-refinement descriptor for Tk's 53
  `subcommand_forms` sites; SpecTcl vocabulary for the widget-class and
  option-database shapes the Rust specs carry natively
  (`creates_instance_at`, `tk_geometry`'s `TkGeometryManagerSpec`, the
  shared `common.rs` enum value sets, the `ttk::` style tables, the
  `TK_NAMESPACED_CLASSIC_ALIASES` co-registration); the B11
  lowercase-`tk`/uppercase-`Tk` predicated co-provide and loader-alias
  model, which the placement layer does not yet express; and the I10
  behavioural-parity harness. Until those land, `Tk` is a package with a
  real placement whose *declarations* are still compiled Rust — which is
  exactly the split this phase set out to prove is possible.
- **P4 — smaller packages.** iapps/tmsh (splitting the shared
  `IAPPS|TMSH` sources into two packs + shared `values`/`descriptor`
  tables), expect, and the EDA environment shells move into their packs
  incrementally with the same behaviour and trust gates; the Rust
  catalogue shrinks to core. **The F5 half is additionally gated on the
  §0.2 evidence programme**: `BigIpExecutionContext` keying, the
  conformance corpus with its acceptance-matrix coverage, the tmsh
  syntax axis, and the policy overlays land before any F5 row migrates;
  unmeasured contexts ship as `Unknown`, never as inherited defaults.
  The evidence lane (#15) discharged the typed half of that gate —
  `BigIpExecutionContext`, `EmbeddedRuntimeEvidence`, the tmsh syntax
  axis, the iApp action overlays, and the hermetic corpus — so what P4
  still owes is: (a) **corpus-generated rows** (the F5 catalogue entries
  are still hand-authored; the corpus asserts them rather than producing
  them) plus the transcript-schema validator and the prose/rows/tests
  drift gate; (b) ~~the **21 recorded event-context divergences** and
  the missing `matches` word operator resolved in registry data one way
  or the other~~ — **done (#27)**: the fifteen open event cells were
  moved in per-command registry data and the bare `matches` is the
  trunk's tenth word operator, leaving only the six deliberate
  `RULE_INIT` compile-acceptance rows diverging (§0.2's F8 row has the
  detail); (c) the **role/policy visibility overlay** (F4) and the
  `tcl_platform` CMP-effect overlay (F5), which this lane recorded as
  evidence but did not wire as overlays; and (d) the acceptance
  matrix's remaining coverage, which needs another appliance run, not
  code: one supported 17.x build, one older build, a restricted-role
  tmsh column, and the two APL contexts (E4 step 6).
- **P5 — tcllib by adversarial module.** Importer-driven from release
  snapshots (2.0 now; 1.17–1.21 as history — **Q9** decides depth and
  bundling), per-module packs mirroring tcllib's structure — **starting
  with the hostile shapes**: `struct::tree`, `struct::graph`,
  `fileutil::traverse`, and `oo::dialect`, scaling to the long tail only
  after those dynamic surfaces are honest. Multi-train cases (`struct`,
  `struct::graph`) are the version-set acceptance tests.

  **Status: the package model is landed on the real surface; the
  declarations stay compiled** — the same split P3 proved for Tk, now
  proved for 200 independently versioned third-party modules.

  - **Per-module identity, from the sources.**
    `tcl_registry::model::tcllib::TCLLIB_MODULES` is a 200-row census
    read out of `tmp/tcllib-2.0`: each module's `package require` name,
    the **trains** its own `pkgIndex.tcl` offers, its
    `package require Tcl` floor, and the file each fact came from. A
    module's declarations now carry that set as their applicability
    (`declarations_for_spec`'s package row), on the module's **own**
    axis. No row is a point: a train is a requirement, so single-train
    `csv 0.10` is `[0.10, 1)` and seven modules — `md5`, `sha1`, `snit`,
    `struct::tree`, `struct::graph`, `doctools::idx`, `doctools::toc` —
    are genuine **parallel trains**, two disjoint ranges each. That is
    the multi-train truth of §3.2 as data rather than as a plan.
  - **Hosted, never ambient.** No compiled environment places a tcllib
    module, so none is closed-world or placement-gated: visibility stays
    §5.3's lenient `open` rule with W120 owning the nag, and the
    **floor** comes from the document's own `package require`. Pinned by
    `hosted_modules_are_never_ambient` over the whole table, so an
    environment that later placed one ambient would fail the build
    rather than silently become a closed world. Enumerating 200
    placements per plain-Tcl environment would add data that says
    nothing a `Placement::Requirement` with no point primary does not
    already say — the ruling is the invariant, not the rows.
  - **Two identity defects the sources proved.** `sha2` is a *namespace*
    (`::sha2::sha256`); the package is `sha256`
    (`sha1/sha256.tcl` ends `package provide sha256 1.0.6`). `ooutil` is
    a *directory*; the package is `oo::util`. Both are renamed, so
    `package require sha256` / `package require oo::util` — what tclsh
    actually accepts — now activate the surface. Eleven further names
    the catalogue uses (`tcl::chan`, `fileutil::magic`, `pt`,
    `pt_export_api`, `tcl::combine`, …) are namespace prefixes or
    manpage identities that no `package provide` backs; they are
    recorded in `UNBACKED_PACKAGE_NAMES` with what the sources say
    instead, and `the_identity_census_is_closed` fails the build if a
    new unbacked name appears, so the list can only shrink.
  - **The Tcl-core floor, applied from evidence.** The old
    `tcllib_package_dialect_floor` was a two-name `match` (`report`,
    `stooop`) while every other module carried the identical
    `package vsatisfies [package provide Tcl] 8.5 9` guard and was
    offered under `tcl8.4` regardless. It now reads the per-module floor
    off the census, so the whole distribution is gated consistently —
    §5.4's "package interplay" at ladder granularity, under D5's
    "oldest never over-reports".
  - **Range gating.** `# tcl-lsp: supports struct::tree 1.2-2.2` warns on
    `::struct::tree::prune` (W150, naming `struct::tree 2.0` and the
    failing targets); `supports struct::tree 2.1-` is clean;
    `package require struct::tree 1.2` gives the single-floor W135. The
    axis controls are stronger than P3's could be, because a tcllib
    module can be gated against **another tcllib module**: a
    `supports struct::list 1.2-2.2` declaration says nothing about the
    `struct::tree` axis, and neither does a core `supports tcl 8.5-9.0`
    (invariant I2).
  - **The adversarial cases.** `struct::tree`'s walker is modelled
    across both trains: `walk`'s 2.x `loopvar script` body (resolved
    positionally, since the option prefix is variable-length), the 1.x
    `-command` option carrying `retired: "2.0"` and the `Exactly(0)`
    appended arity its `string map` + `uplevel` really has, `walkproc`
    with `introduced: "2.0"`, and `::struct::tree::prune` as a real
    command with `CompletionCodeDomain::Exact([Other(5)])`.
    `http::geturl` gained the whole 20-option table it never had, with
    the four callbacks' measured appended arities (1/2/3/3, all
    deferred), the release deltas the four bundled Tcl trees prove, the
    `-query`/`-querychannel` conflict, and its credential and
    network-sink facts intact. `bibtex::parse`'s five proven
    `-command`-versus-SAX conflicts are now `OptionConstraint` rows
    beside its existing cross-option timing hook.
  - **What stays inexpressible, with the field each needs.**
    (a) *Scoped completion codes* (E-R6): `prune`'s code 5 is
    loop-adjacent **only inside a `walk` body**. A body slot carries a
    timing and a kind, never a set of codes its command consumes; the
    missing field is `body_completion_codes: &[(u8, CompletionCode,
    &str)]` on `SubCommand`/`OptionSpec`, so the `BREAKS_LOOP` machinery
    could be scoped to that body. Until it exists `prune` carries no
    control-flow trait, because `CONTINUES_LOOP` would be a lie the CFG
    builder acts on. (b) *Instance-method version gating*: an
    `ObjectClassSpec` method (and its options) already carries a
    `Lifecycle`, but the analyser's instance-method path has no
    diagnostic site, so `walkproc`'s `introduced: "2.0"` and
    `walk -command`'s `retired: "2.0"` are declared and unread — the
    missing piece is a consumer (`record_lifecycle_site` on
    instance-method resolution), not a field. ~~(c) *Option-requires
    relations*~~ **closed by E-R14 (O1).** `bibtex::parse`'s `-command`
    requires `-channel` is now
    `option_requires -command {-channel}`, and its companion "`-channel`
    excludes the inline text word" is
    `option_forbids -channel {{arg 0}}` — the option-to-positional half
    of the same gap. Both are declared on the shipped spec and both are
    checked natively. (d) *Callback substitution
    sets*: `struct::tree` 1.x's `%n`/`%a`/`%t` and `struct::graph`'s
    placeholders need `-substitutions {%n node …}` on the prefix slot;
    `CallbackTaintInput::TkPercent` names the spelling but only as a
    taint colour. (e) *Callee-frame expressions*:
    `math::calculus::integralExpr`'s fourth argument is an `expr`
    evaluated in the **callee's** frame against a callee-provided `x`;
    `ArgRole::Expr` is right about the language and wrong about the
    scope, and there is no `-scope callee -provides {x}`. (f)
    *Transitive core floors*: `processman` declares Tcl 8.5 and requires
    `cron 2.0`, which declares 8.6 — the census records direct floors
    only.
  - **Two enumerated behavioural deltas**, both pinned: tcllib commands
    whose module declares a Tcl floor are no longer offered below it
    (every 8.5-floor module loses `tcl8.4`; `defer`, `generator`,
    `websocket` and the other 8.6-floor modules lose `tcl8.5` too), and
    the two renamed packages answer to their real names.
- **P6 — jim rebased.** The jim branch re-lands on the new model: its
  measured grammar data and probe scripts carry over as **release ×
  build-profile** columns (never one default-build column as the family
  truth — H3); its 134-file pack becomes either multi-row availability on
  shared core specs or a jim surface pack (**Q6**); its ten profiles and
  `JimVersion` disappear into the `jim` family ladder. (**Q10** may
  reorder P6 earlier if the branch should merge early.)

  **Status: the wiring tax is discharged; the surface pack stays Q6.**
  The branch (`claude/jimtcl-dialect-rust-5q48z8`) is **not merged into
  the redesign line**, so none of §1's measured tax was present to
  delete: no `jim0.76`–`jim0.84` profiles, no `JimVersion`, no 76
  re-authored core commands. What P6 did instead is make the model
  *hold* jim, so the tax cannot be re-incurred — and then check that
  claim against the upstream sources rather than against the branch's
  summary of them. Every value below is read from `jim.c`, `auto.def`,
  `utf8.h` and `jim_tcl.txt` at the upstream tags 0.76 … 0.84.

  - **Nine profiles → one environment and one ladder.** `jim` (aliases
    `jimsh`, `jimtcl`) is a single `EnvironmentDefinition` targeting the
    whole `0.76-0.85` span on the **jim core axis**, with no point
    primary — a document that names no jim release is not judged against
    one. Grammar is `grammar(Family::Jim, release)`: one measured
    `LexerGrammar` plus one struct update (expr comments arrive at 0.81),
    where the old model needed nine rows because a profile carries
    exactly one resolved grammar. `expr` is five struct-update values
    over the same ladder. **Ten user-facing surfaces grew by zero rows**:
    the editor catalogues are generated from `DialectProfile::all`, which
    P6 does not touch, and `gen-editor-dialects` / `gen-editor-extensions`
    report the same 19 languages and 24 extensions as before.
  - **No parallel version enum.** `Release` on the jim ladder already
    was the replacement for `JimVersion`; P6 adds the ingress that makes
    it reachable — `# tcl-lsp: supports jim 0.81-0.84` — and the ordering
    is ladder-ordinal, so the branch's lexical `"0.76" >= since`
    comparison (which breaks at `0.100`) has no home to come back to.
  - **76 re-authored core commands → one ancestry edge.** `Family::Jim`
    now carries an `Ancestry` — but a `Lineage::Reimplementation`, not a
    fork: Jim shares no source with Tcl ("a small footprint
    reimplementation of the Tcl scripting language", `jim_tcl.txt`), so
    calling the edge a fork would have been a provenance lie for the sake
    of reusing a mechanism. The edge is anchored at **Tcl 8.6** — "a
    significant subset of the Tcl 8.6 command set" — and carries the
    *surface* only: every lexical and expr axis is Jim's own, and the
    test asserts the two grammars differ. `provider_active` walks the
    edge, the resolved context takes an 8.6 point primary on the Tcl
    axis, and the derived authoring mask is the 8.6 line, so `set`, `if`,
    `proc`, `lassign`, `lmap` and `dict` resolve for a jim document out
    of the shared catalogue. The generalisation paid for itself
    immediately: the registry's lineage walk lost its `if ancestor ==
    Tcl { F5_FORK_POINT }` special case and the authoring mask lost its
    per-family `DialectSet::TCL84` hardcode — both now read
    `Ancestry::anchor`.
  - **`--minimal` is a real `BuildProfileId`, and the build axis is
    semantic twice over.** `BuildProfileId::{JimFull, JimMinimal}` carry
    measured capability records: `--minimal` compiles out `JIM_UTF8`
    (so `utf8.h` defines `utf8_strlen` as `strlen` — "No utf-8 support.
    1 byte = 1 char") and `JIM_MATH_FUNCTIONS` (so nineteen of the
    twenty-six mathfunc rows are not compiled). The second proof is
    sharper and was not in the design: **`auto.def` flipped its default
    at 0.82** ("Note that full is now the default"), so through 0.81 a
    bare `./configure` gave neither utf8 nor math. `CapabilitySet::
    canonical` is therefore keyed by `Release`, not by `Family`:
    `expr {sqrt(4)}` is a syntax error on a stock `jimsh 0.81` and
    answers `2.0` on a stock `jimsh 0.82`, from the same command with no
    flags. A build axis that were metadata could not say that.
  - **The mathfunc gate is per function, not per build.** Seven rows —
    `int`, `wide`, `abs`, `double`, `round`, `rand`, `srand` — sit
    *outside* `#ifdef JIM_MATH_FUNCTIONS`, so a `--minimal` build has a
    mathfunc surface of seven, not of zero. `MathFunc` gained
    `needs_math_extension` and `CoreProfile::mathfunc` consults it; every
    C Tcl and F5 row is `false`, so nothing else moved.
  - **Range targeting and I2.** `resolve_declared_targets` recognised
    only the literal name `tcl` as a family, so `supports jim 0.81-`
    minted a fictitious *package* axis named `jim` and switched range
    mode on against it. It now matches any `Family::name()` and honours
    the declaration **only when the document's core is that family**:
    `supports jim …` under `tcl8.6` and `supports tcl …` under `jim` are
    both dropped, because each is a claim about a ladder the document is
    not on. Everything below the ingress needed no jim-specific code at
    all — `targets_from_clauses`, `next_line_bound`, `ladder_coverage`,
    `ladder_releases_in`, `targets_outside_window` and
    `available_at_targets` all read the family's own ladder, so
    `supports jim 0.76-0.79` names `0.76 0.77 0.78 0.79`, clamps
    `0.81-` to `0.84`, and answers `None` on the Tcl axis. The leak is
    unrepresentable rather than merely untested: intersecting a jim
    target set with a Tcl one is a typed `AxisMismatch`.
  - **Five corrections the sources forced on §3.1.** (a) `lt`/`le`/`gt`/
    `ge` are **not** on every modelled Jim release: they are absent from
    `Jim_ExprOperators` at 0.76–0.79 and arrive at **0.80**, so offering
    them under `jim 0.78` would offer a syntax error. (b) Precedence is
    not purely a per-*family* fact: `**` bound at **250 and
    left-associative** at 0.76 and at **120, right-associative** from
    0.77, so `expr {-2 ** 2}` is −4 on 0.76 and 4 from 0.77 (the unary
    minus, at 150, overtakes it) and `expr {2 ** 3 ** 2}` is 64 at 0.76
    and 512 at 0.79. The table is release-keyed for that one row, and the
    binding powers are now the whole `OPRINIT` table rather than the
    comparison block alone. (c) The mathfunc set is release-gated within
    the family: 0.76 ships twenty-three, and `atan2`, `hypot` and `fmod`
    arrive at 0.77 to make the twenty-six §3.1 pins. (d) Jim's `${…}`
    close rule is `FirstClose`, the 8.x rule, and it skips no leading
    BOM — the interim value (Tcl 9's grammar wholesale) was wrong on
    both. (e) The 0.81 arity flip is `#ifndef JIM_COMPAT`, so a
    `--compat` build still concatenates; `--compat` is off unless asked
    for, so the ladder carries the default build's value and the other
    column is recorded as the next probe.
  - **The mathfunc disposition.** The rows are no longer empty. Twenty-six
    at 0.77+ (the count §3.1 pins), twenty-three at 0.76, each with its
    introducing release and its `#ifdef` status, read from the `OP_FUNC`
    block of `Jim_ExprOperators`. §3.1's five named absentees are
    **confirmed against the table**: `entier`, `bool`, `min`, `max` and
    `isqrt` appear at no modelled tag, and the test asserts that each of
    them *is* a real C Tcl 8.5 function — which is exactly why the
    `TclVersion`-floor keying in `tcl-syntax/src/expr/mathfunc.rs` would
    have offered all five under Jim.
  - **Three honest gaps, each with the field it needs.** (a) *Numerals*:
    `JimNumberBase` accepts `0x`/`0o`/`0b`/`0d` and leading zeros do
    **not** imply octal, which is none of `NumberSyntax`'s three values.
    `Tcl90` ships as the closest (right that `010` is ten; wrong only in
    accepting Tcl 9's `_` separators, which Jim rejects) because `Tcl85`
    would be wrong the dangerous way round. The missing piece is a
    `NumberSyntax::Jim` variant — 231 sites across 43 files, a lexer
    change, not a data edit. (b) *Byte counting*: a non-utf8 Jim build
    counts bytes, a third rule `StringCharacterModel`'s two **Tcl**
    models cannot express, so `character_model` answers `None` (every
    consumer abstains) and the measured fact travels on
    `CapabilitySet::utf8_character_model`. The missing piece is a
    `StringCharacterModel::Bytes` variant and its `count_for` agreement
    rule, which changes constant folding for every dialect. (c) *Live
    lexing*: the analyser still takes its `LexerConfig` from the interned
    `self.profile.grammar`, and `jim` sinks to the permissive fallback
    profile exactly as `tk` and `tcl` do, so the measured Jim grammar is
    not yet what lexes a jim document. That is ledger **C1**'s interned-
    `DialectProfile` seam, not a P6 regression — but until it retires,
    `braced_var` and the BOM rule are the fallback's. The five further
    lexical axes the branch measured (word separators, brace
    continuation, quote termination, `$(…)` variable syntax, list parse)
    remain new `LexerGrammar` fields, unchanged by this lane.
  - **The probe matrix, run rather than assumed.** Five `jimsh` binaries
    were built from the upstream tags for this lane — 0.76 `--full`,
    0.79 `--full`, 0.81 default, 0.84 default, 0.84 `--minimal` — and
    every claim above is a transcript, not a reading of the C alone:

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

    The 0.81-vs-0.84 `sqrt` column is the configure-default flip; the
    0.84 `--full`-vs-`--minimal` column is `--minimal` alone; `int` beside
    `sqrt` is the per-function math gate; `string length` in both forms
    is the character-model delta and the byte-counting gap; and `1_000`
    is the one place `NumberSyntax::Tcl90` over-accepts for Jim. This
    corpus is transcripts on disk rather than a hermetic in-tree fixture
    set — the natural follow-on is a jim probe corpus shaped like the F5
    one (§0.2's evidence layer), keyed by `(release, configure flags,
    platform, commit)` exactly as H3 requires.
  - **What P6 deliberately did not do.** No jim command specs were
    authored: the jim surface pack is **Q6**, and the inherited Tcl 8.6
    surface therefore over-admits what `jim_tcl.txt` says Jim lacks —
    threads, coroutines, and command/variable traces. That
    over-admission is recorded as data
    (`JIM_ABSENT_FROM_THE_INHERITED_SURFACE`) with a test that fails the
    day the pack lands, so it is a named gap rather than a silent one.
- **P7 — irules surface pack-expression (optional, deferred).** Requires
  the seven words + `event_requires` draft-model fix; the dialect
  (grammar, structure) and closed-world policy stay compiled regardless.
  **Q5**.

  **Status: deliberately not done, per Q5's ▸ deferral — and the
  prerequisite is now six-sevenths met.** P2-H/P3 landed loader readers
  for six of the seven words including `event_requirement_form` with its
  nested `event_requires` block; only `bpf_op` is still unread, and it is
  not an iRules word. So what P7 waits on is a decision, not a
  prerequisite: the iRules command surface remains compiled Rust, and the
  dialect and closed-world policy stay compiled either way. Kept in §11 as
  an owner decision rather than an engineering remainder.
- **P8 — sweep.** Docs (the ~15 design/contract docs and ~10 KCS pages
  the inventory flagged, README dialect tables), samples,
  `docs/generated` regeneration, deletion of the last transitional data.

  **Status: done (2026-08-27) for the documentation half.** Every design
  document in this programme was reconciled with what shipped: phase and
  ruling statuses made true, the claims the implementation disproved
  corrected *where the original claim lived* (§3.1's Jim values, §2's F5
  rows, §9's defect list, P5's tcllib identities), and the open items
  collected once in §11 instead of scattered across six documents.
  `docs/generated` regenerated continuously per lane rather than in this
  sweep — the callback-surfaces, Zed command data, diagnostic tables and
  editor artefacts each regenerated with the change that staled them, and
  `make xtask-check` reports nothing to regenerate. **Not done in P8**:
  the KCS "Applies-to" vocabulary regeneration (ruling R9) — it depends on
  environment names becoming the controlled vocabulary, which needs the
  T-row payload changes P1 left open — and the "deletion of the last
  transitional data" clause, which is exactly the ledger's remaining rows
  (C1, C6, C8, and the T payloads). §11 carries both.

## 9. Defects found during research (fix regardless of this design)

Each entry carries its **final state** as of 2026-08-27. Everything still
open here is also in §11.

1. **Salsa lexer-config truncation**: `LexerCfgKey` interns only
   `expand_syntax` + `irules_brace_separator` and rebuilds with
   `..LexerConfig::default()`, so 8.x documents lex `${a{b}c}` and
   `\x4142` under 9.0 close/escape rules on the memoised path
   (`rust/tcl-lsp-db/src/lib.rs:3017-3034`; `ProcBodyKey` likewise at
   `:1720-1750`; the doc comment above it is stale).

   **State: still open, narrowed.** The key gained a third field
   (`brace_line_continuation`, added with the F5 N-rules axis) and the
   stale doc comment is corrected, but the truncation itself stands:
   `LexerCfgKey::to_config` and `lower_proc_body` both close with
   `..LexerConfig::default()`, whose `braced_var` is `Tcl9Nesting`,
   `escapes` is `Tcl90` and `leading_bom` is `Content` — while
   `LexerConfig::from_grammar` *does* set all three per dialect. So the
   memoised path still answers an 8.x document under 9.0 close/escape
   rules. Widening the key is mechanical but changes which documents share
   a `compilation_unit` build per edit, so it is a behaviour change with a
   performance dimension, not a comment fix. It is the same seam ledger
   **C1** retires (the interned `DialectProfile`); §11 carries it.
2. **Stale owner-map claim**: `AGENTS.md:74` "tmsh mode per dialect" — no
   such lexer mode exists.

   **State: withdrawn — the claim was right and the defect was wrong.**
   Live measurement ([measurements](bigip-irule-parser-measurements.md)
   §4a) showed a tmsh `cli script` reproduces the entire fork grammar
   byte-identically to TMM — the R-rules, the N-rules, the inert `{*}`,
   the expr word operators. A tmsh lexing mode **is** required; it selects
   the `f5-tcl` trunk grammar, which the shipping `f5-tmsh` catalogue row
   now does (`GRAMMAR_F5_TCL`, P1-G). The owner map needed no correction.
3. **Contradictory prose on iRules' expr base**: `grammar.rs:85-88` and
   `expr_lexer.rs:318-319` say iRules has a `None` expr base; the
   catalogue sets `Some(V8_4)` (`profile.rs:623`). Behaviour is right,
   prose is wrong.

   **State: fixed (P8).** Both comments named the wrong dialects. The
   `None`-base profiles are `f5-bigip` and the two off-catalogue sinks
   (`PLAIN_TCL`, `TK_PROFILE`); `f5-irules` has carried
   `Some(TclVersion::V8_4)` throughout. Both doc comments now say so.
4. **Seven ratified DSL words silently unimplemented** (§6.2 list) —
   symptoms of the `DraftOpaque`-masks-`LoaderGap` blind spot (§6.3).

   **State: six fixed (P2-H, P3), one deferred on a model change.**
   `result_stability` (command *and* subcommand scope),
   `event_requirement_form` with its nested `event_requires` block,
   `data_collection`, `body_scope`, `side_switch_target` and
   `event_handler_priority` all have loader readers at the shared
   row-reader seam of both loaders. `bpf_op` is deferred: it needs a named
   `id → &'static BpfOpSpec` catalogue in `tcl_registry::bpf_op` before
   `bpf_op -native ID` can resolve anything (§11). **The blind spot that
   produced all seven is itself unfixed** — §6.3's loader-direction
   round-trip gate does not exist, so `bpf_op` is ratified and unread and
   no CI job says so.
5. **`# tcl-dialect: tk` inconsistency**: used by `tk_dialect.rs` e2e
   tests and module docs, rejected by the server-side directive tier
   (`tk` is not in `KNOWN_DIALECTS`).

   **State: still open.** `detect_dialect_directive` still gates on
   `KNOWN_DIALECTS`, which still has 18 names and none of them is `tk`, so
   `# tcl-dialect: tk` still abstains and falls through to the next
   detection tier. §5.1's "finally coherent" is intent, not shipped
   behaviour. The fix is not local: `KNOWN_DIALECTS` is also the CLI's
   `--dialect` choices, the MCP `dialect_schema` enum and the explorer
   dropdown — exactly the *payload* halves ledger rows T1/T3/T6 hold open
   — so adding one name there is a user-visible surface change. §11.
6. **Ungated hand-written map**: `editors/sublime-text/plugin.py`
   `_SYNTAX_DIALECT_MAP` (and its missing `tcl8.6`/`tcl9.1` rows).

   **State: still open.** The map is still hand-written and ungated
   (ledger F12/T13). It has since grown `Tcl 9.1`, and maps bare `Tcl` to
   `tcl8.6`, so the two rows this entry named are covered — but
   `f5-tmsh`, `spectcl`, `bpf` and `microchip-libero-eda-tcl` have no row
   and nothing fails when the catalogue moves. §11.
7. **Stale doc counts**: `dialect-detection.md`'s 16-name list vs 18;
   `dialect-profile-model.md` §8 "16 catalog entries"; the `spec-author`
   skill's vocabulary list stopping at 1.1.

   **State: the two doc counts fixed (P8); the skill still stale.** Both
   design docs now say 18 and `dialect-detection.md` lists the two names it
   was missing (`microchip-libero-eda-tcl`, `spectcl`). The `spec-author`
   skill (`ai/claude/skills/spec-author/SKILL.md`) still tells authors to
   declare `speclib <name> 1.1` and still describes 1.1 as the newest
   vocabulary — two majors behind the loader's `2.0`. That is ledger row
   **T9**, a P2 deliverable that did not land; §11.
8. **Withdrawn** (originally: lexical version comparison in the jim
   branch's lifecycle gating). Incorrect — the branch's gating resolves
   through `Lifecycle::introduced_in` → `version::compare`, which walks
   numeric segments: `compare("0.100", "0.76")` is `Greater` and
   `meets_min("0.100", "0.76")` is true. Lexical and numeric orders
   merely coincide across 0.76–0.84, and the branch now pins the
   property at the two inputs where the orders diverge. The unified
   `Release` comparator (§3.1) remains a *unification* win — one
   ordering type instead of two parallel enums — not a bug fix.

## 10. Owner questions and their outcomes

Recommendations marked ▸. These gated P0; the owner directed the build to
proceed on the ▸ recommendations as working rulings, so **every question
below has an outcome**, recorded in the disposition table first and then
in the question's own text. Anything the table marks *open* is repeated in
§11 with what it blocks and what would resolve it — §11, not this section,
is the place to look for outstanding work.

| Q | Outcome |
|---|---|
| Q1 core surface source of truth | **Ruled and sequenced as recommended.** `dialect` blocks landed in P2-H and pack-declared families convert (P3). The endgame — shipped cores authored as SpecTcl, compiled by `tcl spec build --emit rust` — is **not built** (ruling R7); §11 |
| Q2 pack-declared environments | **Ruled ▸ yes; the mechanism landed and is live, but no shipped pack uses it.** `environment` blocks parse, validate, convert to `EnvironmentDefinition`s at their tier's provenance, register through the one publish point, retire when their pack leaves the workspace, and route documents by claimed extension — proved end to end by a server e2e test over a *workspace* pack. **Not one of the eight bundled `specs/*.tclspec` packs declares an `environment` block**, so all six EDA catalogue shells (plus `f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`, `tk`, `spectcl`, `bpf`) are still compiled-in. The "fully centralised end-state" is a data move that nobody made; §11 |
| Q3 fate of the borderline three | **Ruled ▸ as recommended, partly enacted.** `f5-bigip` in an `available` row is an error and never translates (U2), so it has left the Tcl availability axis. `spectcl` and `bpf` are `Closed`-world environments — and U3 found the *measured* reason they cannot yet carry membership as `available` rows: neither environment declares an ambient package, because both surfaces are compiled. They remain catalogue profiles; §11 |
| Q4 selection UX | **Ruled ▸ flat per-release names, and they are what shipped** — P6 confirmed the generated editor catalogues stayed at 19 languages and 24 extensions when the jim ladder landed, which is the property this ruling bought |
| Q5 iRules pack-expression (P7) | **Ruled ▸ deferred**, and deliberately not done. See P7's status |
| Q6 family surface composition | **Open.** P6 authored **no** jim command specs; the family inherits Tcl 8.6 through one `Ancestry` edge, which over-admits threads, coroutines and traces. `include` landed (P2-H) so the composition word exists; §11 |
| Q7 strictness defaults | **Ruled ▸ as recommended; two of three policies are load-bearing.** All three `WorldPolicy` values exist and `package_provider_active` distinguishes all three, but the *availability* query `package_active` treats `AmbientPlusRequire` exactly like `Open`, and the tcllib-excluded-from-iApps rule is still the subtractive `DialectSet` difference §3.3 says it stops being; §11 |
| Q8 require position sensitivity | **Settled by review B2/H5** — the assistance/semantic split is typed and enforced. The residual UX question (should the assistance view surface an ordering hint by default) is **unanswered and unbuilt**; §11 |
| Q9 tcllib depth and shipping | **Ruled ▸, and the ruling's numbers were wrong.** P5 read the census from tcllib 2.0 and found **200** independently versioned modules, not 135; seven ship genuine parallel trains. The "windows derived back to 1.17" half did **not** happen — the census is 2.0-only, with release deltas expressed as core gates where no bundled tree witnesses a boundary. Bundled-with-the-binaries stands |
| Q10 jim branch sequencing | **Ruled ▸ rebase first, and that is what P6 did.** The branch `claude/jimtcl-dialect-rust-5q48z8` is still unmerged; P6 rebuilt the model's jim support from the upstream sources instead of from the branch |
| Q11 speclib numbering | **Ruled ▸ yes and shipped.** `NEWEST_VOCABULARY_VERSION` is `2.0`, `VOCABULARY_VERSION` bumped once, `dialects`/`ambient_package` are documented-legacy spellings, and B13's fail-closed classes ride with it |
| Q12 invocation refinement | **Ruled ▸ the declarative descriptor — and it is not built.** This is the single largest blocker on any surface-to-pack conversion (Tk's 53 `subcommand_forms` sites); §11 |
| Q13 `DialectSet` residue | **Ruled ▸ delete outright; partially done.** `DialectSet::parse` has no name-ingress caller and the bits are no longer an ingress vocabulary, but the type survives behind the semantic-facts bundle (ledger C1); §11 |
| Q14 keyed version UX | **Ruled ▸ CLI/config knobs, unchanged.** `--bigip-version` / `--tool-version` still set environment placement floors; no per-package override was added |
| Q15 primary target default | **Open.** P1b explicitly deferred explicit `primary` selection for multi-target projects — the primary is the environment's, and a declared range never moves it; §11 |
| Q16 range diagnostics shape | **Ruled ▸ a dedicated family, and it shipped**: W150 (not available across the range) and W151 (numeral-grammar divergence) |
| Q17 range strictness and defaults | **Ruled ▸ warnings by default, assistance permissive — and that is the shipped split.** W150/W151 fire at Warning severity only for items clean at the primary, and a primary failure always outranks a range warning at the same word |
| Q18 dynamic environment scope | **Ruled ▸ pack-name-prefixed ids, and shipped**: a converted `dialect` block gets a namespaced `PACK/DIALECT` id; compiled canonical names and aliases are reserved at every tier |
| Q19 trust defaults and UX | **Partially ruled.** E-R2's registration-time tier gate landed (workspace/studio tiers cannot override compiled names, extend compiled environments, or declare dialects), which is (a)'s half. Per-pack consent for security-weakening overrides and provenance-in-hover — (b) and (c) — are **not built**; §11 |
| Q20 build-profile scope for the Tcl family | **Ruled ▸ one canonical profile per release with the axis representable, and that is what shipped.** The axis earned its keep three times outside `tcl`: `JimFull`/`JimMinimal`, `F5Scriptd32` (iApps' 32-bit build), and P6's release-keyed `CapabilitySet::canonical` |
| Q21 realm-analysis depth | **Ruled ▸ single-realm first, and that is exactly P1a's scope.** The multi-realm `AnalysisWorld` map is the named remainder; §11 |
| Q22 stub fate (R1) | **Ruled ▸ yes; not enacted.** `rust/tcl-registry/src/stub_overlay.rs` still exists as a separate overlay type consulted per consumer; §11 |
| Q23 the variable axis (R2) | **Ruled ▸ yes; half enacted.** `special_vars::resolve_dialect` is deleted (P1-G, ledger C2) so the private dialect-name ingress is gone, but special variables are still a compiled Rust table, not SpecTcl declarations; §11 |
| Q24 `tclpkg.tcl` targets vs MVS (R6) | **Ruled ▸ the `supports` directive; half enacted.** `# tcl-lsp: supports NAME RANGE` ships and drives W150/W151. The manifest's multi-clause `tcl` constraint grammar is **not** ingested, and the three version comparators have not collapsed; §11 |
| Q25 hook `ctx` vocabulary (R5) | **Open.** No `environment` key was added to the hook `ctx` dict; `dialect` remains the only spelling; §11 |

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

## 11. The open-questions ledger

**Everything the #1631 programme did not close, in one place.** Compiled
2026-08-27 by the P8 sweep from all seven documents of this programme, their
phase reports, and a re-read of the tree. Each row says *what it is*, *what
it blocks*, and *what would resolve it*. Nothing outside this section
should be read as an outstanding item: where a phase, ruling, or ledger row
elsewhere says "still open", it points here, and where it does not point
here it is done.

Three things are deliberately **not** in this ledger: work that landed
(§8, §9, §10 record it), items the owner ruled out (a per-major `tcl`
family split; `tcl8`/`tcl9` named-range sugar; contraction of a programmed
pack back into a program), and the six *deliberate* F5 divergences the
conformance corpus records, which are meant to diverge (§0.2, F8).

### 11.1 Owner decisions never ratified

These were implemented, adopted as working rulings, or recorded for review
without an owner answer. Each is reversible and each names what a re-ruling
would re-cut.

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| O1 | ~~The option *requires* relation vocabulary.~~ **RESOLVED (owner, 2026-08-27) and LANDED.** `OptionConstraint` is now `OptionRelation`: a `RelationKind` (`MutuallyExclusive`, `Requires`, `RequiresOneOf`, `Forbids`) over an optional subject and a list of `RelationTerm`s (`Option`, `OptionValue`, `Argument`, `ArgumentValue`), plus `OptionPlacement` so the checker knows where a command's options live. `OptionRelation::evaluate` is the whole declarative checker — a few slice scans, **no VM entry** — and the `constraints` hook is reached only when a spec declares one *and* every relation reported nothing. Four SpecTcl statements (`option_conflict`, `option_requires`, `option_requires_one_of`, `option_forbids`) at both loader seams, with an export round-trip gate over all four kinds and all four term shapes. W147 covers the exclusions, **W152** the requirements. Measured on a 40-call-site option-heavy corpus: 33 option-bearing sites walked, 14 judged, 50 relations evaluated natively, **0 entered tclvm** | — | Done. See §6.2's `constraints` contract |
| O2 | ~~The M9 dead axes.~~ **RESOLVED (owner, 2026-08-27): delete them all now — LANDED, with one exception.** `ProfileSpec::capabilities`, `EventRequires::init_only`/`::capability`, `Traits::PASSWORD_OPTION`, `Traits::IRULES_DATA_GETTER`, `xc_operation` (command and subcommand) and the whole `arg_rows` machinery (`VersionedArgRow`, `ProjectedArgs`, `ArgTables`, `project_arg_rows`, `arg_tables_at`, `arg_indices_for_role_at`, `command_prefixes_at`) are gone, and every spelling is in the retired-api gate with a seeded-violation self-test. **`ProfileSpec::conflicts` was kept**: it is the one axis of the seven with a live consumer — `tcl-bigip`'s `check_virtual_event_profile_graph` reads it to emit `BIGIP6039`, a registered public code with a catalogue entry and a test. Every shipped row is `&[]`, so the diagnostic cannot fire today; deleting the field would have deleted a published diagnostic, which this ruling does not cover. Principle P-C: anything genuinely needed later comes back with its consumer | — | Done |
| O3 | ~~Formal ratification of E-R11, E-R12, E-R13.~~ **RESOLVED (owner, 2026-08-27): all three ratified.** E-R11 (the canonical form and `tcl spec export`) and E-R13 (the `spectcl_expand` verb) were ratified as shipped. E-R12 (never rewriting a programmed pack; form edits become `StudioOverride` patch packs) was ratified **with an amendment**: standing overrides must be *visible* — a studio indicator, and a `spec check` warning once a patch has outlived a threshold, so an override reads as a staging area rather than a home. Contraction stays ruled out | — | Done; §14 absorbs all three. The E-R12 visibility amendment is the work item |
| O4 | ~~The trusted-tier choice for the studio buffer and `spectcl_check`.~~ **RESOLVED (owner, 2026-08-27), differently for each surface.** The **studio authoring buffer stays trusted**: it is the author's own unsaved file in their own editor, `-override` on a compiled name is a first-class studio operation, and the gate still bites at `pack_set` install with an explicit untrusted-refusal report. **`spectcl_check` gains an explicit tier defaulting to the tier the pack would actually install at** (workspace for a workspace pack), so the check predicts reality instead of answering a different question; the trusted view stays available on request. The determinism sandbox runs unconditionally either way (E-R2) | — | Done; the `spectcl_check` tier parameter and its default are the work item |
| O5 | **P7 — the iRules surface as a pack.** Q5 ruled ▸ deferred, and six of the seven prerequisite words have since landed, so the deferral is now a standing choice rather than a blocked one | The iRules command surface stays compiled Rust. The dialect (grammar, structure) and the closed-world policy stay compiled either way, so nothing about correctness rides on it | An owner decision to schedule it, plus the M9 ruling (O2) and the invocation-refinement descriptor (D2) |
| O6 | **The assistance-view ordering hint** (Q8's residual). Should the assistance view surface "used before its `package require`" by default, or leave ordering to the semantic diagnostics? ▸ recommended surfacing it; nothing was built | One diagnostic's default. The scan already records the ranges, so the cost is a message and a severity | An owner answer |
| O7 | **`primary` for a multi-target project** (Q15's residual). What the UI defaults `primary` to when a user declares targets without one, and how visibly. P1b shipped ranges without an explicit `primary` selector: the primary is the environment's, always | Multi-target projects cannot choose which release assistance answers under. Compatibility checking is unaffected — it evaluates the whole set — so this is an assistance-quality gap, not a soundness one | An owner answer plus the settings/directive surface to carry it |
| O8 | **Per-pack trust consent and provenance in hover** (Q19's (b) and (c)). The registration-time tier gate landed; the consent surface and the "which tier won this fact" hover did not | A workspace pack cannot be granted security-fact overrides at all today, which is the safe direction — so this blocks a capability, not correctness | An owner ruling on where consent is recorded, then the config surface and the hover field |

### 11.2 Deferred model items

Each of these is a known, scoped change to the Rust model that a lane
declined to make and named precisely rather than working around.

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| D1 | **Ledger C1 — the executable-IR re-key.** `DialectSet` survives behind `SemanticAnalysisBundle`'s `dialect` field, its `unavailable(DialectSet::empty())` constructors, `tcl-lsp-db`'s `semantic_dialect_set` projection, `build_linear_executable_ir(registry, dialect, …)` and the world-state SSA it keys, plus `bpf-tcl-ir`'s `semantic_bridge`. P1a surveyed it and ruled it a coordinated re-type, not a port | The **largest** remaining item, and the gate on four others: the interned `DialectProfile` (so a pack-declared dialect can actually *lex*, D5), the analyser-vs-unit `tk` asymmetry, the salvage of the Tk specificity row, and the salsa lexer-config truncation (E1) | Re-key the bundle on `ResolvedContext`, which drags the WASM and BPF semantic bridges with it. It is one change or none — a partial re-key leaves two dialect vocabularies |
| D2 | **The invocation-refinement descriptor** (Q12). The declarative, whole-descriptor replacement for `command_forms` / `subcommand_forms`: per-form word patterns, traits, mutator/query split, effects as data | Every surface-to-pack conversion. Tk's 53 `subcommand_forms` sites are the migration test and 53 of the 67 sites in the tree are Tk's, so **Tk cannot round-trip** without it; P3 named it first in what a conversion still needs | Design and land the descriptor type, then convert Tk's 53 sites behind the I10 behavioural-parity harness |
| D3 | **`bpf_op`'s id catalogue.** The seventh ratified word has no loader reader because `CommandSpec::bpf_op` is `Option<&'static BpfOpSpec>` and every shipped value is a private per-command `static OP` under `tcl-registry/src/commands/bpf/`, so `bpf_op -native ID` has nothing to resolve `ID` against | A `bpf` surface pack; and it is the live proof that §6.3's loader-direction gate (E3) does not exist — the word is ratified, unread, and nothing fails | A named `id → &'static BpfOpSpec` table in `tcl_registry::bpf_op`, then the ordinary `-native ID` reader at the shared seam |
| D4 | **Body-scoped completion codes** (E-R6, census G13). `::struct::tree::prune` exits with completion code 5, but *only inside a `walk` body*. A body slot carries a timing and a kind, never the set of codes its command consumes | P5 deliberately gave `prune` **no** control-flow trait, because `CONTINUES_LOOP` would be a lie the CFG builder acts on. So the CFG for a `struct::tree` walk is wrong-but-conservative, and every library-defined completion code has the same ceiling | `body_completion_codes: &[(u8, CompletionCode, &str)]` on `SubCommand`/`OptionSpec`, plus scoping the `BREAKS_LOOP` machinery to that body. E-R6 is the ruling; the field and the consumer are the work |
| D5 | **Pack-declared dialects cannot lex.** P3 converts a validated `dialect` block into a `tcl_dialect::model::DynamicFamily` with a real `LexerGrammar` per release, and an `environment … { core DIALECT RELEASE }` binding resolves it — but `Family` is a closed enum, `grammar()` is a `const fn` over ladder ordinals, and `tcl_lexer::LexerConfig` is built from a `&'static DialectProfile` out of a compiled table. So the grammar is reachable and nothing on the analysis path consumes it | The `dialect` block's whole point: a third-party family that actually parses differently. Today a pack-declared dialect is data with no lexer | D1. This is not more conversion work — it is the `DialectProfile` re-type |
| D6 | **`NumberSyntax::Jim`.** Jim accepts `0x`/`0o`/`0b`/`0d` and leading zeros do **not** imply octal — none of `NumberSyntax`'s three values. P6 ships `Tcl90` as the closest (right that `010` is ten; wrong only in accepting Tcl 9's `_` separators, which Jim rejects) because `Tcl85` would be wrong the dangerous way round | `expr {1_000}` is accepted for a jim document and errors on every real `jimsh`. One over-accepting literal form | A `NumberSyntax::Jim` variant: 231 sites across 43 files plus a lexer arm — a lexer change, not a data edit |
| D7 | **A byte-counting character model.** A non-utf8 Jim build counts bytes, a third rule `StringCharacterModel`'s two Tcl models cannot express. P6 answers `character_model` as `None` (every consumer abstains) and carries the measured fact on `CapabilitySet::utf8_character_model` instead | `string length` folding under `--minimal` jim. The abstention is honest, so nothing is wrong — it is simply unavailable | A `StringCharacterModel::Bytes` variant and its `count_for` agreement rule, which changes constant folding for **every** dialect and therefore needs the differential gate |
| D8 | **The multi-realm `AnalysisWorld`** (Q21's remainder). P1a landed the single realm. Child interpreters, safe interpreters and `hide`/`expose` widen everything they touch to `Unknown` rather than being modelled | Precision only: an `interp create` widens more than it must. Soundness is preserved by the widening | Build the realm map on the landed `BindingKnowledge` and `RealmState` shapes, with the parent/child/safe/ordering e2e suite invariant I3 names |
| D9 | **Ledger C6 and C8 — the transition vocabularies.** The analyser's `command_aliases` / `renamed_commands` / `deleted_commands` tables and `indirection.rs`'s link walk feed the one oracle but are not re-homed onto `state_transition.rs`; `CommandTableEffect` is still a third transition vocabulary beside `CommandBindingTransition` | D8 (the realm map wants one vocabulary), and the "no parallel binding tables" half of ruling R10 | A coordinated registry + SpecTcl vocabulary change, which is why P1a left it |
| D10 | **Registry generations** (review B8, invariant I7). The loader still leaks per load (`Box::leak` in `tcl-spectcl/src/loader.rs`); no `RegistryGeneration` type exists | A Spec Studio session editing a mass-migrated surface leaks hundreds of MB — ~3.1 MB per generation of ~2,400 specs. It is a **P2 prerequisite** for any mass migration, so it also blocks P3's and P5's conversion halves | Move dynamic pack specs into an arena/`Arc<RegistryGeneration>`, return generation-bound handles, key salsa on the generation id — gated by the 1,000-reload allocator test |
| D11 | **Shared `InvocationSpec`** (review B6). Taint sinks, forms, deprecation replacements and effects are copied field by field into `SubCommand` instead of living in one invocation capability model | Honest specs for method-level sinks (ticklecharts' file write, SpiceGenTcl's `runAndRead`) — census gaps G7/G15 | The refactor, behind the four-surface parity rule |
| D12 | **`tcl spec build --emit rust`** (ruling R7) and the pack-level, `dialect`-block-aware renderer it needs — the current renderer is WASM-only and per-command. R7's sibling deliverable, promoting the MCP checker to a `tcl spec check` verb, is D18's | Q1's endgame: shipped cores as SpecTcl sources with the compiled catalogue and a loadable pack as two backends of one description | The renderer, then the build step |
| D13 | **`--restyle`** — **owner ruling 2026-08-27: build it.** `tcl spec upgrade` grows the flag; it is mostly wiring `export_pack`'s existing shorthand logic into the upgrade path | Nothing — U2 already rewrites `dialects` → `available`, so this is a formatting affordance over a landed translation | Scheduled |
| D14 | **Ledger T9 — the `spec-author` skill.** `ai/claude/skills/spec-author/SKILL.md` still instructs authors to declare `speclib <name> 1.1` and describes 1.1 as the newest vocabulary | Every pack a model or a human authors from the skill is two majors stale, and will not carry `available`, `environment`, `dialect`, `provides` or `include` | Refresh the skill for 2.0: the new words, `dialect` blocks, the `available` algebra, and the upgrade workflow |
| D15 | **The tooling payload rows** (ledger T1, T3, T4, T6, T7, F9, F12/T13, B10, B11, T10, T12). Every *ingress* moved onto the seam; the *user-visible* payloads did not — the CLI's `--dialect` possible values, the MCP `dialect_schema` enum, the studio picker, `registry-dump --all-dialects`, `listDialects`, `callback-surfaces` row ids, the hand-written Sublime `_SYNTAX_DIALECT_MAP`, `_registry_data.tcl`, and the hardcoded `tcl8.6` defaults | Environment names becoming the user-facing vocabulary — which is also what ruling R9's KCS "Applies-to" regeneration waits on, and what defect §9.5 (`# tcl-dialect: tk`) needs | Each is a deliberate user-visible surface change: re-key the payload, regenerate the artefact, and accept the diff. They were held because a name change is not a refactor |
| D16 | **The per-distinct-profile reference evaluator** (§5.4, review B10). P1b shipped two token-local detectors (lifecycle windows, numerals) — the only pair §5.4 licenses without the reference. The reference itself was not built | Every other range axis: escapes, `${a{b}c}`, expr comments and operators, `{*}`, the leading-BOM rule, differential constant folding at the endpoints, `package require` satisfiability per target, numerals *inside* compound `expr` bodies, and the W151 fix-its | Build the multi-profile evaluation, then admit each per-pair detector only after the differential corpus/fuzz gate proves it equivalent |
| D17 | **No shipped pack declares an environment** (Q2). The `environment` block, its live registration, its retirement and its detection routing all work — and the eight bundled `specs/*.tclspec` packs use none of it. The six EDA catalogue shells are still compiled-in `DialectProfile` rows | Q2's "fully centralised end-state", and the proof that the compiled-in environment set really can shrink to the core families plus the named few | Move `specs/eda_*.tclspec`'s identity, extensions, signatures and keyed tool pins into `environment` blocks, delete the catalogue rows, and hold the diff against the editor-catalogue drift gates. `tcl spec upgrade`'s U5 already rewrites the one bundled `file_extension … -dialect` row into an `environment … -extend` block, so the pilot exists |
| D18 | **The gap rulings that did not land as code** (companion §4). **R1** — `rust/tcl-registry/src/stub_overlay.rs` still exists as a separate overlay type consulted per consumer instead of stubs ingesting as provenance-tagged `SurfaceDeclaration`s (Q22). **R2** — special variables are still `special_vars.rs`'s compiled Rust table rather than SpecTcl declarations, so Jim's `env` and picol 2's capital-initial globals have no home (Q23); the private dialect-name ingress *is* gone. **R3** — `FILE_SCOPED_ENVS` is still a hardcoded one-row Rust table (`("tclpkg.tcl", &TCLPKG_MANIFEST_ENV)`) rather than a detection-scoped environment whose surface is a pack. **R4** — `render_spectcl`'s `is_dialect_set` still matches `"dialects" \| "safe_on_uninit"` together, conflating a behaviour predicate with availability (ledger T8). **R5** — the hook `ctx` dict still carries only a `dialect` key; no `environment` key was added (Q25). **R7's other half** — `tcl spec check` was never promoted from MCP to a CLI verb; `tcl spec` has `import`, `upgrade` and `export`. **R9** — the KCS "Applies-to" controlled vocabulary is unregenerated. **R10** — the one-*oracle* landed (R-c, P1a) but the one-oracle **gate** is not written, so nothing mechanically stops a consumer building a second existence oracle | Each blocks a different small thing; together they are why the centralisation ledger's completion criterion is unmet. R10 is the load-bearing one: without the gate, the retirements P1-G proved can silently un-prove themselves | Each is independently landable. R10 wants visibility narrowing (`pub(crate)` on the raw lookup layer) plus a call-site sweep with a ledger-entry escape hatch, in the shape `retired-api-gate` already demonstrates |

### 11.3 Evidence gaps

Measurements the model wants and does not have. None of these is a
modelling problem; each needs a machine, a build, or a probe run. Where the
model must answer without them it abstains explicitly — that is the design
working, not a bug.

| # | What it is | What it blocks | What would resolve it |
|---|---|---|---|
| V1 | **The two APL contexts** (`IAppPresentationApl`, `IAppPresentationTclCallback`). Never exercised. Both answer `None` on every `BigIpExecutionContext` accessor, `Unknown` through both evidence doors, and `is_tcl()` is `false` for APL | An APL `tcl` callback has no family, no build profile and no surface, and the model must keep refusing to copy `IAppImplementation`'s row into it. Also blocks F1's typed **embedded-range descriptor** for APL's `tcl` clauses — a document/range concern for the compiler and server, not the registry | E4 step 6: exercise a non-interactive presentation renderer on an appliance |
| V2 | **A second and third BIG-IP build** (one supported 17.x, one older), same suites | Every "since when" question about the F5 tree is `Unknown`: `EmbeddedRuntimeEvidence`'s nearest-known assistance answers cannot become interpolation, and the `f5-tcl` ladder's post-fork deltas stay hypothesis rather than data | Two more appliance runs |
| V3 | **A restricted-role tmsh column.** The whole probe corpus was run as the SSH login user, so the §5 command surface carries no role annotation | F4's role/visibility overlay has **no measured input at all** and cannot be wired honestly. Completion should show policy-disabled commands with a reason; today it cannot know which they are | One appliance run under a restricted role, with `systemauth.disablebash` and the other policy settings captured |
| V4 | **The E4 re-run of the §5–§9 suites.** §3 and §4a are E4-grade; the 85-builtin surface, the 120-cell event matrix and the traffic lab are consumed today as `e4_conforming: false` | ~180 corpus vectors stay "strong transcript" rather than ratified evidence. §11 of the measurements says the re-run is mechanical — `lib/runner.sh` needs a prefix change and a pre-create absence check | Re-run under the E4 contract |
| V5 | **`matches` precedence and semantics.** #27 closed the *grammar* half — the bare `matches` is the trunk's tenth word operator. Two things the transcripts do not pin: **precedence** (`expr {"abc" matches "abc"}` is a single-operator expression and exercises no binding power, so the equality class is inherited from its siblings, not measured) and **semantics** (the same probe is exact equality and discriminates none of the equality/containment/glob readings) | The VM answers it as string equality — the one reading the measured cell exercises — and the compiler deliberately **refuses to constant-fold** it so no unmeasured meaning is baked into a rewrite. That decline is the cost | Two probes: `expr {"abcd" matches "bc"}` and `expr {1 or 0 matches 0}` |
| V6 | **The realm scope of `tmsh::modify cli version active`.** Script-, tmsh-process-, session-, or system-scoped? | Where the F6 transition's state lives. It is wired with `scope_is_measured: false` until answered | One appliance probe |
| V7 | **The runtime half of the six deliberate `RULE_INIT` divergences.** Does calling an `HTTP::*` command in `RULE_INIT` always fail at runtime, or only when it touches connection state? | Nothing in the model — these rows are *meant* to diverge, and §8 of the measurements is explicit that `RULE_INIT` compile-acceptance must not be read as "valid to use". Confirming it would let the divergence reason be sharpened from policy to fact | A per-cell runtime re-probe in the traffic lab |
| V8 | **A jim probe corpus.** P6's five `jimsh` binaries and their transcripts are on disk, not a hermetic in-tree fixture set keyed by `(release, configure flags, platform, commit)` the way review H3 requires and the F5 evidence layer demonstrates | Every jim claim is re-derivable only by rebuilding the interpreters. The F5 corpus caught sixteen real model defects on its first run; jim has no equivalent tripwire | Build the corpus in the F5 shape: typed records, two-sided expectations, hermetic vectors |
| V9 | **Corpus-*generated* F5 rows, the transcript-schema validator, and the prose/rows/tests drift gate** (F8, ruling R11). The 205 vectors *assert* the hand-authored F5 catalogue rows; they do not produce them | Ruling R11's actual contract. Until rows generate from the corpus, a measurement and a registry row can still be edited apart — the corpus catches it at test time rather than making it unrepresentable | The generator, the schema validator (which must never run in CI), and the drift gate |
| V10 | **The F4 role overlay and the F5 `tcl_platform` CMP-effect overlay.** Both are recorded as evidence and neither is wired as an overlay | F4 additionally blocked by V3 (no measured input). F5's CMP overlay has its input — TMM's seven fabricated keys are pinned against `special_vars.rs` — and simply was not wired | For F5: wire the effect refinement. For F4: V3 first |
| V11 | **Single-binary probes on the five-version matrix** (P0's fourth bullet). `find_tclsh`'s first-hit callers were not upgraded when the five interpreters started building | Release-differentiating behaviours probed by a single binary silently answer for whichever tclsh is first on `PATH` | Thread `TCL_LSP_TCLSH{84,85,86,90,91}` through those call sites |
| V12 | **The oracle programme's vector domains** (companion §7.2–§7.5). The reference interpreters, the path fixes and the Tk trees landed (P0-B); the variable/namespace-op vector files, the binding vectors, the package/autoload vectors, the real-corpus index parity and the consumer conformance lattice did not | The lattice is the "every consumer leverages this properly" checklist, and it is not yet a set of passing gates | Land the vector files per §7.2's format and wire the five consumers |

### 11.4 Doc-and-code divergences the P8 sweep found

Recorded here because each is a claim that outran the code, and the code is
what a reader should believe.

| # | The claim | The code | Disposition |
|---|---|---|---|
| E1 | §9.1's salsa lexer-config truncation was expected to be fixed alongside the model move | `LexerCfgKey::to_config` and `lower_proc_body` still close with `..LexerConfig::default()` (`braced_var: Tcl9Nesting`, `escapes: Tcl90`, `leading_bom: Content`) while `LexerConfig::from_grammar` sets all three per dialect | **Still open**, gated on D1. The stale doc comment on `LexerCfgKey` is corrected; the truncation stands. Widening the key changes which documents share a `compilation_unit` build per edit, so it is a behaviour-plus-performance change, not a comment fix |
| E2 | §3.3: "the tcllib-excluded-from-iApps rule **stops being** a subtractive `DialectSet::all().difference(IRULES \| IAPPS)` and becomes 'the `f5-iapps` environment is closed over its ambient set'" | `rust/tcl-registry/src/commands/tcllib/mod.rs:301` still computes exactly that difference | **Still open**, gated on D1 — the subtraction is `DialectSet` plumbing. Relatedly, §5.3's "`package_active` implements exactly these three rules" over-claims: `package_provider_active` distinguishes all three `WorldPolicy` values, but the *availability* query treats `AmbientPlusRequire` identically to `Open`. §5.3 now says so |
| E3 | §6.3: the round-trip gate "gains a loader-side direction … so a ratified word without a loader arm fails CI instead of silently dropping" | No such gate exists. `bpf_op` is ratified, unread, and nothing fails | **Still open** (D3 is its live instance). The two-loader byte-identity gate and the export gates are real and strong, but neither exercises a *documented-but-unimplemented* word |
| E4 | The deep dive's §15.4 status block listed `render_spectcl` 2.0 emission and its pin lift as the one unlanded item | It landed one wave earlier: `DSL_VERSION` is `tcl_spectcl::NEWEST_VOCABULARY_VERSION` (`"2.0"`), `availability_rows` writes `available` / `-available` at all seven scopes, and a 1.x document keeps `dialects` | **Fixed by P8** — the status block now ticks it. This was a wave-2 lane recording a wave-1 landing it did not know about |
| E5 | `LexerConfig::for_dialect`'s doc comment said `expand_syntax` is true for "iApps, tmsh, Expect, EDA flavours" and `irules_brace_separator` is "true only for iRules" | Measurement §4a moved both: `GRAMMAR_F5_TCL` is `expand_syntax: false`, `irules_brace_separator: true`, and all three F5 catalogue rows select it (P1-G) | **Fixed by P8** — comment corrected in place |
| E6 | `grammar.rs` and `expr_lexer.rs` named "plain `tcl`, iRules" as the `None`-expr-base dialects (§9.3) | The `None`-base rows are `f5-bigip`, `PLAIN_TCL` and `TK_PROFILE`; `f5-irules` has always carried `Some(V8_4)` | **Fixed by P8** — both comments corrected |
| E7 | `dialect-detection.md` listed 16 `KNOWN_DIALECTS` names; `dialect-profile-model.md` §8 said "16 catalog entries" | Both are 18, and the detection doc's list omitted `microchip-libero-eda-tcl` and `spectcl` | **Fixed by P8** |
| E8 | §5.1: the directive "accepts environment names and aliases — making `# tcl-dialect: tk` … finally coherent" | `detect_dialect_directive` still gates on `KNOWN_DIALECTS`, which has no `tk` row, so the directive still abstains | **Still open** — it is D15's payload change, not a local fix. §9.5 records it; §5.1 now marks the sentence as intent |
