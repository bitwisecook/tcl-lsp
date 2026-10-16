Root-only replay preparation. This package contains NO future source/ELF/listing binding and has run no executable. Both commands require an explicit --build-dir; a failed/running build, missing actual artifacts or current source drift is terminal. No predecessor/fallback ELF is accepted.

After Root records an actual PASS_FIVE_LIBRARY_BUILD (exit0,39001 unchanged inputs, ordinary, no instrumentation, TEST_DEBUG0), freezes all five actual artifacts in that directory, and chooses that exact directory as TCL_LSP_REPLAY_BUILD_DIR:

```bash
source /tmp/naming-env.sh
export TCL_LSP_REPLAY_BUILD_DIR=/absolute/actual-successful-build-dir
python3 /tmp/lsp-cumulative-source-owners-selection-v2/generate_selection.py --freeze --destination "$TCL_LSP_REPLAY_BUILD_DIR" --output "$TCL_LSP_REPLAY_BUILD_DIR/selection.json"
python3 /tmp/lsp-browser-logical-targeted12-launcher-v1/generate_subset.py --build-dir "$TCL_LSP_REPLAY_BUILD_DIR" --frozen584 "$TCL_LSP_REPLAY_BUILD_DIR/selection.json" --output "$TCL_LSP_REPLAY_BUILD_DIR/targeted12-selection.json"
```

The subset generator prints its actual manifest SHA256. Use that exact digest, never a placeholder or a previous selection digest:

```bash
python3 /tmp/lsp-browser-logical-targeted12-launcher-v1/launch.py --mode targeted12 --build-dir "$TCL_LSP_REPLAY_BUILD_DIR" --manifest "$TCL_LSP_REPLAY_BUILD_DIR/targeted12-selection.json" --manifest-sha256 ACTUAL_TARGETED12_DIGEST --out "$TCL_LSP_REPLAY_BUILD_DIR/targeted12-results"
```

This is TWELVE ONLY: original rows222/223/224/232/242/270 verbatim, plus latest original-head-applicability, deferred package-alternative source-order, authored Jim doclink purpose, explicit directNative newvariant refusal and the two approved genuine Logical hierarchy/implementation positives. Every one of the twelve rows is copied verbatim from the ACTUAL frozen584. No row is recast or reordered within the full584. Partial failures remain actual failures; this run cannot claim full584 acceptance.

Only when Root separately chooses to execute the complete unchanged584, use the actual full manifest SHA256 printed by its generator:

```bash
python3 /tmp/lsp-browser-logical-targeted12-launcher-v1/launch.py --mode retained584 --build-dir "$TCL_LSP_REPLAY_BUILD_DIR" --manifest "$TCL_LSP_REPLAY_BUILD_DIR/selection.json" --manifest-sha256 ACTUAL_FULL584_DIGEST --out "$TCL_LSP_REPLAY_BUILD_DIR/retained584-results"
```

The launcher validates exact actual PASS + receipts + source.diff + full39001 current index + all five actual frozen ELF paths/bytes/SHA256 + all selected source/module cfg evidence + original required provider environment and actual tclsh PATH + TEST_DEBUG0. It invokes only the hash-bound canonical replay-selection.py, without subgroup/list-only/skip fallback. The canonical runner performs actual listing30, exact singleton180 and test-threads1. The launcher repeats full39001/allfive guards after the run even on failure, records an invocation receipt, and never overwrites existing output. Runtime/VM/BIGIP targets are excluded.

Unissued bindings remain unissued. This package was prepared by source/hash/AST reads only, not run. Root owns actual freeze, listing, execution, classification and ordinary acceptance. A previous failed repair5 build or current running repair6 grants no admission.
