# Actual all.tcl orchestration question

Can unchanged real C8.6 tests/all.tcl orchestrate a single full var.test file with the real matching init and Tcltest package? Both providers use `-singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}`. Selecting a complete file and excluding unrelated test directories bounds this naming question.

Exact original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/all.tcl`, SHA-256 `84d710d37d8c7cd2070062ddd89254aaf25f0d9f827e11c27995575829da825c`; [compressed original](../originals/tests/all.tcl.gz). [Measured result](../results/all-tcl.md).
