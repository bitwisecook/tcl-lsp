# Literal SpecTest command presence

The CLI prelude asks the selected interpreter's global command inventory with
an exact literal pattern projected by the shared Syntax glob owner. It passes
the original command operand to the autoloader. It neither strips colons nor
interprets wildcard data as an unquoted name pattern.

The current extracted prelude and actual shared pattern projector run under
C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and latest Jim0.84-9-g5bac7c9. Each gives8 rows:
the lone-colon command is present; two wildcard-containing missing commands stay
missing despite matching neighbours; literal star/bracket/backslash commands
are present, including the star name's root/overroot written variants. All48
query observations pass their exact recorded expectations. Original executables
remain unchanged. Matching original init libraries are used; no Tcltest is used.

`namespace-query-attempt/` preserves the rejected presence recipe. Jim reports
`:literal` missing through namespace.which while direct invocation succeeds.
That query purpose cannot certify callable command presence for this tool.
Its original raw reports remain separate from the literal inventory controls.

A CLI/pack integration regression covers the actual discovered spec commands
and selected original C/Jim interpreters. Its Rust execution is pending; these
receipts establish the extracted helper/pattern boundary only. Variable snapshot
filters and whole-repository conformance have separate obligations. No F5 or
private allocation authority is inferred.
