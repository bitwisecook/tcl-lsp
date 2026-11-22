# Cross-cutting review

A synthesis across every study on this branch — [TclOO and Tk root causes](oo-tk-root-causes.md), [object systems](object-systems-plan.md), [command lookup](command-lookup-centralisation.md), [variable lookup](variable-lookup-centralisation.md), [call frames](frame-design.md), [namespaces](namespace-design.md), [expressions](expression-design.md), [packages](package-design.md), [rename and alias](rename-alias-design.md), [scripts, words and writers](words-scripts-design.md), [lists, dicts and arrays](lists-dicts-arrays-design.md), the [overrides review](overrides-package-review.md), the walker census and the [architecture plan](architecture-tidy-plan.md) — looking for what recurs. Each study found the same few faults in a different domain. Fixing them one domain at a time would repeat the same work ten times; fixing them once, as foundations, removes most of the domain items.

## The recurring pattern

Every study found the same shape:

1. A **shared low-level owner** exists and is mostly right (name geometry, the expression parser, `tcl-cmd-core`, `tcl-value-syntax`, the `tclPkg.c` port, `tcl_lexer::script`).
2. The **engines delegate to it** and are right almost everywhere.
3. **Analysis, the editor and tooling re-derive the same answer** in four to twenty places, each with partial context, its own key type and its own meaning for "don't know".
4. Those copies **disagree with each other**, and some are **wrong** in ways the engines are not.
5. **Conformance data exists but drives only one side**, so nothing catches the disagreement.

| Domain | Owner | Copies in analysis and tooling | Conformance that drives analysis |
| --- | --- | --- | --- |
| Command names | `command_lookup_bases` | ~22 resolvers, frame chosen 4+ times | settlement only, not the LSP |
| Variable names | variable geometry | ~20 resolvers, frame chosen 7+ times | none |
| Frames | `frame_level` (partly) | ~12 frame types, 3 frame-effect models | none |
| Namespaces | `NsId`, naming | 4 models, ~7 import implementations | none (172-row table drives only tclsh) |
| Expressions | tcl-expr parser and `ExprOps` | 6 precedence tables, 7 math-function lists, 5 floor divisions | syntax vectors only |
| Packages | `package_versions.rs` | 3 other version schemes, byte-exact names | none for the others |
| Rename and alias | engines | 5 identity schemes, ~5 rename models | none |
| Scripts and words | `group_commands` | private groupers, ad hoc writers | segmentation differentials |
| Lists, dicts, arrays | `tcl-cmd-core` | 2 evaluator tiers, ~9 index readers, ~9 container models | none |
| Objects | grammar data | 7-family enum matches, 3+ identity stores | none |

## Fundamental faults

### 1. No semantic identity layer

