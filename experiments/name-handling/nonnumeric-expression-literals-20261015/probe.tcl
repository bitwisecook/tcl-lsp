puts [list PATCHLEVEL [info patchlevel]]
foreach lhs {GET POST 01 1 {GET }} {
    set numeric [expr {$lhs == "GET"}]
    set textual [expr {$lhs eq "GET"}]
    switch -exact -- $lhs {GET {set exact 1} default {set exact 0}}
    puts [list NONNUMERIC $lhs NUMERIC_OPERATOR $numeric TEXTUAL $textual EXACT $exact]
}
foreach rhs {1 01 { 1 } 1e0} {
    set lhs 1
    set numeric [expr {$lhs == $rhs}]
    set textual [expr {$lhs eq $rhs}]
    puts [list NUMERIC_LITERAL $rhs NUMERIC_OPERATOR $numeric TEXTUAL $textual]
}
