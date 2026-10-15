# Scalar variable source spelling

The original probe seeds every tested name, constructs a complete braced
reference, and evaluates that reference with the original C Tcl 8.4–9.1 and
latest Jim executables. It uses no Tcltest library. These are source
interpolation observations; they do not execute the Rust or LSP implementation.

| Name data | C8.4/8.5/8.6 and Jim | C9.0/9.1 |
| --- | --- | --- |
| Plain, space, literal `$` or `[`, backslash, newline, rooted or empty | Exact seeded value | Exact seeded value |
| Unmatched opening brace `a{b` | Exact seeded value | Incomplete variable-name error |
| Balanced braces `a{b}c` | First-close prefix value plus literal suffix | Exact seeded value |
| Unmatched closing brace `a_}_closebrace` | Missing truncated-name error | Missing truncated-name error |

The combined `selected_array(key)` row reaches a seeded array element on all
six providers. Lexical completeness does not make that operation a scalar
read. F5 is untested. Binary hashes identify the recorded executables; no
complete startup or library closure is claimed.

[probe.tcl](probe.tcl), the original outputs, and [manifest.json](manifest.json)
are byte-identical to the capture. The receipt retains its original absolute
paths. [archive-index.json](archive-index.json) maps those paths to files here
and checks their hashes. [provider-provenance.json](provider-provenance.json)
compares the recorded executable hashes with the independently retained
[reference-extent receipt](../variable-completion-extent-20261014/manifest.json).
Neither original receipt has been rewritten.

Run `python3 verify.py` in this directory to verify the portable archive. The
optional `--current-providers` check also compares installed executable bytes
with the recorded hashes; it does not execute the providers.

The [source-name contract](../../../../docs/design/contracts/name-handling.md)
keeps Logical source assistance separate from Native ownership. The
[scalar source proposals](../../../../docs/design/compiler/name-resolution-implementer-guide.md#exact-scalar-source-proposals)
use the selected full lexer grammar and independently check the complete
reference and literal name. These observations grant no Native naming recipe,
cell identity, current value, successful read, source applicability or edit
permission. Rust source-constructor and LSP completion checks are independent.
