# tcl-source

Configured Tcl source syntax and geometry, independent of command lookup and
compiler binding. `cst` owns the lossless red-green tree, `segment` derives
commands and words, `spectcl` reads configured pack syntax, `words` represents
word geometry and provenance, `text` supplies spelling suggestions, and
`limits` supplies shared structural recursion bounds.

The only workspace dependencies are `tcl-lexer`, `tcl-syntax`, `tcl-dialect`
and `tcl-core-types`. Grammar comes from the selected lexer configuration.
Command-role dispatch and recovery diagnostics belong to consumers; case-list
flattening takes a `tcl_syntax::case_list::CaseListShape`.

The public differential tests compare CST-derived segments with the frozen
token-loop oracle and lexer command geometry, including the installed Tcl
corpora. Run `cargo test -p tcl-source`. Ignored random and exhaustive campaigns
remain manual.
