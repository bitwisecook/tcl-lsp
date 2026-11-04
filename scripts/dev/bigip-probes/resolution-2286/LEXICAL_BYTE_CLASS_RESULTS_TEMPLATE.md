# BIG-IP lexical byte-class execution: <context>

Use one file per executed script and follow the
[Tcl execution evidence standard](../../../../docs/design/contracts/tcl-execution-evidence.md).
Record the actual product/build; the reference appliance does not establish the
version of a later run.

## Question

How do the tested byte values and source producers affect Tcl's recognition of
braced and unbraced variable names in this execution context, and what does
that mean for the LSP and compiler's lexical name handling? State the precise
cases and boundary this script was built to test.

## Assumptions

Record the execution context, source producers, newline format, applicable
profiles and TMM units. State which setup and operations were reached and any
missing or incomplete chunks. Write `Not recorded` for missing information.

## Script

Insert the exact executed Tcl bytes as a `tcl-visible` block using
`scripts/dev/tcl-script-to-block.py`.

## Results

### <BIG-IP version, context and unit>

- Recorded version: <version/build or Not recorded>
- Recorded input channel: <channel or Not recorded>
- Exit status: <status or Not recorded>
- Standard output: <text-visible block, empty, or Not recorded>
- Standard error: <text-visible block, empty, or Not recorded>

For a result observed through another channel, add a Recorded result
`text-visible` block and describe that channel. Keep loader rejection, runtime
completion and an unreached operation distinct. Do not claim behavior for an
unmeasured context or fill missing rows from another interpreter.
