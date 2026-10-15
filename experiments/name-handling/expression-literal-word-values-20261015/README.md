# Original RHS source-word values

These fixed observations ran on the original Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0 executables and pinned Jim before the configured RHS literal
consumer's new Rust assertions. Each manifest retains the exact executable and
selected original init.tcl hashes before and after execution, process status,
stdout and stderr. All six controlled processes exited zero; empty stderr files
are retained. The selected original library was set explicitly for each C Tcl.

`controlled/` is the evidence capture with its actual pre-existing file driver,
`capture.py`, retained byte for byte. `initial-excluded/` preserves the earlier
inline-driver trial, including its manifest, probe and all output channels.
The initial trial has no retained Python driver file and is excluded as the
controlled capture authority; no driver is reconstructed for it. The controlled
manifest binds that initial manifest by hash. Historical absolute temporary
paths inside original receipts and the actual original driver are unchanged.

A trailing dollar in `"^/api/.*$"` remains literal. An actual `$suffix` reads
ALTERED, whereas escaped-dollar and hex-escaped-dollar values remain literal
sigil data. Braced command-looking bytes remain data, and escaped quotes decode
once. These observations concern Tcl source-word values, separately from F5
operator or regular-expression semantics, URI/path satisfaction, handler reach,
frame/cell identity, edit equivalence and Rust acceptance.

Verify every retained file with `sha256sum -c SHA256SUMS`. `replay.py --out
/absolute/new-output-directory` provides a separate portable replay using the
controlled manifest and probe, validating original provider/init hashes before
and after each process. It creates a new output directory and never rewrites
these receipts. This replay utility was syntax checked only when archived; its
existence is not another original observation. All 12 empty output channels in
both captures must be copied during adoption even though a unified diff omits
empty files.

See the [name-handling contract](../../../docs/design/contracts/name-handling.md)
and [shared semantic owners](../../../docs/design/contracts/shared-utility-contracts-rust.md).
