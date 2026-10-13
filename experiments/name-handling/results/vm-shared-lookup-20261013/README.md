# VM shared name lookup observations

Question: does the live VM command-table adapter select the same registered
identity as the retained C oracle command vectors after consuming the shared
current/path/root owner, while keeping pending publication and namespace
incarnations independent of display text?

Eight exact tests pass. [records.json](records.json) retains commands, raw
channel hashes and exits; numbered directories retain complete stdout/stderr.
Every selection executes exactly one passing test. [Provenance](provenance.json)
identifies the captured test binary hash, compiler/Cargo versions, actual
build configuration, scoped source hashes and relevant input snapshots.
The [before](source-hashes-before.json) and [after](source-hashes-after.json)
source stamps agree. The capture never shares its Cargo target with another
builder.

## Live command identity

The [shared command vectors](inputs/command_resolution_vectors.txt) have 46
rows per release. The actual VM core is created in each tested C profile;
fixture declarations allocate real VM namespace and command slots. Original
unique definition labels record those registered slots. The oracle winner only
selects a label; returned text is never reparsed into namespace identity.
The opaque handlers are not executed.

| Profile | Actual lookup comparisons | Original setup boundaries |
| --- | --- | --- |
| C 8.4 | 30 | 16 |
| C 8.5 | 41 | 5 |
| C 8.6 | 46 | 0 |
| C 9.0 | 46 | 0 |
| C 9.1 | 46 | 0 |

All 209 compared identities match. The 21 setup boundaries remain unexecuted
lookup questions: eleven C8.4 namespace-path controls and five rejected original
definition setups on each of C8.4/C8.5. Exact vector lines and reasons are
printed in [the live test stderr](01/stderr.txt). Ordinary missing-command
vectors are compared, not omitted.

The C expected columns are retained reference observations independently checked
by the shared Syntax oracle gate. This capture does not rerun the C programs,
execute Tcl procedure publication, establish native C allocation equivalence or
claim general VM conformance.

## State and identity controls

Three existing pending-publication storage controls pass: create/delete
tombstone candidate selection, missing deletion without namespace creation, and
retained-context creation beneath its actual token. Their storage-unit subject
remains separate from original C execution.

Four actual native-core namespace controls pass: colliding colon displays retain
distinct contexts, deletion/recreation cannot retarget an old token, replay
retains its original context, and foreign-interpreter contexts are rejected.
These fixtures use `Vm::with_native_core`, preserving the constructor subject.

The executable includes the two VariableRoot geometry adapters in
`interp.rs`; their scoped bodies are retained. Compilation and these command
tests do not establish separate variable-cell conformance. Namespace-empty
adaptation and public-source execution are also outside this capture. No Jim
or F5 outcome is claimed. The compiler emits ten warnings, retained in raw
stderr, and no compilation error.

Reproduce one exact selection with the coordinated, isolated Cargo target:

```sh
source /tmp/name-env.sh
CARGO_TARGET_DIR=/absolute/isolated-target cargo test -p tcl-vm --lib \
  interp::native_command_resolver_tests::live_lookup_matches_shared_oracle_command_vectors \
  -- --exact --nocapture --test-threads=1
```

The other seven exact selectors are retained in `records.json`. The observed
results certify their recorded binary and source/input hashes; later source
changes require their own scoped validation.
