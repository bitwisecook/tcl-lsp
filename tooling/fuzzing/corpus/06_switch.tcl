# Switch statement
set val "beta"
switch -- $val {
    alpha {
        set result 1
    }
    beta {
        set result 2
    }
    gamma {
        set result 3
    }
    default {
        set result 0
    }
}

# Switch with return value
set x [switch -- "b" {
    a { expr {1} }
    b { expr {2} }
    c { expr {3} }
}]
