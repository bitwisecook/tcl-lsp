# Oracle result records

Each run has its own directory containing input identity, provenance,
stdout/stderr, exit status and decoded case outcomes. Use names identifying
oracle, exact version/revision and context. Never reuse `latest` as a durable
identity or overwrite older runs.

A result document records:

| Field | Required content |
| --- | --- |
| Oracle | C Tcl/Jim/F5 and exact version/revision/build |
| Context | Global interpreter, proc/namespace frame, TMM event, CLI script, iApp, or other exact context |
| Input | Stable case IDs, script paths, SHA-256 and invocation |
| Upstream suite, when used | Suite revision, file/test IDs, exact definitions and support files, harness version/dependencies, constraints and pass/fail/skip counts with reasons |
| Raw channels | Relative stdout/stderr/log paths, process status |
| Per case | Setup/acceptance status, completion code, byte-valued result and available error options |
| Interpretation | Question answered and release/context scope; disagreement/unsupported/unavailable stated explicitly |
| Conformance mapping | Vector/fixture and consumer tests using the observation, or “not promoted” |

`local-tcl8.6.16` is an exploratory observation from an installed system
interpreter. It is outside the pinned 8.6.18 reference and does not establish
a completed C release gate or Jim/F5 agreement.

[Compiler101](compiler-production101-20261014/README.md) preserves the unexecuted linker failure; [Compiler102](compiler-production102-20261014/interruption.json) preserves the interrupted original equality assertion output. The [retained92 phase probe](compiler-production92-phase-20261014/README.md) diagnoses #2286 without semantic acceptance.

[Compiler103](compiler-production103-20261014/README.md) retains the original source producer observations. [VM104](vm-production104-20261014/README.md) retains the preflight failure and distinct actual Jim version outputs.

[VM107](vm-production107-20261014/README.md) records31/35 current naming controls with zero captured drift. [Runtime110](runtime-production110-20261014/README.md) records102/104 with original registration failures repaired and two remaining controls.

[Compiler114](compiler-production114-20261014/README.md) records208/265 ordinary naming controls with source-subset drift0 and lossless large-log retention.

[Core117](core-production117-20261014/README.md) records71/101 ordinary LSP naming controls with all92 prior planned selectors retained and source-subset drift0.

[Compiler119](compiler-production119-20261014/README.md) retains ten failed bounded original owner diagnostics, source-subset drift0 and lossless equality output; temporary instrumentation is removed before ordinary replay.

[Compiler120](compiler-production120-20261014/README.md) records85/109 focused ordinary consumer controls with expected-source and captured-source drift0.

[VM121](vm-production121-20261014/README.md) records44/45 current controls with full captured drift0; the original procedure parse-frame spelling remains failing.

[Runtime126](runtime-production126-20261014/README.md) records106/107 exact controls with full captured drift0; original import passes and child-alias allocator ingress remains failing.

[Historical196 debugger captures](historical196-gdb-original2286-20261014/README.md) retain the same original #2286 selector passing once and timing out on repeat, with bounded CPU/SSA stack observations; no current-source, performance or C oracle acceptance.

[Original Jim lambda context SDK controls](original-jim-lambda-context-sdk-20261014/README.md) retain original object getter and command lookup observations used by the VM context integration.

[TEMP207 SSA diagnostic preflight](temp207-ssa-preflight-20261014/README.md) records the failed test compile, zero executions and exact source restoration.

[TEMP208 SSA diagnostic preflight](temp208-ssa-preflight-20261014/README.md) retains the callback-target compile failure and exact restoration; no selector ran.

[Original dying-namespace descendant SDK controls](original-dying-descendant-sdk-20261014/README.md) retain physical parent ancestry, visibility and teardown observations from C Tcl8.4–9.1.

[Original descendant command-delete SDK controls](original-dying-descendant-command-sdk-20261014/README.md) retain eight actual unset/command-delete callback windows per C5 provider, with original ancestor identity and final cleanup; implementation acceptance remains independent.

[TEMP209 SSA diagnostic preflight](temp209-ssa-preflight-20261014/README.md) retains the failed combined build and actual artifact scope; temporary SSA is excluded from production checkpoints.

