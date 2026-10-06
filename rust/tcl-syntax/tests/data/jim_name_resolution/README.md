# Jim naming conformance

`jim_name_resolution_conformance.rs` compares shared `NativeNameProtocol::Jim084`
recipes with original source executed by a real Jim interpreter. The interpreter
is selected through `TCL_LSP_JIMSH`; `TCL_LSP_REQUIRE_JIM_ORACLE=1` requires its
presence. These fixtures cover 203 fixed inputs on Jim 0.84, patchlevel
`0.84-9-g5bac7c9`, source commit
`5bac7c99ad65864c87da513e22e2f01703fa4e03`.

The comparison covers namespace object canonicalisation and textual
qualifiers, tails and parents; procedure publication spelling and its actual
procedure namespace; ordered command lookup in Jim's flat table; global alias
local spelling; and scalar versus combined dictionary-sugar variable names.

Input tables retain bytes as hexadecimal. The first field names the input, the
second contains the actual counted namespace object, and the third contains the
original counted name. An empty field denotes an empty object. The separate
`lookup_keys.txt` table contains original global procedure declarations; `-`
denotes the empty declaration operand. Source templates substitute only the
hexadecimal context and name fields. `binary format H*` materialises the bytes;
there are no literal NUL bytes in the source files.

Inputs include empty names, single colons, root markers and partial colon runs,
embedded NUL bytes before and after qualification or parentheses, UTF-8 `c3a9`
and `65cc81` as distinct byte sequences, and invalid UTF-8 `ff`. Neither input
materialisation nor result comparison normalises Unicode or converts names
through Rust strings.

Jim preserves a procedure's original publication spelling for enumeration while
its flat command table compares a separately selected key. A declaration written
as `:::` can therefore enumerate as `:::` while lookup selects the empty key.
The tests compare those two purposes separately. Global alias output includes an
alias count, so an empty local name remains distinguishable from no alias.

`provenance.json` records the native binary and relevant source hashes, each
fully expanded original source hash, raw stdout and stderr as hexadecimal, and
process exit codes. Tests compute their expectations from the shared naming
owner and run the original sources directly. The recorded observations are
independent evidence rather than a replacement for that comparison.

These checks establish byte projections and observable naming behaviour. They
provide no native object header, reference count, compiler admission, cache
identity or interpreter authority. They do not establish C Tcl or BIG-IP
behaviour. A separately supplied root and index is a different variable API
purpose: Jim's Tcl `array` commands enter its combined variable grammar, so
these sources do not attest a separate variable API.
