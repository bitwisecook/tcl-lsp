Exact retained selections, recovered without executing Cargo, providers, binary listings or tests.

Use this V3 directory. V2 remains byte-identical to its published manifest. The new adapter's Formatter environment handling now follows the inherited ordinary environment, while Runtime/VM retain their explicit original provider environment. V3 also retains source-function hashes and provider source/configuration/harness references.

Runtime174 is exactly Runtime lib120 + native_namespace_command_query51 + Registry lib3. The original plan's110 Runtime selectors are followed by its ten original Jim compatibility selectors. The complete ordered vector equals both the actual archived8df exact-plan.json and result.selected_tests. Separate Runtime API4 and engine/WASM plan fields are excluded from174. Historical171PASS3FAIL is archived evidence only; it establishes no current result.

VM133 is exactly VM lib117 + native_compilation_conformance7 + cross_interp_alias_e2e2 + Syntax lib7. Original25 priority selectors run first, followed by the remaining original target/order vector. Its complete exact plan and original runner were recovered from the SHA-verified validation-plans archive. No old executed VM133 vector comparison is claimed here.

Formatter88 uses tcl-lsp-core --lib; integration55 uses --test formatting_engine_residual; integration15 uses --test lsp_core_residual. Their exact original JSON hashes match the independently retained name-current238-consumer-plan-peer review. Every vector is complete, nonempty, unique and has no filters. All original test budgets are180 seconds. Runtime/VM original listing budget30 seconds, build1800; Formatter's original failclosed253 runner supplied no separate listing timeout and no explicit build timeout. The compact adapter adds a30-second listing bound and never builds.

Files:
- *-selection.json: complete exact vectors, order, targets, original build command/metadata, environment, budgets and count guards.
- *-exact-plan.json: per-target exact/filter-free plans compatible with Root's retained failclosed replay shape. VM target order includes its original priority ordering.
- retained/: exact small archive members, including original runners and original metadata. Old historical runners keep their original source/provider admission guards and are not ready to run against current source without a reviewed source rebind.
- retained-members.json: original archive/member hashes and byte counts. Both outer archives were independently checked against handoff/packets.json SHA256 before selective reads.
- manifest.json: hashes for the compact package and recovered vectors.
- replay-selection.py: NEW compact capture adapter, AST checked only. It does not implement the historical source/provider/compiled-input admission gates. It always labels results SELECTION_CAPTURE_ONLY_NO_ORDINARY_ACCEPTANCE. Root must join independent fresh ordinary build/source/provider/compiled-input receipts before interpreting results as acceptance.

The adapter requires an artifact map created from Root's actual current Cargo/source receipts:

    {"tcl_runtime": {"path": "<actual fresh current binary>", "sha256": "<actual captured digest>"}, ...}

It verifies artifact hashes, a nonempty actual listing with each selected name occurring exactly once, exact test argv, singleton run count, exactly ok1/0 summary, original180-second process-group deadline and unchanged artifacts. It executes serially, retains every raw log, and captures failures without dropping later selectors. A --group capture is permanently partial; Runtime-lib120 cannot claim Runtime174. --list-only cannot claim executed controls. No Cargo/Git/provider/network operation occurs in the adapter.

Example, with the manifest hash from manifest.json:

    python3 /tmp/naming-retained-selection-recovery-v3/replay-selection.py \
      --manifest /tmp/naming-retained-selection-recovery-v3/runtime174-selection.json \
      --manifest-sha256 0c757949d6ae2d2790f3fc6f50c94eb5c55fa3b17d7dae094b8a78054ce5d5cf \
      --artifacts <Root actual current artifact map> \
      --root /workspace/tcl-lsp --out <new Root capture directory> \
      --group tcl_runtime

Omit --group only when every actual target artifact is supplied. Add --list-only for preflight. For VM/Formatter/integration, select the corresponding manifest and its own exact manifest hash. Do not execute historical rebuild scripts to compensate for a missing current artifact.

Ordinary environment: no TCL_LSP_TRACE* or TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC. Runtime/VM original environment clears TCL_LIBRARY/TCLLIBPATH and selects each C5/Jim original executable with REQUIRE_ALL_TCL_ORACLES=1 and REQUIRE_JIM_ORACLE=1. Only provider probes select their own original TCL_LIBRARY. Retain actual interpreter/library/config/harness/linked-library pairing; the adapter's environment settings alone are not that evidence. Original Formatter runner inherits Root's environment rather than clearing these variables.

The old generic failclosed253 script's process exit checks shape/drift, not every captured test's pass status. A zero runner exit cannot replace per-test exit/status/ok1/0 checks. The new compact adapter checks all those fields explicitly.

No current binaries were listed or executed during this recovery. No ordinary acceptance, migrations or helper counts are advanced.
