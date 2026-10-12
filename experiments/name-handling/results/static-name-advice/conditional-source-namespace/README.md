# Do bounded conditional source namespace changes pass owner controls and the actual packaged LSP?

The retained runs do not establish a complete pass. The five focused logical
namespace tests, one conditional native catalogue/import test and one lifecycle
control pass. The 12-test source-advice owner group has six passes and six
failures. Replacing only the captured source-advice owner files with their
original `head` versions leaves the exact same six failures. Both actual
packaged LSP semantic-token tests fail.

| Capture | Result |
| --- | --- |
| [Focused logical namespace controls](logs/source-advice-current-focused-logical.log) | 5 passed; 0 failed |
| [Focused native conditional imports](logs/source-advice-current-focused-native.log) | 1 passed; 0 failed |
| [Lifecycle control](logs/source-advice-current-lifecycle.log) | 1 passed; 0 failed |
| [Owner controls](logs/source-advice-current-owner-controls.log) | 6 passed; 6 failed |
| [Partial owner-file counterfactual](logs/source-advice-counterfactual-owner-controls.log) | 6 passed; 6 failed; exact failure set unchanged |
| [Packaged LSP regressions](logs/source-advice-packaged-lsp-regressions.log) | 0 passed; 2 failed |

The packaged failures are
`semantic_tokens::test_imported_tcltest_test_structure_recognised` and
`semantic_tokens::test_namespaced_imported_tcltest_test_structure_recognised`.
Their imported `-body` option remains classified as a string in these captures.
Focused owner success does not establish packaged consumer success.

The [original comparison receipt](../provenance/source-advice-counterfactual/comparison.json)
lists all six owner failures without shortening their names and identifies the
binary and both log digests. [The source manifest](../provenance/source-advice-counterfactual/manifest.json)
names original commit `bd062ae6c9312f2db0ed276b9c063bec576dd74c`, records the
five owner inputs, and explicitly says “HEAD source-advice owner files over
otherwise current work-in-progress; not pristine HEAD”. The original
[restoration receipt](../provenance/source-advice-counterfactual/restoration.json)
reports restored owner hashes and unchanged other-work-in-progress hashes.
This is a bounded counterfactual, not a pristine-commit baseline or a guarantee
that the full tree remained unchanged throughout every other run.

[The shared provenance index](../provenance/README.md) explains the exact source
backups and patches. There is no complete original source/build hash receipt
for every focused, cached or packaged invocation; no current source hash is
substituted for that missing provenance. The owner counterfactual establishes
an unchanged observed failure set, not the cause or age of every failure.

All other original source-advice logs are retained as investigation context:
[earlier owner controls](logs/source-advice-existing-owner-controls.log),
[fresh logical controls](logs/source-advice-fresh-logical.log),
[fresh native control](logs/source-advice-fresh-native.log),
[cached logical controls](logs/source-advice-logical-cached-binary.log),
[cached native failure](logs/source-advice-native-cached-binary.log),
[native trace](logs/source-advice-native-trace.log),
[logical build output](logs/source-advice-logical-focused.log), and
[native build output](logs/source-advice-native-focused.log).
The last two contain no final test result and cannot be counted as passes.
Earlier cached failures do not override the later scoped pass receipts.

See [retention records](../records.json) and [integrity checks](../integrity.json).
