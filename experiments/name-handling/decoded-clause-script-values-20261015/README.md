# Decoded clause script values

These fixed original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned
Jim 0.84-9-g5bac7c9 observations preserve the distinction between a selected
script value whose one command word is `{return one}` and a script invoking
builtin `return` with argument `one`.

The original `if`, `elseif` and `else` literal-command cases all report catch
code 1, `invalid command name "return one"`. The plain return case reports catch
code 2 and result `one`. Removing the literal command-name braces from an
already-decoded body value changes the command. A refactor indentation helper
must not perform that second source-word unwrap.

The probe, original manifest and all stdout/stderr files are byte-identical
copies. Each provider printed its actual patchlevel; the manifest retains all
process statuses, selected original libraries and provider/init/output hashes
before and after. No `tcltest` is used. Empty stderr channels remain explicit.
These are fixed script-value observations, not Rust/LSP acceptance, general
multiline indentation equivalence, F5 execution, Native or edit permission.
Earlier canonical proof receipts remain unchanged.

Run `python3 replay.py --repository /path/to/tcl-lsp --out /tmp/new-clause-values`
to capture a separate result with byte-identical local original providers.
The runner refuses changed probes or providers and never rewrites this receipt.

See the [name-handling contract](../../../docs/design/contracts/name-handling.md),
[shared owners](../../../docs/design/contracts/shared-utility-contracts-rust.md)
and [pattern catalogue](../patterns/README.md).
