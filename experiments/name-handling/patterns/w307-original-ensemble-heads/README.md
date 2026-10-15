The fixed original source invocations in `references.tcl` and
`command-prefixes.tcl` retain command-head concatenation observations under
C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 and pinned Jim. Their receipts
retain exact executable/input/output digests, source identity and capture times.
Each C process uses its appropriate original Tcl library; no `tcltest` or
replacement initialization script is involved.

A braced scalar reference can compose literal namespace text, including spaces,
literal sigils, an unmatched parenthesis or an empty scalar name. Nested-brace
reference grammar differs by release. A bare `$prefix::tail` reads one qualified
variable and supplies no prefix concatenation. These observations establish no
installed command, current lookup, variable cell or Native execution. W307
still checks independent value and command-slot evidence for scalar prefixes.

Command-substitution assistance retains its independent literal-tail
eligibility. The command-prefix probe separately records list representation
and nested closer data; a dynamic suffix can execute but remains outside
static component advice.
