# Original command substitution geometry

Question: do brackets inside braced words, quoted words and comments terminate
the containing command substitution, and which separators delimit its script?

`capture01` records8 exact programs on each original C Tcl8.4.20,8.5.19,8.6.18,
9.0.4 and9.1.0 interpreter provisioned by `scripts/dev/tcl-reference-toolchains.sh`.
Each provider reports its actual patchlevel, library and executable before the
reports. Its complete captured C/header/Tcl/build/binary/library input hashes
are unchanged before/after execution. The exact standalone driver, input,
invocations, stdout/stderr and process completion are retained; no tcltest
harness was used. Run `python capture.py` in a fresh capture location to repeat.

All five providers agree on these controls: braced and quoted close brackets
stay inside their words; a commented close bracket stays in the comment; an
open bracket in a braced word is data; adjacent substitutions remain separate;
a quoted semicolon is data; actual script commands both execute; the incomplete
braced input reports `missing close-brace`. Hex reports are the actual `list`
result string, including its quoting, rather than a reconstructed element value.

`failed-setup01` is a separate reporting-harness failure: Tcl8.4 has no `binary
encode` command. It supplies no admitted substitution result. The corrected
capture uses original `binary scan H*` on every release. No Jim/F5 or compiler
Native entry, binding, completion, edit or optimisation authority is claimed.

Compiler original bracket/terminator consumers delegate to the configured
Lexer owners. `codegen::cmd_subst::tests::original_substitution_geometry_preserves_braces_quotes_and_comments`
retains the observed complete/malformed/counterexample geometry; Rust execution
is still required independently of this original-provider capture.
