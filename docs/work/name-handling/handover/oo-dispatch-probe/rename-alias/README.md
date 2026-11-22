# Rename and alias fixtures

`r*.tcl` are runtime-semantics fixtures (`engines.out`: tclsh against the VM and runtime). `editor-rename/before` holds scripts in which the LSP was asked to rename `helper` to `zz` (or `p` to `x` for `v2`) at its definition; `editor-rename/after` holds the result of applying the returned edits (unchanged when refused); `edits.out` lists each file's edits. `nav/` holds navigation fixtures (`;# CALL word`); in `analyser.out` rows with `col=0` had the cursor off the word, so only their diagnostics count.
