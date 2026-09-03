# T1: switch with literal patterns -> should become a chained compare / br_table.
foreach v {apple banana cherry durian} {
    switch -- $v {
        apple -
        cherry { puts "$v: red" }
        banana { puts "$v: yellow" }
        default { puts "$v: unknown" }
    }
}
switch -glob -- "file.tcl" {
    *.tcl { puts tcl }
    *.c { puts c }
}
