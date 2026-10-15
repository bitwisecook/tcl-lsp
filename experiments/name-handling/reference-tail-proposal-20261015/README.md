# Variable-reference tail proposal grammar

These fixed probes observe original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and the existing pinned Jim 0.84-9-g5bac7c9 provider. Each provider and
original C init library is hashed before and after execution. No tcltest is used.

The source strings are constructed from character data so the probe itself can
contain each release's spelling without its host script parser consuming it.
The controlled probe isolates completeness, missing-name reads and populated
reads in separate processes and prints the interpreter's actual patchlevel.

| Provider | Original `${ns{::x}}` | Proposed `${ns{::new}` |
| --- | --- | --- |
| C 8.4, 8.5, 8.6 | FirstClose reads `ns{::x` and leaves a literal `}` | Complete; reads `ns{::new` |
| C 9.0, 9.1 | Nesting reads `ns{::x}` | Incomplete; missing close-brace for variable name |
| Pinned Jim | FirstClose reads `ns{::x` and leaves a literal `}` | Complete; reads `ns{::new` |

The controlled follow-up completed all eighteen provider/case runs.
The first Jim trial exited with signal 11 and emitted no result; that original
trial remains in [initial/manifest.json](initial/manifest.json). The successful
isolated observations are separate receipts and do not rewrite or explain that
signal. Unsupported operations, missing reads and nonzero statuses remain
observations rather than expected successful variable reads.

[controlled/manifest.json](controlled/manifest.json) records the exact selected
provider, original library, argv, status and output hashes. Original stdout and
stderr files remain byte-for-byte captures; [SHA256SUMS](SHA256SUMS) seals the
archive. Both original manifests are preserved without path or receipt edits.

This evidence concerns source constructor validity only. It supplies no current
analysis, namespace identity, entered frame, variable cell, lookup, invocation,
Native capability or edit eligibility. The shared constructor must independently
parse the complete proposal under the caller's actual configuration, preserve
its original literal/computed index bytes, and refuse changed name/root/index
shape. LSP and Syntax tests require their own actual fresh compiled artifacts.

See the [name handling contract](../../../docs/design/contracts/name-handling.md),
[shared owners](../../../docs/design/contracts/shared-utility-contracts-rust.md)
and [pattern catalogue](../patterns/README.md).

To repeat the controlled probe against these exact pinned providers without
changing the retained receipts, run `python3 replay.py --repository /path/to/tcl-lsp
--out /path/to/new-capture`. The runner verifies the original provider/library
hashes and writes a new manifest. `--initial` separately repeats the original
trial, including any refusal or signal; no result is promoted to a conformance
pass by the runner.
