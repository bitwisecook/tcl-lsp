# Static name-advice experiments

These are retained experiments, copied from existing temporary logs and source
receipts. No test, build or counterfactual was rerun during retention. Names such
as `current`, `fresh` and `head` belong to the original captures; they do not
identify the repository's current HEAD or its changing working tree.

| Question | Captured answer | Experiment |
| --- | --- | --- |
| Do bounded conditional source namespace changes pass owner controls and the actual packaged LSP? | Focused controls pass; six old owner failures remain exactly unchanged in the partial counterfactual; both packaged LSP tests fail. | [Conditional source namespaces](conditional-source-namespace/README.md) |
| Do source-selected Tcltest lifecycle changes restore genuine missing-read diagnostics? | Six Tcltest controls pass; three genuine missing-read tests still fail. | [Source-selected Tcltest lifecycle](source-selected-tcltest-lifecycle/README.md) |

The experiments share [original provenance](provenance/README.md); large source
snapshots, patches and inventories are retained once. They are diagnostic
evidence, not canonical semantic proof records or a gate pass.

[records.json](records.json) maps every raw capture to its original temporary
path, byte count and SHA-256. [integrity.json](integrity.json) records exact-copy,
source-receipt and failure-set checks. From this directory, run:

```sh
sha256sum -c SHA256SUMS
```

This verifies all retained raw captures and generated indexes against
[SHA256SUMS](SHA256SUMS). The hashes computed during retention establish byte
integrity; they do not supply missing original build or invocation receipts.
