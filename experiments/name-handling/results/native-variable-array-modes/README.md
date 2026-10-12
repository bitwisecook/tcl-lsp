# Original variable declaration and array-name controls

`controls.tcl` is the complete portable experiment. Each `NH_CONTROL` row
retains its case ID, reached Tcl completion and result. C executions use the
source-built 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 binaries with their real
matching libraries; `library-records.json` and each library capture retain
`info library`, patchlevel, real Tcltest version and init/Tcltest hashes.
Jim is the independently built `5bac7c99ad65864c87da513e22e2f01703fa4e03` oracle.
Its helper syntax differs from C's; no C mode grammar is donated to Jim.
F5 answers for these exact inputs are not measured by this capture.

`oracle-records.json` retains executable/input/output hashes and exact argv.
`conformance.tcl` retains the same selected controls as a returned list for the
native regression tests. Its only adaptations are accumulating rows rather
than printing them and leaving `names-regexp` in the separate complete
experiment. `conformance-records.json` identifies those separately executed
C/Jim inputs and outputs. No expected result is inferred from our engines.

| Question | C 8.4/8.5 | C 8.6/9.0/9.1 | Jim |
| --- | --- | --- | --- |
| Bare `variable`, root/namespace/procedure | Error, `?name value...? name ?value?` | Empty success | Error, `name ?value?` |
| `array names` glob/unique-prefix glob mode | Supported | Supported | Wrong argument count |
| Explicit exact mode | Supported | Supported | Wrong argument count |
| Explicit regular-expression mode | Supported | Supported | Wrong argument count |
| Sole trailing `-glob` word | Literal glob pattern | Literal glob pattern | Literal glob pattern |

`runtime-before-8.5` preserves the old executable's disagreement on all three
bare-variable contexts and explicit array modes. Its executable hash matches
the immutable six-suite subject in `../runtime-current-tcl8.5.19`; that
subject's real Tcltest failures are `var-7.16`, `var-7.17` and `var-14.2`.

The shared array-name worker uses native `ArrayNamesSearch` matching, including
its release-selected trivial-pattern lookup. Exact C matching uses the shared
C-string extent; mode words use original-object option lookup. The original
regular-expression enumeration worker remains an explicit host refusal in our
engines: C regex success in these oracle captures is not an implementation
conformance claim. Future support must preserve original-object regex policy,
conversion/cache behaviour and entry ordering.

The native runtime regression is
`runtime/rust/tests/native_variable_array_modes.rs`; the native VM declaration
regression is `rust/tcl-vm/tests/native_variable_declaration.rs`. Fresh runtime test and
real upstream-suite captures are separate files or directories; old raw
captures must not be overwritten or claimed for a later executable.

`runtime-test.log` records the passing native matrix test across the five C
profiles and Jim. The five `runtime-after-*` directories capture the frozen
actual native runner before loading init; each result matches its corresponding
C conformance output byte for byte. `runtime-records.json` retains the
executable SHA-256 and distinct startup scope for those controls.

`upstream-var-8.5/records.json` records the complete original 8.5.19 `var.test`
with real matching init/Tcltest: C and runtime each pass 69 tests, skip 17 and
fail zero. The three previously failing IDs execute and pass. This frozen
runner's build/source evidence is in `../native-compiled-alias-cells/`.
The separate `vm-variable-declaration-test.log` and its hash/build record
retain the passing VM declaration regression: zero, three and four operands
across all five C profiles and Jim. This is a direct native command invocation
control; it does not claim compiled-source or whole-file VM conformance.

`extra-arity.tcl` and `extra-arity-records.json` retain a further independent
boundary: all five C releases accept three/four `variable` operands, while Jim
rejects them with its `name ?value?` usage. The shared declaration grammar accepts
exactly one or two operands for Jim while preserving C's variadic declarations.
`runtime-max-arity-test-fixed.log` records both native runtime tests passing
across all six profiles, including these upper-arity controls. The earlier
`runtime-max-arity-test.log` preserves a test compilation failure and contains no
guest outcomes. The successful results belong to the build captured in
`../native-c84-declared-cell/`; previous runner results remain separate.
