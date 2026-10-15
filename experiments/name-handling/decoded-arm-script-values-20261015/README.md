# Decoded switch arm script values

This fixed probe records original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0
and pinned Jim 0.84-9-g5bac7c9. Actual patchlevels, catch codes, results and error
codes are in each original stdout file. Provider binaries and the original C
init libraries are hashed before and after execution; no tcltest is used.

Every observed provider treats an arm whose decoded body is `{return one}` as
one literal command word `return one`: catch code 1, invalid command name.
The different body `return one` invokes Return: catch code 2, result `one`.
The original switch/list value owner has already removed the body container.
Peeling braces from that decoded value again changes the command being named.

[manifest.json](manifest.json) and all stdout/stderr files are unchanged
original captures. [SHA256SUMS](SHA256SUMS) seals every archive file; empty
stderr files remain present. These observations were captured before the new
Rust conformance assertions. No refusal/error is converted into successful
execution, and no Rust/LSP result is inferred from these interpreter captures.

This evidence concerns script-value interpretation only. Original operation
selection, current source/configuration, Native input, frame/cell identity and
edit eligibility remain independently owned. The standalone DataGroup proposal
control uses supplied F5 Registry metadata only; this probe supplies no F5
runtime observation or capability.

See the [name contract](../../../docs/design/contracts/name-handling.md),
[shared owners](../../../docs/design/contracts/shared-utility-contracts-rust.md)
and [pattern catalogue](../patterns/README.md).

Run `python3 replay.py --repository /path/to/tcl-lsp --out /path/to/new-capture`
to create separate new receipts against the exact pinned providers. The runner
checks original hashes, retains every process status and never edits this
archive. A new compiled test replay remains a separate requirement.
