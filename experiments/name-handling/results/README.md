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
