
        proc take {fixed value} {list $fixed $value}
        set prefix [list take {x; error injected}]
        eval [linsert $prefix end {a b}]
    
