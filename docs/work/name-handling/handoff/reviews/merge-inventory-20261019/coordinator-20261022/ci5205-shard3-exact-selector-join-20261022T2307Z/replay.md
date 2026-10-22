# Reproduce the CI5205 shard-3 exact selector join

The retained raw log is `shard3-raw.log` (SHA-256 `278aff271a8b44ec5def7e6180349d64e1d5e7e7916dd76a2a5f9678a79c9c45`). Current named outcomes are `current-outcomes.json` (SHA-256 `f0fc9917bb94759a1afdc34201676f506345faaba4d3c33822ef1e89291d7444`), parsed by the exact archived `parse-current.py` (SHA-256 `7c5444d710469b271096d45b8ff5c794fcd6bb287d9ad2d908ac582b19d4a6d2`). The full comparison is stored losslessly as `comparison.json.gz`; its uncompressed original is 16,443,673 bytes, SHA-256 `19efa358f42a6378780ec5d5badf4843d0ee5b16ea33f51e8c3cdc829ef87015`.

The parser intentionally preserves the original run-time locations. To replay parsing, from the repository root run:

```sh
mkdir -p /tmp/ci5205-shard3-exact-join
cp /workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/ci5205-shard3-exact-selector-join-20261022T2307Z/shard3-raw.log /tmp/ci5205-shard3-120061100614.log
python3 /workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/ci5205-shard3-exact-selector-join-20261022T2307Z/parse-current.py
python3 /workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/baseline-exact-comparator-20261022/ci5203_exact_selector_compare.py /workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/origin-rust-ci-5203-outcomes-20261022.json /tmp/ci5205-shard3-exact-join/current-outcomes.json --output /tmp/ci5205-shard3-comparison-replay.json
```

The baseline is the existing immutable 24,312-row CI5203 named-PASS receipt at `/workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/origin-rust-ci-5203-outcomes-20261022.json` (SHA-256 `e481a695912b73493afff3238318c2d77d042bbb5d4dfd2e75d754ca7f97e7df`); the fail-closed comparator is `/workspace/naming-migration-active/tcl-lsp/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/baseline-exact-comparator-20261022/ci5203_exact_selector_compare.py` (SHA-256 `65cb36e14d9145039546e4b5a0b0de02b01e9feb749bc05f2d846d7c3625eaf2`). Identity is exactly `(binary_id, selector)`. The original comparison bytes hash to the SHA recorded in `receipt.json`; no baseline rows or outcomes were rewritten. Missing/skipped selectors remain unknown; current nonpasses without a baseline row remain separate.
