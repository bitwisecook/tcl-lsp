# Step 319 (proposed): every tooling consumer on SpecTcl facts and hooks

Status: proposed 26 November 2026, as plan items 319–330. It extends
[313](313-consumer-hooks.md), whose work list stops at the optimiser,
minifier, formatter and VM, to every consumer in the workspace. It builds on
[305](305-spectcl-3.md) (facts, rungs, `ctx`), [302](302-libraries.md)
(facets), [308](308-positioned-context.md) (the positioned query), the
registry split (items 262–265) and item 90's crate steps. Its gate extends
`registry-axes`
([registry-consumer-contracts.md § The per-axis lint and ledger](../../compiler/registry-consumer-contracts.md#the-per-axis-lint-and-ledger)).
Item 307's deletions and the editor bundles build on it.

## Problem

The registry rule in AGENTS.md names "the compiler, analyser, or LSP". Its
lint scans 14 roots (`ANALYSIS_TIER_ROOTS`, `xtask/src/util.rs:301`). The
ledger (`docs/generated/registry-axes.md`) holds **529 unwaived sites in
102 files** and **101 waived sites** (51 `irreducible`, 50 on other axes).
Most unwaived sites are in `tcl-bigip` (216), `tcl-compiler` (71),
`tcl-syntax` (44), `tcl-sslictcl` (39), `tcl-mcp` (36) and `tcl-codegen`
(33). The lint misses three kinds of debt:

- **Unscanned crates.** It does not scan `tcl-vm`, `runtime/rust`,
  `tcl-explorer`, `tcl-spec-studio`, `bpf-tcl-ir`, `f5-cli`, `f5-xc`,
  `tcl-lsp-server`, `tcl-cmd-core` or the `xtask` generators. An approximate
  scan, using the registry's command and subcommand names, finds:
  - 261 sites in `tcl-vm` (36 files);
  - 47 in `f5-xc`;
  - 38 in `f5-cli`;
  - 37 in `runtime/rust`;
  - 30 in `tcl-spec-studio`.

  These are lower bounds, because the lint's vocabulary also includes
  options and `.tclspec` words.
- **Tables it cannot see.** A `&[&str]` table counts only when the file calls
  `.contains(` on it. `tcl-compiler/src/lowering/body_cache.rs:83-94` is
  read through `.iter().any(…)`, so its 11 words (`proc`, `namespace`,
  `interp`, `oo::`, `itcl`, …) are invisible.
- **Library and dialect names.** There are 49 `is_irules()` calls outside
  `tcl-registry`. Three sites read the `"f5-irules-cmds"` floor
  (`hover/builtin_commands.rs:150`, `completion/argument_values.rs:113`,
  `tcl-irules/src/source_context.rs:502`). `tk_checks.rs:174` hard-codes
  `TK_PACKAGE = "Tk"`. Items 301 and 304 each propose a separate
  only-shrinking count for these.

## The consumer interface

The consumers below migrate once, onto one interface (coordinator decision
D-C), not onto `tcl-db` queries now and item 85's semantic model later:

- **`DocumentFacts`** in `tcl-model`: the binding-free record of a
  document's analysis, which `AnalysisResult` projects into.
- **`tcl-semantic`** (item 85), between `tcl-unit` and `tcl-diagnostics`,
  implements it as the single analysis query API. Diagnostics become rules
  over it.
- **`DocumentInput { text, environment, facts: FactView, context:
  &ContextIntervals, document: &dyn DocumentFacts }`** is what every feature
  crate takes ([289](../../workspace/289-lsp-core.md#domain-crates-plug-in)):
  the facet at a position is `context` over `facts`, which is the
  selection 308 computes in `tcl-binding`.
- **Who builds it:** the server from `tcl-lsp-session` (its `PackState` sets
  the `UserPack` inputs); the CLI and MCP from `tcl-db`'s session-free batch
  constructor ([293](../../workspace/293-mcp.md#analysis-without-a-session)).
  No consumer below `tcl-db` names it.

So the Now 8 waves target `tcl-semantic`'s API. Item 85 deepens what is
behind it (more facts, the agreement harness) without moving a consumer
again. When 308 below says "`positioned_context`", a consumer reads it as
`input.context` (or `BatchDocument::context`). Only `tcl-db` and the
session call the query.

Tools that run code select no surfaces. `tcl-irule-test`, `bpf-tcl-ir` and
the packaging verbs take their facts as a `FactView` from the entry point
that calls them (workspace rules 4 and 8).

## The consumers

Each consumer gives today's hard-coded knowledge (**Today**), what replaces
it (**With**), the migration steps (**Steps**), what must land first
(**After**) and a **Done when** line.

**1. Analyser and diagnostics** (`tcl-compiler` → `tcl-diagnostics`, `tcl-irules`)
- **Today.**
  - 71 pinned sites in 29 files and 44 waived. The largest are
    `lowering/mod.rs` and `taint.rs` (9 each), `diagnostics/dataflow.rs` (6)
    and `bounds_checks.rs` (4).
  - 32 `AnalyserHookId`s (`hooks.rs:374`).
  - Library passes: `tk_checks.rs` (1,737 lines) and `irules_checks/` (3,085
    lines, driven by `EventRegistry`, 3 pinned).
  - `signature_scan/ctx.rs:676` lists `method`, `classmethod` and
    `itcl::class`.
- **With.**
  - 305's rung table: 21 hook IDs become rung 1 descriptors and 11 stay
    rung 3.
  - `clause_plan`, `option_effects` and `state_transitions`.
  - `Traits` for the body-cache words, and `definition_body` rows for the
    factory heads.
  - 308 for "is Tk present here".
- **Steps.**
  1. Retire the pins file by file, through the axis each waiver names.
  2. Register `tk_checks` and `irules_checks` as rung 5 contributions of
     their domain crate. Item 285 does this for iRules; Tk needs the same
     `UnitRule`.
  3. Move the version gate onto 308.
- **After.** 263, 264, 285, 257, 305 and 308.
- **Done when.** `tcl-diagnostics` has no pin, and Tk's pass is registered,
  not gated by name.

**2. LSP features** (`tcl-lsp-core` → `tcl-ide-*`, item 289): hover, completion, signature help, semantic tokens, code actions, folding, symbols
- **Today.**
  - 21 pinned sites in 11 files, and 24 waived.
  - Versions come from three places: `DocumentFloor` (8 files), the iRules
    floor, and whole-file `package_requires`.
  - `"--"` is spelled 32 times in 17 consumer files, because
    `EndsOptions` is set only on `switch` and `regexp`.
  - `semantic_tokens/object_dispatch.rs` hard-codes `destroy` as the only
    instance method.
  - `snippets.rs:334` holds 16 templates, and
    `editors/vscode/snippets/tcl.code-snippets` copies the same 16 by hand.
- **With.**
  - `DocumentInput`'s `context` and `facts` (308's intervals, `VersionLine`
    and facets, 302) for hover, completion and signature help, so they select
    the same facet the diagnostics did.
  - A `--` option row with `-ends-options`.
  - `subcommand … -exported instance|class`.
  - A `snippet PREFIX LABEL BODY` row.
  - Semantic tokens, folding, symbols and code actions already read
    `ResolvedInvocation`.
- **Steps.**
  1. 308's work items 3 and 4.
  2. The new rows (320). Then delete the `"--"` sites and three waivers.
  3. Generate snippets (328).
  4. Each `tcl-ide-*` crate is born at pin zero.
- **After.** 289, 302, 305, 308 and 320.
- **Done when.** No `tcl-ide-*` crate has a pin, and
  `rg 'DocumentFloor|placement_floor\("f5-irules-cmds"\)' rust/tcl-ide-*` is
  empty.

**3–5. Optimiser, minifier and formatter.** 313 covers these. This step
adds three things:
- **Rewrite templates.** The optimiser's own rewrites are hand-written
  `format!` templates: `pattern_recognition.rs:203,335` (`lassign`, `incr`),
  `tail_call.rs:180-381` (`set`, `lassign`, `tailcall`) and
  `expr_simplify.rs:274`. They become bundled `rewrite` rows with proof
  sets.
- **Folds.** `propagation/command_substitution_folds.rs` (2 pinned: `list`,
  `lindex`) becomes folds.
- **`--` handling.** The minifier and formatter stop reading `"--"` by
  spelling.

**After** 313, 284 and 289. **Done when** 313's done-when holds, and
`tcl-optimiser` and `tcl-ide-format` have no pins and no command name in a
`format!` template.

**6. Codegen and lowering** (`tcl-codegen`, `tcl-lowering`)
- **Today.**
  - 33 pinned sites (`cmd_subst.rs` 19, `emitter/bytecoded.rs` 13,
    `inlining/mod.rs` 1).
  - Waived: 8 in `tcl-codegen` and 9 in `lowering/structured.rs`.
  - Closed hook sets: `LoweringHookId` (26), `CodegenHookId` (21),
    `InlineCodegenHookId` (32) and `ReturnTypeHookId` (5).
- **With.**
  - The `return`, `switch` and `dict` option and subcommand rows, read
    through `option_effects` and the selected subcommand.
  - `try`'s clause plan.
  - `semantic_operation` for the inline sites.
  - The hook IDs stay rung 3, per 313.
- **Steps.** Retire each waiver at its named expiry. The `cmd_subst.rs` pins
  are subcommand reads that the `InlineCodegenHookId` arms already imply.
- **After.** 254, 263 and 292.
- **Done when.** Neither crate has a pin.

**7. VM and `runtime/rust`**
- **Today.**
  - The VM's name tables are its implementation: `DICT_SUBS`
    (`cmd_dict.rs:239`) and the tables in `cmd_info.rs:173`,
    `cmd_file.rs:199`, `cmd_array.rs:107`, `cmd_binary.rs:170`,
    `cmd_prefix.rs:126`, `encoding_command.rs:37` and
    `cmd_namespace.rs:84`.
  - `runtime/rust/src/interp/native_index_lookup.rs:306,408` repeats the
    VM's tables word for word.
  - `command-backing` checks commands only. The subcommand sets are a third
    copy, and nothing checks them.
- **With.**
  - 313's (c) and (b) for libraries.
  - 117a's native model for the tables the two engines share.
  - An **agreement** check: `backing_report` lists each ensemble's
    subcommands per release, and `command-backing` compares them with the
    registry's facets.
- **After.** 117a, 291, 312 and 313.
- **Done when.** `command-backing --check` fails on any subcommand that one
  of the three sources lacks.

**8. Explorer** (`tcl-explorer`)
- **Today.** No command knowledge (about 11 hits, all its own JSON keys). It
  cannot show which fact or hook answered, at which rung, or what
  abstained.
- **With.** A registry view per site. It shows:
  - the selected facet, roles, effects and option effects, each with its
    source (native, image section or user pack) and rung;
  - hook answers and abstentions, with reasons;
  - 308's context;
  - rewrite proof sets.
- **After.** 294, 305, 308 and 313.
- **Done when.** Every conformance example in 313 and here shows its
  provenance.

**9. CLI and MCP** (`tcl-cli`, `tcl-mcp`)
- **Today.** 17 and 36 pinned sites.
  - `command_info` is **hard-wired to iRules**
    (`tcl-mcp/src/tools/editor_queries.rs:59-61`), so it cannot find `lsort`.
  - `irule_gen.rs:50-74` copies `EventRegistry`'s implied profiles, which
    `event_info` already returns.
  - `irule_test.rs:40-60` lists actions and taint sources by hand.
  - `tcl-mcp/src/spectcl.rs:103` `CTX_KEYS` copies the hook `ctx`
    vocabulary, with different spellings (`nwords`, `tcl-version`).
  - `tcl-cli/src/commands/spec.rs:581` `VERIFY_DIALECTS` copies the
    catalogue.
- **With.**
  - `command_info {dialect?, file?, position?}` through 308's context, read
    from `tcl-db`'s batch constructor; MCP and the CLI name no salsa.
  - Dialect selection and configuration from `tcl-session` (decision D-M),
    shared with the server.
  - `EventRegistry` for the profile implications.
  - Effects and taint facts for the action and source lists.
  - `tcl-spec-hooks`' input schema for `CTX_KEYS`.
  - `tcl-catalogue` for the dialect list.
  - The lint, minify and format verbs follow their library crates.
- **After.** 264, 293 and 308. The `ctx` key set also needs 305.
- **Done when.** Neither crate has a pin, and `command_info` answers for
  every shipped surface.

**10. Spec Studio** (`tcl-spec-studio`)
- **Today.**
  - It has its own `render_spectcl` (4,829 lines, with a `GAPS` table) and
    `render_rs` (2,477 lines).
  - `TCL_LADDER` (`render_spectcl/tcl_words.rs:195`) copies the release
    list.
  - The form schema (`schema.rs:444`) is written by hand.
  - It depends on `tcl-compiler` and `tcl-lsp-core`.
- **With.**
  - 310's one printer.
  - A form schema generated from reader3.
  - Releases from 301's catalogue.
  - `render_rs` only for `tcl-dialect-tcl`.
- **After.** 301, 305, 307 and 310.
- **Done when.** Studio has no `render_spectcl` and no release literals.

**11. Editor extensions** (`editors/*`, generated by `xtask`)
- **Today.**
  - The generators read `build_default()` and keep only ambient commands
    with no `required_package`.
  - `gen_tmlanguage_keywords.rs:154-186` adds hand lists:
    `CONTROL_STYLE` (18 words), `CONTEXT_SENSITIVE_EXCLUDE` (6) and
    `STATIC_OTHER_WORDS` (1).
  - `gen_zed_queries.rs:95` has `GRAMMAR_NODE_WORDS` (14). These belong to
    the grammar and are irreducible.
  - `util.rs:338` filters names by spelling.
  - No Tk, tcllib or EDA command reaches a grammar. Zed's
    `tcl/highlights.scm` lists 208 words.
- **With.** See *Editor artefacts* below.
- **After.** 264, 303, 306, 307 and item 119's manifest.

**12. `tcl-irule-test` and `bpf-tcl-ir`**
- **Facts from the caller.** Both are libraries. They take a `FactView`
  from the tool that calls them (`f5-explain`'s callers, MCP, `tcl-cli`,
  `bpf-tcl`), not `tcl-catalogue`, so an iRules fact edit rebuilds the
  entry points only.
- **Today.**
  - iRules getter semantics live in four places:
    - `command_mocks.tcl` (1,695 lines, 97 procs);
    - `tcl-cmd-core/src/irules.rs:86`, which matches on names;
    - `f5-cli/src/commands/explain_flow.rs:404` (7 `HTTP::*` getters mapped
      to connection fields);
    - `tcl-mcp`'s `TAINTED_REFS`.
  - bpf residue: `lower.rs:830` (`is_thread_command`, which item 263
    covers), `frontend.rs:268` (`when`'s `priority`) and the map kinds in
    `ir.rs:346`.
- **With.**
  - 313's `implementation -native ID`, keyed by `semantic_operation`, so
    `tcl-cmd-core` exports kernels by ID.
  - A `reads connection FIELD` effect detail for getters.
  - Mocks keyed by the generated mock name.
  - For bpf: `when`'s clause plan and closed value sets.
- **After.** 263, 304 and 313.
- **Done when.** No crate matches an iRules getter by name.

**13. BIG-IP tools** (`tcl-bigip`, `f5-cli`, `f5-xc`)
- **Today.**
  - `tcl-bigip` has 216 pins. Most are BIG-IP **schema** words (`ltm`,
    `pool`, `virtual`) owned by item 118.
  - `tcl-irules/src/lib.rs` (26 pinned) special-cases `class`, `persist` and
    `pool member` (`:161`, `:310`, `:336`).
  - `f5-xc/src/mapping.rs:40` is a 51-arm per-command translation table,
    with event tables at `:85-139`.
  - `match_criteria.rs` compares `HTTP::*` names 18 times.
- **With.**
  - `-object-kind KIND` on name roles.
  - `semantic_operation` for the getters.
  - The XC map is open question 1.
- **After.** 118, 264 and 304.
- **Done when.** `tcl-bigip`'s pins are reclassified under a schema owner,
  and `tcl-irules`, `f5-cli` and `f5-xc` have no pins.

**14. Packaging** (`tcl docker`, `tcl pkg`)
- **Today.**
  - `tcl docker create` lists the extensions an image needs by loading the
    project's 2.x packs and reading `command.spec.runtime_backing` and
    `command.spec.required_package` directly
    (`tcl-cli/src/commands/docker.rs:58-84`, `tcl_spectcl::pack::load`).
    SpecTcl 3.0 has no `required_package` (`available P 0-` replaces it), and
    406 makes the registry read images only, so this breaks or silently
    lists the wrong extensions.
  - `tcl pkg discover` runs the analyser and the optimiser itself
    (`pkg_discover.rs:31-33, 214, 258`) over `tcl_cli_support::registry_for_dialect`.
  - The `spec` directive's requested tier and the lockfile's graph set a
    package's pack trust (`tier::clamp_requested`), but nothing tells the
    analysis that `tclpkg.lock` changed.
- **With.**
  - `runtime_backing` read through a `FactView` over the project's packs,
    and each library's package names from its image, not the 2.x `spec`
    record.
  - `discover` reads `DocumentFacts` from `tcl-db`'s batch constructor.
  - The lockfile as a `tcl-db` input (integration § 5), set by `PackState` in
    the server and passed to the batch constructor by the CLI, so a lockfile
    edit re-tiers the packs.
- **After.** 305, 307, 406 and 484, and the batch constructor.
- **Done when.** `rg 'tcl_spectcl::pack|required_package|Analyser::new'
  rust/tcl-cli/src/commands/{docker,pkg_discover}.rs` is empty, and a
  3.0 pack's host-native command lists its package in `tcl docker create`.

## Consumer purity: the gate

`cargo xtask registry-axes` becomes the **consumer purity** ratchet: one
gate, one ledger.

1. **Scope from roles.** It scans every crate in 314's
   `crate-layers.tsv`, plus `runtime/rust` and `xtask/src/gen_*`.

   | Role (314) | Treatment |
   |---|---|
   | `surface` (core, dialects, libraries, catalogue) | owner: not scanned |
   | `vocabulary` crates with a shared-owner row (`tcl-syntax`, `tcl-expr`, `tcl-numeric`) | owner of that axis; other words scanned |
   | `domain` (`tcl-irules`, `tcl-bigip`, `tcl-sslictcl`) | may use only the words of the surfaces in a new `owns` column |
   | engines (`tcl-vm`, `runtime/rust`, `tcl-cmd-core`) | **agreement**: their tables must equal the registry's |
   | every other crate | consumer: every site is debt |

2. **Axes.** Two new axes, `library` (library names, `Family::X`,
   `is_irules()`) and `dialect` (environment and release ids), absorb the
   counts 301 and 304 propose. An entry point may name the environment it
   selects.
3. **Tables.** Each of these is one site, however it is read, weighted by
   its entry count:
   - any `const`/`static` array, `phf` map or `HashMap::from` literal with
     two or more vocabulary words;
   - any `match` with three or more vocabulary arms.
4. **Ledger as data.** `docs/work/name-handling/ownership/consumer-purity.tsv`
   has the columns `crate`, `file`, `axis`, `sites` and `retired-by`. It is
   seeded from `RATCHET` (102 rows) and `CLEAN_FILES` (22). The report
   becomes `docs/generated/consumer-purity.md`.
5. **Only shrinks.** `--check` fails on:
   - a count above its pin;
   - a stale pin;
   - a new row;
   - a crate extracted by item 90 or 289 that starts above zero;
   - an `irreducible … never` waiver whose reason names a fact the registry
     could carry (a reviewed word list: `--`, `exported`, …).
6. **Vocabulary from the catalogue.** The vocabulary is read from
   `tcl-catalogue` and every image (307). Each word is tagged with its owning
   surfaces, which makes `owns` checkable.

A keyword list under `editors/` must sit in a region that item 119's
generator manifest names.

## Order

| Wave | Items | Value / shared infrastructure |
|---|---|---|
| 1 | 319 (gate), 325 (MCP/CLI duplicates) | stops new debt; `command_info` is user-visible; needs nothing |
| 2 | 320 (fact gaps: `--`, `-exported`, `snippet`, `presentation`) | one registry change unblocks the LSP, minifier, formatter and editors |
| 3 | 321 (LSP on 308), the CLI half of 325 | share `positioned_context` |
| 4 | 313's optimiser, minifier and formatter; 322, 323 | share `ResolvedInvocation`; follow 257, 254, 284 and 292 |
| 5 | 324 (engine agreement), 326 (iRules semantics) | share `implementation` and `semantic_operation` IDs |
| 6 | 328 (editors), 329 (Studio, explorer) | share the images and `tcl-catalogue` |
| 7 | 327 (BIG-IP tools), 330 (equivalence packs), packaging (consumer 14) | after 118, 304 and each surface's 307 move |

## Editor artefacts from the images

The generators read the installed catalogue: `from_surfaces(SHIPPED)` plus
every bundled image, the EDA packs included. They project one keyword set
per `(environment, library)`. `CONTROL_STYLE` and
`CONTEXT_SENSITIVE_EXCLUDE` become a `presentation -class
control|keyword|builtin ?-context-sensitive?` row, so a library classifies
its own words.

A static grammar cannot see positions. Each library's words therefore go in
their own scope: `support.function.library.<lib>.tcl` in TextMate, and a
capture per library in Zed. An editor setting turns each library on, by
default only for ambient libraries. With the server running, semantic
tokens carry a `library` modifier from 308, which gives the precise answer.

Snippets, the diagnostic catalogue (`package.json`, JetBrains'
`DiagnosticCatalog.kt`) and the AI prompts come from the same input. One
`make codegen` after an image change refreshes every editor.

## Equivalence packs and duplicated catalogues

The equivalence packs are 98,349 lines in `rust/tcl-spectcl/equivalence/`.

1. **They first serve as a consumer-equivalence corpus.** For each surface
   307 moves, the hover, completion, signature-help, semantic-token,
   format, minify and diagnostic outputs over 305's corpus must be equal
   under the Rust and the image surface.
2. **`equivalent_f5_irules.tclspec` becomes source.** `tcl spec upgrade`
   converts it once, and it becomes `tcl-lib-irules`' authored source.
3. **`equivalent_tcl8_6.tclspec` is deleted** for CI's temporary export, as
   307 says.

| Duplicated catalogue | Copies | Fate |
|---|---|---|
| snippets | `snippets.rs`, `tcl.code-snippets` (16 each) | `snippet` rows; the VS Code file is generated |
| event → profile | `irule_gen.rs:50-74`, `EventRegistry` | delete the copy (325) |
| getter values | mocks, `tcl-cmd-core::irules`, `explain_flow.rs`, `TAINTED_REFS` | `semantic_operation` (326) |
| action classes | `irule_test.rs:40-42`, effects and taint | read the facts (325) |
| hook `ctx` keys | `spectcl.rs:103`, `tcl-spec-hooks` | the hook schema (325) |
| release ladders | `TCL_LADDER`, `VERIFY_DIALECTS` | the catalogue (325, 329) |
| subcommand sets | `tcl-vm`, `runtime/rust`, the registry | agreement (324) |
| `--` handling | 32 spelled sites, `EndsOptions` on 2 specs | `--` rows (320) |
| keyword classes | `CONTROL_STYLE`, `Traits` | `presentation` (328) |
| diagnostic codes | `DiagCode` (249), editor catalogues | stay generated; see open question 2 |
| XC mapping | `f5-xc/src/mapping.rs` | open question 1 (327) |

## Verification

- **Done when** the ledger's consumer rows are empty, except reviewed
  `irreducible` waivers and the engine agreement rows, and every consumer's
  done-when above holds.
- **Measurements:**
  - the gate's runtime (under 10 s);
  - the site count per wave;
  - the generators' time with images;
  - the grammar size per library.

## Risks

- **Over-matching.** Schema words and English words collide with the
  vocabulary. Mitigation: per-surface tags and `owns`.
- **Over-highlighting** in static grammars. Mitigation: a per-library scope
  and setting.
- **The ratchet stalls on engine tables.** Mitigation: engines are checked
  for agreement, not absence.

## Corrections

- **313:**
  - Its work list stops at four consumers.
  - It misses the optimiser's hand-written rewrite templates.
  - Its minifier size (7,888) matches neither 5,713 production lines nor
    9,470 with tests.
  - Its "minifier-safe" rule needs the `--` rows.
- **308:** MCP `command_info` takes no dialect parameter. It is fixed to
  iRules (`editor_queries.rs:59-61`).
- **registry-consumer-contracts.md:**
  - "the same ten roots" is 14.
  - "every waiver is `irreducible`" is wrong: 50 of 101 are not.
  - Three `never` waivers in `semantic_tokens/object_dispatch.rs`
    (`destroy`, `--`) are fact gaps.
- **AGENTS.md:** the rule should name every consumer crate.
- **301 and 304:** their counts fold into the `dialect` and `library` axes.

## Open questions

1. **Consumer-specific facts** (`translate xc …`). Are they rows in the
   iRules library, or a waived table keyed by `CommandId`? This is settled
   by whether a second translator appears.
2. **Pack diagnostic codes.** `DiagCode` is closed (249 codes), while
   305's `check` emits new ones (W160). Do libraries get a code namespace?
3. **Library words in static grammars.** Are they on for ambient libraries
   only? This is settled by a VS Code grammar timing test with tcllib's
   roughly 460 command names.
4. **`xtask` generators in the gate's scope.** The proposal includes
   `gen_*`.

## Review fixes (26 November 2026, deep review)

- **W6, C1:** new section *The consumer interface*. The Now 8 consumers target
  `tcl-semantic`'s API (`DocumentFacts`, `DocumentInput`, `FactView`,
  `ContextIntervals`; decision D-C), so they migrate once, and item 85
  deepens the implementation behind it. Consumers 2 and 9 read 308's context
  through the input, and the CLI and MCP through `tcl-db`'s batch
  constructor.
- **C6:** a fourteenth consumer, packaging (`tcl docker`, `tcl pkg`): the
  docker extension list reads `runtime_backing` and package names through a
  `FactView`, `discover` reads `DocumentFacts`, and the tclpkg lockfile is a
  `tcl-db` input. Wave 7.
- **C7:** `tcl-irule-test` and `bpf-tcl-ir` take their facts from their
  caller.