[TEMP210 SSA diagnostic preflight](temp210-ssa-preflight-20261014/README.md) retains the failed combined build and actual artifact scope; temporary SSA is excluded from production checkpoints.

[Original #2286 SSA equation diagnostic](temp210-original2286-equations-20261015/README.md) retains the actual unique-definition oscillation graph, unchanged180-second timeout, partial-build scope and exact production restoration.

[Original Jim first-substitution SDK capture](original-jim-first-substitution-sdk-20261015/README.md) retains ten fixed genuine-object cases distinguishing failed substitutions from reached command lookup and namespace byte materialisation; no Runtime/VM acceptance is inferred.

[Original Jim remote HEAD recheck](original-jim-remote-head-recheck-20261015/README.md) retains the actual00:31:51–52UTC remote reference observation independently of SDK builds and implementation acceptance.

[Original Jim apply frame argv SDK capture](original-jim-apply-frame-argv-sdk-20261015/README.md) retains seven genuine-object controls for lazy namespace/argument getters and actual frame argv member identity; no Runtime/VM acceptance is inferred.

[Original direct Jim canonical getter capture](original-jim-canonical-getter-order-sdk-20261015/README.md) retains three direct API cases/25rows. The independent [original public/helper which capture](original-jim-which-helper-getter-sdk-20261015/README.md) retains sixteen cold/hot invocations/97rows; neither scope accepts Runtime implementation or a general public-command getter optimisation.

[Original Jim tailcall SDK capture](original-jim-tailcall-selection-sdk-20261015/README.md) retains four genuine-object invocations/19rows, cold/hot getter state and actual native worker/delete callback order across issuer cleanup; no Runtime/VM or full lifecycle acceptance is inferred.

[Original Jim tailcall cancellation SDK capture](original-jim-tailcall-cancellation-sdk-20261015/README.md) retains two genuine-object cases/16rows, actual pending List and payload pointer observations during native deletion, and independent timed upstream provenance. Phase-specific selected-role observations do not establish Runtime public defer, argument final-free or full lifecycle acceptance.

[Original Jim namespace Code SDK capture](original-jim-namespace-code-sdk-20261015/README.md) retains sixteen cases/217rows with32 actual public/direct cold/hot getter and original argument/List identity phases, actual helper replacement/deletion and timed upstream provenance. Excluded failed trials remain exact assets, without Runtime/VM or full lifecycle acceptance.

[Original Jim namespace Origin SDK capture](original-jim-namespace-origin-sdk-20261015/README.md) retains22 cases/293rows with44 actual public/direct cold/hot getter, original header and alias/chain observations, helper replacement/deletion and timed upstream provenance. These selected observations accept no Runtime/VM or full lifecycle behavior.

[Original decoded clause script values](../decoded-clause-script-values-20261015/README.md) retain exact C5/pinned Jim if/elseif/else command-value observations before the related tests; general indentation, F5 and Rust edit acceptance remain separate.

[Original method command-source candidates](../method-command-source-candidates-20261015/README.md) retain relative method-namespace counterexamples and separate original lookup-operand effects, without supplying a method frame, namespace donor or Native lookup proof.

[Original completion argument values](../completion-argument-values-20261015/README.md) retain midcommand hash argv and complete quoted TclOO selector observations, including baseline OO unavailability; these are original fixture evidence, not a Rust/LSP acceptance result.

- [Original import export-filter/self-import capture](original-import-export-policy-20261015/README.md): actual C5/Jim no-export target and self-import outcomes; source advice and Runtime acceptance remain separate.

[Original Jim namespace Inscope SDK capture](original-jim-namespace-inscope-sdk-20261015/README.md) retains14cases/421rows across28 public/direct cold/hot phases: genuine selected tailcall, original List child/extra argv identity and headers, worker Return, arity/replacement/deletion, excluded trial and timed upstream receipt. No Runtime/lifecycle acceptance.

[Original method pre-entry lookup](../method-pre-entry-lookup-20261015/README.md) retains actual C8.6–9.1 helper replacement and unavailable TclOO providers, independently of argument effects; possible source coordinates supply no closed binding or installed method frame.

[Original data-group record values](../datagroup-record-values-20261015/README.md) preserve C5/pinned Jim static literal data, decoded escapes and dynamic values before related controls, without F5, Rust edit or equivalence acceptance.

[Original body-reference scope](../patterns/w214-body-reference-scope/README.md) retains original six-provider BOM/body and expression observations separately from source MAY advice and Rust acceptance.

[Original Jim inventory staging](original-jim-inventory-stage-sdk-20261015/README.md) retains105 rows/nine cases/eighteen phases, original pattern/namespace getter frontiers and the exact excluded trial. Its missing original failed-process exit remains unknown; no Runtime, full SDK roster or final-free/lifecycle acceptance follows.

[Original Jim namespace-info helper replacement](original-jim-info-helper-replacement-sdk-20261015/README.md) retains19 exact rows across actual default/replaced/deleted helper phases with original headers sampled before getters. No Runtime/full-info/lifecycle acceptance follows.

[Original package requirement values](../patterns/package-requirement-word-values/README.md) retain C5 static data/error observations and Jim's unsupported subcommand, without loader or Native acceptance.

[Original discovery list data](../patterns/tcllibpath-list-data/README.md) retains release-dependent escapes, malformed values and exact list data before C installation selection; no startup/filesystem or runtime result follows.

[Original expression equality](../refactor-expression-equality-20261015/README.md), [scalar fixture source](../scalar-expression-fixtures-20261015/README.md) and [fixed nonnumeric values](../nonnumeric-expression-literals-20261015/README.md) retain original six-provider observations separately from shared source syntax, corrected setup and Rust/F5/edit acceptance.

[Original Jim variable inventory SDK captures](original-jim-variable-inventory-sdk-20261015/README.md) retain133 frame-stage rows,17 physical key/header/count rows and9 true-top locals rows with exact excluded-trial/freshness provenance. No Runtime, table-census or complete lifecycle acceptance follows.

Original [configured package body-word values](../patterns/package-body-word-values/README.md) retain the five C Tcl and pinned Jim source/provenance/output channels captured before new expectations. Decoded quoted return, nested literal body data and dynamic missing-variable observations distinguish word values and control transfer; they grant no package loading, source-file, entered namespace or Native compiler authority.

Original [generated request word/container probes](../generated-request-words-20261015/README.md) retain all six provider/init receipts, actual capture driver and raw word/body/entry channels. Literal-value and enclosing braced-script byte preservation are independent: quoted source can activate substitutions or fail its enclosing body. These observations establish neither F5 request behavior, Rust acceptance, reached handlers nor path satisfaction.

Original [fresh dying-namespace operands](original-fresh-dying-namespace-20261015/README.md) retain separate legacy-trace and trace-add probes, all original channels and provider/init hashes. Reached C8.4 sees the dying name and evaluates it; C8.5–9.1 hide it from exists and refuse fresh eval. C9 legacy-trace and Jim trace-unavailable attempts remain explicit. Pinned conversion-source anchors are separate from build correspondence and establish no hot/cache/header, Runtime/VM, F5 or final-free/lifecycle acceptance.

[Original Jim namespace-delete helper](original-jim-namespace-delete-sdk-20261015/README.md) retains333 rows over16 public/direct cases and32 original-object COLD/REUSE phases, including getter order, actual replacement/deletion, callback counts and replacement List member identity. Destructive restoration advances epochs; no universal hot-hit, Runtime or final-free acceptance follows.

[Original inline-word and configured quoted-name values](../inline-word-source-values-20261015/README.md) retain all six original providers, matching Tcl libraries, source/capture provenance and exact output channels. Original substituted literal quotes remain data; raw source splicing fails on C Tcl or changes Jim values, and literal dollar data becomes a read in the raw proposal. Nested braced quote-name success on Tcl9 remains separate from C8/Jim refusals. The mixed C8.6/Tcl9-variable-axis Rust lexical control is not a C8.6 runtime observation; no Native read, cell, motion, frame, F5, Rust or whole-edit/evaluation equivalence is supplied.
