# Original source normal results

`source.tcl` observes selected native completion codes and result bytes for final
commands, caller-specific formal values, numeric calls, return routes and command
replacement. Each output file is produced by the corresponding native engine.
The C 8.6 analysis tests use its selected math-function command contract; engines
without that command-based lookup retain their own recorded behavior.

The source inventory joins conditional successful result bytes independently of
representation and total completion. A pending return becomes a successful
procedure result only at that procedure's native completion boundary. Actual
caller results cannot establish a declaration-wide constant.
