# Registry and VM naming consumer verification

The frozen capture runs 49 exact selectors against the recorded source hashes.
43 tests pass; six Compiler tests cannot build because a new test fixture imports
`Provenance` through a private re-export. Every attempted selector and its raw
output remain recorded; no selector executes zero tests.

[The separate Compiler fixture capture](../registry-vm-isolated-compiler-fixture-correction-20261013/)
uses the public import and passes all six exact Compiler selectors. Comparing the
source inventories shows only that test file changed. Production bytes match.
Together these receipts accept the 49 distinct planned selectors; the original
49-attempt receipt remains 43 passes and six build blocks.

The accepted scope covers:

- two pure query grammar/projection controls and 20 Registry metadata, descriptor,
  collision, selected-worker layout and namespace compiler controls;
- six Compiler controls for descriptor indexes, emitted `RESOLVE_CMD`, selected
  hook refusal, and actual original ensemble prefixes and implementation tokens;
- 13 VM controls comparing 60 C namespace address observations, 48 C/Jim variable
  query observations, and retained/retired namespace ownership;
- six genuine VM public command-query controls comparing all 150 unchanged
  C8.4–9.1/Jim query observations plus 48 setup/context observations;
- genuine VM import-cache and counted TclOO variable-name controls.

The [original query input and results](../../namespace-command-query/) are
independent reference executions. Query answer bytes are compared as reports;
they never reconstruct command identities. Jim's Core patchlevel `0.84` and the
reference build `0.84-9-g5bac7c9` are checked separately. These controls establish
Rust Core adapter behavior and retained metadata, without a Native C allocation
or F5 conformance claim.

`records.json` contains exact commands, executed test counts, raw output hashes
and executed binary hashes. The source inventories, compiler/toolchain identity
and copied capture runner bind each result to its actual inputs.
