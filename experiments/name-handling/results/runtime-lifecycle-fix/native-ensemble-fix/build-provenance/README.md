# Frozen build provenance

[Original process receipt](../receipt.json), [process records](../records.json) and
[unit records](../unit-records.json) remain unchanged. The process and unit
executables have separate recorded hashes; their outcomes must remain separate.

The 115,244,463-byte source patch is a broad post-build working-tree snapshot,
including unrelated documentation and proof metadata. It is build provenance,
not an experiment. Its exact chunks remain retrievable from published commit
`e803f53f1e55102b31bb5c051c86de1728cb38e0`; [manifest.json](manifest.json) records their paths/order/hash,
the base commit, scoped source hashes and unchanged raw-capture hashes.

Recover it without modifying the worktree or active git index:

```sh
python3 reconstruct-source.py /tmp/name-native-ensemble-source.patch
GIT_INDEX_FILE=/tmp/name-native-ensemble.index git read-tree c49ef6fd5d147edd0cf473070dfaac2834c20dd7
GIT_INDEX_FILE=/tmp/name-native-ensemble.index git apply --cached --whitespace=nowarn /tmp/name-native-ensemble-source.patch
GIT_INDEX_FILE=/tmp/name-native-ensemble.index git show :runtime/rust/src/cmd_namespace.rs
```

Run the helper from this directory or provide its repository-relative path.
It requires the archived commit locally, verifies every chunk and the complete
hash, and writes the recovered patch outside the tracked experiment directory.
Use a fresh temporary index path. `git show :PATH` under that index recovers any
snapshot file; source excerpts record their exact line ranges and whole-file hashes.

The original receipt says the patch was captured immediately after the build
while concurrent agents shared source. Its assertion that the snapshot preserves
the exact executable is not independent proof of stable compiler inputs. Retain
this limit: neither reconstruction nor these excerpts establish a reproducible
binary or that every snapshot edit entered either recorded executable.