Analysis keys almost everything by readable strings: namespaces (four models), commands (five identity schemes; the analyser's legacy rename maps hold one string per destination), variables (two `PlaceKind` enums, name-keyed diagnostic sets), objects and widgets (three proof stores, a class-name string, a `HashSet<String>` of Tk paths, nothing for images and fonts), packages (byte-compared names, no aliases) and frames (about twelve types). The naming guide's central rule — deleting and recreating a thing can reuse its name while creating a different thing — holds only in command binding and the engines.

**Improve:** one identity layer for analysis — command, namespace, variable place, frame, body unit, instance, package — each a typed, interned key carrying its incarnation or generation and source provenance, with display text derived from it and never parsed back. `tcl-runtime-api` exposes the engines' tokens in the same vocabulary so engine and analysis answers can be compared directly.

### 2. No query layer, so every consumer computes its own answer

The same question — what does this name mean here, which frame is this, what value is this, what does this call do — is answered locally by each diagnostic, editor feature, optimiser pass and tool. That is why hover and go to definition disagree on imports, `next`, `apply` and `uplevel`, why W210 contradicts navigation, why `next` gets three answers, and why there are about 45 walkers beside the pipeline.

**Improve:** the semantic model of the architecture plan, built on the identity layer: one owner per question, one query API, every consumer (W-codes, LSP features, the optimiser, codegen, MCP, the diagram, the explorer) reading it. Consumers that cannot get an answer abstain; they never compute a private one.

### 3. "Unknown" has no common meaning

Answers come back as `Present`/`Absent`/`MayPresent`/`Unknown`, `Option`, `bool`, "overdefined" or a best guess, and editor refusals come back as an empty edit list or `null`. Unknown is repeatedly treated as absent or false, which produces the false positives (W123 on `my`/`self`/`next` in entered methods, the W210 family after `catch`, `regexp`, callee `upvar`, `uplevel 2`, alias-then-`upvar`, `array unset` with a pattern, W307 after `oo::define`), the wrong claims (I230 "always false" from a misparsed `in`), and coarse fallbacks (one `oo::define` resets every object; one relative head refuses a whole method body; one unknown package drops every W120 in a scope).

**Improve:** one answer type for every semantic query — proven value, a bounded set of possibilities, absent, or unknown with a reason and provenance — and one outcome type for every editor operation (done, or refused with a reason the user sees). Diagnostics fire only on proofs. Transitions change exactly what they touch and mark the rest unknown per name, not per world. Track abstentions as a measured cost, separately from wrong answers.

### 4. Release and dialect policy is copied, not owned

The per-release facts are copied by hand into many tables: six precedence tables (one wrong), seven math-function lists, two `lsort`/`lsearch` option tables, four version schemes, nine index readers, escape widths, `concat` trimming, character counting, five partial bundled-package rosters, Itcl pins twice, the `namespace` subcommand table three times, and float spelling and the missing-release default decided separately by four constant-folding bridges ([constants and folding](constant-folding-design.md)). Several copies are release-blind (taint's escape widths, the registry's prerelease-blind version comparator, `.chars().count()` in three components), and some engine copies miss release features (`max`/`min` in 8.6 mode, 8.6's system-encoding source reading).

**Improve:** every release-dependent fact lives once, in `tcl-dialect` or the registry, generated from or checked against the C Tcl and Jim sources and the oracles (operator tables from `tclCompExpr.c`, bundled packages from each tree's `pkgs/package.list.txt`, math functions from the library); every other table is generated or deleted. A lint flags hand-written release tables and `.chars().count()`/`trim()`-style host primitives on Tcl text.

### 5. Verification is per component, not across components

The repository has good conformance data: command-resolution vectors, variable-resolution vectors, a 172-row namespace-operation table, frame-level vectors, the package-version oracle, captured Tcl test results for lists and dicts, segmentation differentials. Almost all of it drives one implementation — usually the engines or the pure owner — and none drives the analysis consumers or the LSP features. That is how `in`/`ni` precedence, the VM's `next` level, `lremove`, the runtime's stale dict representation and the analyser's false I230 all went unnoticed.

**Improve:** one conformance framework: vector tables as rows, implementations as columns (tclsh 8.4–9.1, Jim, the VM, the runtime, the compiler folder, the analysis queries, each LSP feature, the optimiser, the writers), every row run in every applicable column, disagreements as failures. Add property tests for inverses (`parse(render(t)) == t`, `read(write(v)) == v`) and execution-checked editor operations (apply the rename, refactor or optimisation, then run both scripts). Adding a behaviour stays "add a vector", and now covers every consumer.

### 6. The shared kernels don't accept partial knowledge

`tcl-cmd-core`, the number tower and the expression evaluator serve the engines well, but analysis needs to evaluate over partly known values, so it re-implements them (the registry's `const_fold.rs`, SCCP's `llength`, the interval evaluator, hand-rolled dict keys) and keeps itself safe by declining. `ExprOps` shows the fix: one walk, parameterised by the value operations.

**Improve:** extend the kernels with abstract-value interfaces in the `ExprOps` style — commands, index arithmetic and comparisons evaluated over a value domain that includes unknowns and sets — so SCCP, value transfers, the folder, intervals and the optimiser run the same code as the engines.

### 7. The IR loses structure, so everyone re-reads text

Command words are kept as flat text, so nested substitutions are rediscovered by scanners used in 143 files; expressions are re-parsed in about 70 files; analysis re-splits Tcl lists and words with `split_whitespace` and unchecked brace stripping; and Tcl source is written by ad hoc `format!` in a dozen places (SslicTcl can emit unbalanced braces). Reading and writing are not inverses anywhere by test.

**Improve:** structured words and expressions with source maps in the IR, read once; a writer owner that takes a purpose; "never re-derive from text" enforced by gates on the host primitives (`split_whitespace`, `strip_prefix('{')`, `.join(" ")` producing Tcl, `format!` producing Tcl).

### 8. Analysis does not cover all reachable code

Method bodies get no SSA, one-member `oo::define` bodies are not read, code after an unproven call stays unlowered, nested commands miss checks that top-level commands get, callbacks and deferred scripts are rendered as synthetic procedures or skipped, code inside data (lambdas in dictionaries) is invisible, and cross-file facts are partial. Each gap spawned fallback walkers.

**Improve:** every reachable body is a unit with entry facts from a frame value, and the registry declares, for every argument that holds a script, command prefix, lambda or name, how it is evaluated (caller frame, named namespace, global at call time, level-selected, object frame) — including user procedures that pass a parameter to `uplevel 1`.

### 9. The registry is not yet the source of truth for semantics

Knowledge that belongs in data is hard-coded in the compiler: the seven-family `DefinerFamily` matches, `TCLOO_GRAMMAR` references, Tk checks with option tables on constructors only, variable-creating and frame effects written as handler cases, value domains as free-text hints, package aliases absent, evaluation contexts absent, `subst` options read in three places.

**Improve:** registry descriptors for effects (variables created, linked or removed; frames entered; commands defined, renamed, aliased, deleted; instances created and destroyed), evaluation contexts, value grammars (closed sets, letter sets, geometry, indices, coordinates, event patterns), object-system profiles, package identities and aliases, and per-release rosters, authored in SpecTcl where that fits, with the compiler reading only descriptors under the `registry-axes` gate tightened to zero.

### 10. The two engines duplicate and drift

The VM and the runtime have different frame models, namespace models, object representations and alias paths, and copied code (frame selection, ensemble objects, subcommand tables). Bugs land in one or both: `in`/`ni` precedence and `max`/`min` in 8.6 mode in both; renamed-alias deletion in both; `next` levels, `filter` in class scripts, `info frame`, literal `{*}`, silent file-mode errors and `lremove` in the VM; the stale dict representation, `array statistics` and the runner without a standard library in the runtime.

**Improve:** complete engine traits (`Frames` with selection and same-level push, `Namespaces` with import/export/path/unknown/ensembles, command identity), shared implementations behind them, one differential suite (the fixtures from these studies plus the fuzzer) running both engines against every tclsh, and an explicit decision on whether the engines converge or stay separate for stated reasons.

### 11. Failures are silent

The VM exits with status 1 and no message in file mode; editor rename returns an empty list and MCP returns `null`; the server drops every W120 in a scope when one package is unknown; W123 suggests "did you mean '!'". Silent failure hides the faults above and makes users distrust correct refusals.

**Improve:** every failure path carries a reason to the user or the log, and tests assert on reasons as well as outcomes.

### 12. Gates check spellings, not meanings

`segmentation-drift` matches four spellings, the parser census finds functions by name, `resolution-drift` matches `.name ==`, `registry-axes` matches word comparisons. None checks that two implementations agree.

**Improve:** keep the spelling gates as cheap ratchets, and add the semantic gates: the conformance matrix (fault 5), the walker and resolver manifest with an only-shrinks ratchet, the crate-dependency rule, the memo-type rule, and the release-table lint (fault 4).

### 13. Documentation states things the code does not do

Contracts cite `Vm::enter_level` and `frame_depth_stash` (both gone), one contract says a source rename removes an import (tclsh keeps it), `segmentation-drift`'s header counts four implementations that no longer exist, `listKnownPackages` and `ObjectClassSpec` carry stale comments, and `BodyFrame` documents `namespace eval` as a parameter frame.

**Improve:** every behavioural statement in a contract is backed by a vector row or a test that the `owner-resolution`-style gate links to; contract claims without evidence are flagged.

### 14. Measurement is ad hoc

Every study here was measured by appending a temporary probe test, building, and reverting; oracles were assembled by hand (Tk through Python's tkinter, itcl from a cloud SDK, no snit, tcllib or critcl); tclsh 8.4–9.1 and Jim exist but `TCL_LIBRARY` had to be set per release.

**Improve:** a permanent probe harness — fixture directories in, a matrix of tclsh, Jim, the VM, the runtime, the analyser's diagnostics and each LSP feature out — which becomes the conformance framework's runner; and an oracle environment that installs Tk, itcl, tcllib (snit, stooop), critcl and NX for every release, recorded in the environment setup.

### 15. Caching is layered by accident

Salsa memoises whole documents and some bodies; beneath it about 110 caches use five staleness mechanisms, registries and spec packs are treated as static but versioned (held in generation-keyed globals, several leaked), the server analyses one document through several pipelines, and an edit re-analyses the whole file (1.27 s for a 687-line file at opt-level 2, 17 ms for two lines), with about 230 MB retained after ten distinct analyses.

**Improve:** everything versioned becomes a salsa input; memos inside memoised values and process-global memos become queries or per-query scratch; one analysis pipeline; per-body granularity; interned identities in the database; per-layer memory accounting, eviction and gates. Details in [caching](caching-design.md).

### 16. The IR does not carry what the registry knows

The lattices are typed, but command nodes are strings with structure held beside them; per-word roles, places, types, the resolved command and its effects are never projected onto the IR, so about 990 production lines re-ask the registry by name, and passes key their tables by strings (GVN by command spelling, block-name maps, name-keyed escape and class sets, rendered-text equality). Details in [analysis data structures](analysis-data-structures.md).

**Improve:** annotated commands computed once from the registry and packs, typed keys in every pass, and type-level gates.

## Correctness backlog

Wrong answers found by these studies, independent of the refactor; each has a fixture under [`oo-dispatch-probe/`](handover/oo-dispatch-probe/README.md).

| Where | Fault |
| --- | --- |
| VM, runtime | `in`/`ni` bind tighter than `==`/`eq` (wrong results on every release) |
| VM, runtime | no `max`/`min` in 8.6 mode |
| VM, runtime | deleting a renamed alias by its new name succeeds (tclsh refuses) |
| VM, runtime | script files read as UTF-8 in 8.6 mode |
| VM | `next`, `nextto`, chained constructors run one level too deep |
| VM | `filter` rejected in class definition scripts; no `info frame` |
| VM | `{*}` on a literal braced word fails in some compile contexts; file mode exits silently (together: `package require msgcat` kills the VM) |
| VM | no `lremove` (also absent from `tcl-cmd-core`) |
| runtime | `puts $d` prints an empty line after `dict with` changes `d` |
| runtime | no `array statistics`; its script runner has no standard library |
| analyser | I230 "always false" and W154's explanation for `$a == $a in $l` |
| analyser | W233 on float division by zero (Tcl returns ±Inf) |
| analyser | false W210 after `catch` bodies and options variables, `regexp` matches, callee `upvar`, `uplevel 2`, alias-then-`upvar`, `subst`, `eval [list set …]`, `array unset` with a pattern |
| analyser | false W123 on `my`, `self`, `next`, `nextto`, `link`, `callback` in entered methods |
| analyser | false W120 for `package require itcl` |
| analyser | false W214 on fixed-signature callbacks, and on `ResolveCmdName` around `"${ns}::$x"` (with a lost W301) |
| analyser | false W102 on literal `subst` templates; W241 on coroutine generators; spurious W307 after `oo::define` |
| SslicTcl | writes unbalanced Tcl for values ending in a backslash |
| codegen | `concat` fold strips non-breaking spaces Tcl keeps |
| contracts | a source rename is documented as removing an import |
| optimiser | `optimise()` and a `None` dialect silently fold nothing, even `1 + 2` |

## Changes to the plan

The domain items 1–45 in [plan.md](plan.md) and the phases of the [architecture plan](architecture-tidy-plan.md) stand, but they should hang off the foundations above instead of each re-solving them:

- **Phase 0 becomes foundations-first:** the probe harness and conformance matrix (faults 5, 14), the answer and outcome types (fault 3), the identity layer (fault 1), and the correctness backlog fixed with its fixtures in the matrix. These unblock everything else and make every later step measurable.
- **Then the data foundations:** release facts in one place (fault 4) and registry descriptors for effects, evaluation contexts, value grammars and identities (fault 9); kernels with abstract-value interfaces (fault 6).
- **Then the pipeline:** structured words and every-body-a-unit (faults 7, 8), one binding walk, the semantic model (fault 2).
- **Then consolidation:** each domain's items become "move the consumers onto the owner and delete the copies", checked by the matrix; engine traits and convergence (fault 10); crates and caching.
- **Throughout:** silent failures get reasons (fault 11), gates move from spellings to meanings (fault 12), contracts are tied to evidence (fault 13).
