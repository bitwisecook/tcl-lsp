# Publication without a namespace holder

The [original program](probe.tcl) catches `proc missing::audit` and reports its
completion, result and `namespace which -command` without using `namespace exists`.
C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all return completion1 with the
exact unknown-namespace error and no command. The identified Jim build returns
completion0, result `missing::audit` and selected name `::missing::audit`.
Every process exits zero with empty stderr.

[results.json](results.json) retains executable/input hashes, original argv,
environment selection and raw transcript paths. It ran from the recorded `/tmp`
script; the retained source has identical bytes. This is a separate reached
question from the [orchestration](../evaluated-argv-oracles/results.json) that
cannot complete on Jim because `namespace exists` is unavailable. Both records
keep their actual outcomes. No Tcltest harness or F5 execution is used.
