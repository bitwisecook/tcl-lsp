# Original expression variable index evaluation

C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and latest Jim execute the retained probe with matching results. Braced combined names preserve literal dollar and command-looking index bytes. Separate indices evaluate variable or command substitutions, including nested array indices. Quoted expression operands substitute; braced operands preserve their data. The stdout and stderr files record actual provider results.

`result.json` records exact executables, their unchanged before/after SHA256, commands and exit codes. Each C provider used its matching original `init.tcl`, retained beside its stdout. No tcltest was used. The capture script is retained. These observations establish provider behaviour; they do not establish compiler acceptance or F5 behaviour.

The companion original-expression-variable-syntax corpus distinguishes `${a(k)}` lexical index geometry from its combined array lookup. `bigip-questions.md` contains unanswered provider questions.
