
        proc take args {return global}
        namespace eval n {
            proc take args {return [linsert $args 0 local]}
            set ::prefix [namespace code [list take fixed]]
        }
        eval [linsert $prefix end {a b}]
    
