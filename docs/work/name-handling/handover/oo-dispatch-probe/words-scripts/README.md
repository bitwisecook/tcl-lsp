# Script, word, list and string fixtures

`s*.tcl` run under tclsh, the VM and the runtime (`engines.out`; the runtime's `error:` lines differ only in formatting) and through the analyser (`analyser.out`). `s05-escapes.tcl` contains a raw UTF-8 `é`, so it also tests source-file decoding (Tcl 8.6 reads files in the system encoding). `vm-literal-expansion.tcl` reproduces the VM's failure on `{*}` applied to a literal braced word: run it as a file (silent exit 1) and with `tclvm -c` (prints the error).
