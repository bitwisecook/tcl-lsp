# Literal SpecTest command presence

The actual CLI prelude queries the original command operand in the global lookup
frame through `namespace which -command`, then passes that same operand to the
autoloader. Wildcards remain literal data rather than info-commands patterns;
leading colons remain original input to the selected interpreter.

The extracted prelude runs under C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and latest Jim
0.84-9-g5bac7c9. Each gives6 query rows: two wildcard-containing missing commands
remain absent despite matching neighbours, and a literal-star command is present
through original/unrooted/overrooted written operands. C5's lone-colon command is
present. Jim reports that operand missing; its direct invocation control records
the independently observed result. No C expectation is transferred to Jim.
Raw reports, source and executable hashes are retained. Matching original init
libraries are used; no Tcltest is used.

The first capture's attempted common C/Jim expectation was rejected by Jim's
actual lone-colon query. The retained final program includes the direct-call
control and keeps provider-specific expectations. Process exit alone does not
establish command presence.

These36 original query observations cover this helper's input/context boundary.
CLI binary, pack discovery, loader/generator integration and variable snapshot
filters remain separate obligations. No F5 or private allocation authority is
inferred. Replay the retained probe using each matching original interpreter.
