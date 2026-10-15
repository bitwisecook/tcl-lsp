# Reference-body variable source geometry

Two questions distinguish source grammar from sandbox admission. Does
`${a{b}::x}` read the qualified variable, and do braces inside an array index
suppress `$::g`? The exact input, completions, hexadecimal results, provider
identities and hashes are retained beside `capture.py`.

C Tcl 8.4.20, 8.5.19 and 8.6.18 stop the braced name at the first close brace;
9.0.4 and 9.1.0 read the qualified name and return `OUT`. The 8.x interpreters
substitute the global inside the index braces and return `IN`; 9.x reject the
literal index braces. Jim at the freshly rechecked upstream revision retains
the former answers. These direct-script observations do not use Tcltest and
establish no Rust execution or Native compilation acceptance.

`initial/` preserves the first direct measurement before explicit library
identity was included. `paired/` selects each C release's original source
library, records its reported library and hashes the unchanged original init,
parser, binary and build configuration. No expected answer comes from the Rust
implementation. The fixed Registry control uses the actual grammar distinction;
raw index substitution is checked by shared word components.
