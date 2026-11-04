# BIG-IP Tcl execution: <short descriptive title>

Create one file per script actually run. Follow the
[Tcl execution evidence standard](../../../docs/design/contracts/tcl-execution-evidence.md),
which defines the reversible notation for scripts and results.

## Question

<State the full LSP, compiler, or VM question this script was designed to answer,
why the distinction matters, and the specific case being exercised.>

## Assumptions

<State the experimental assumptions and limits, including relevant BIG-IP
context, profiles, event, unit, setup and whether the intended operation was
reached. Write Not recorded for missing details.>

## Script

<Insert the exact executed Tcl bytes as a tcl-visible block using
scripts/dev/tcl-script-to-block.py.>

## Results

### <BIG-IP version and execution context>

- Recorded version: <version/build or Not recorded>
- Recorded input channel: <channel or Not recorded>
- Exit status: <status or Not recorded>
- Standard output: <text-visible block, empty, or Not recorded>
- Standard error: <text-visible block, empty, or Not recorded>

For a result observed through another channel, such as a browser option or an
iRule event, add a Recorded result text-visible block and describe that channel.
Keep load acceptance or rejection distinct from runtime completion. Do not
infer results for an unexecuted context or infer an operation was reached when
setup failed.
