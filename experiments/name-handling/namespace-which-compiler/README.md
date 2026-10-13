# Original namespace which compiler selection

Question: does the real C compiler inline `namespace which -variable`, or
deliberately enter the runtime namespace query handler?

[input.tcl](input.tcl) asks the original interpreter to disassemble 16 exact
ASCII source forms. The queried scripts are compiled and not executed. Every
release retains raw stdout/stderr, exact command/input/binary/init hashes and
reported interpreter identity. No Tcltest is used.

| Reference | Available rows | Bytecode observations |
| --- | --- | --- |
| C 8.4.20 | 0 of 16 | Original `tcl::unsupported::disassemble` is unavailable; all 16 refusals retained |
| C 8.5.19 | 16 of 16 | All forms emit `invokeStk` |
| C 8.6.18 | 16 of 16 | Seven accepted command forms emit `resolveCmd`; nine declined forms emit `invokeReplace` |
| C 9.0.4 | 16 of 16 | Same bounded classification as C 8.6 |
| C 9.1.0 | 16 of 16 | Same bounded classification as C 8.6 |

Accepted command forms are bare names, literal `-command`, its `-c` and
`-comman` prefixes, a substituted name following literal `-command`, and
quoted/braced `-command` options. Declined forms are `-variable`, `-v`,
`-commandx`, `-`, `-unknown`, wrong arity, a substituted option and a
backslash-containing option. All labels and exact source bytes remain in the
input and raw output.

For C 8.6–9.1, the public ensemble's `-variable` fallback pushes the actual
private name `::tcl::namespace::which` and emits `invokeReplace 4 2`. A proved
worker hook decline does not justify replacing that original private-worker
lookup with arbitrary public command dispatch. Accepted command syntax
requires its own original instruction recipe and lookup owner.

`classify.tcl` parses the retained Tcl-list output and classifies only opcode
names. Per-release `classification.txt` and `classification-records.json`
identify that derived parser invocation separately from the oracle experiment.
The raw listings retain compiler pointers and epochs; those incidental values
must not supply stable conformance expectations.

`upstream-source-integrity.json` identifies independently fetched Git blob
SHA values at the shared manifest's immutable upstream commits. The observed
C 8.5 monolithic namespace compiler and C 8.6–9.1
`TclCompileNamespaceWhichCmd` source files match those complete original blobs.
The modern worker accepts only a SIMPLE_WORD `-command` prefix of two through
eight bytes; it declines variable queries and non-SIMPLE options. Original
source provenance and observed source hashes are separate from the binary
identity and actual disassembly results.

The real execution results for the eight namespace-variable queries are
retained independently in `../variable-namespace-query-bases/`. This compiler
experiment does not establish VM admission, runtime variable binding,
namespace lookup conformance, raw NUL behavior, Jim behavior or F5 behavior.

Reproduce with a matching original C interpreter and library into new files:

```sh
TCL_LIBRARY=/absolute/tcl8.6.18/library \
LD_LIBRARY_PATH=/absolute/tcl8.6.18/unix \
/absolute/tcl8.6.18/unix/tclsh \
  experiments/name-handling/namespace-which-compiler/input.tcl \
  > /absolute/new-stdout.bin 2> /absolute/new-stderr.bin
```
