# Original formatter variable-reference values

These fixed cases ran on original Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0
and pinned Jim before the formatter reference-proposal conformance assertions.
The original file driver, probe, manifest and all twelve output channels remain
byte-identical. All six processes exited zero and all six empty stderr channels
are retained. The manifest records actual patchlevels, selected original C Tcl
libraries and executable/init.tcl hashes before and after each run.

The original scalar `$x` and `${x}` cases both return SCALAR. A separately
computed array index `$a($i)` returns ELEMENT, while the braced combined name
`${a($i)}` returns LITERAL from the literal `$i` index. Quoted receivers retain
the same distinction. Literal-dollar and empty scalar names remain data. The
nested closer-name case succeeds on Tcl 9.0/9.1; C8 and Jim preserve their actual
missing-variable result after selecting their earlier closer. No refusal is
repaired or omitted.

These are bounded original source-reference/value observations. They establish
no formatter or whole-transformation equivalence, F5 behavior, Native/compiler
admission, binding/frame/cell identity or edit permission. The formatter's shared
syntax proposals and independent eligibility remain separate. See the
[name-handling contract](../../../docs/design/contracts/name-handling.md) and
[shared owners](../../../docs/design/contracts/shared-utility-contracts-rust.md).

`sha256sum -c SHA256SUMS` verifies the retained evidence/support files. Run
`python3 replay.py --out /absolute/fresh-directory` for a separate reproduction;
it checks the original providers/init files and never rewrites these receipts.
Historical absolute paths inside the original capture driver/manifest are left
unchanged; the separate replay uses this archive's probe and manifest. The replay
was AST checked only when archived. Adoption must copy all eighteen afterimages,
including every zero-byte stderr omitted by a unified diff.
