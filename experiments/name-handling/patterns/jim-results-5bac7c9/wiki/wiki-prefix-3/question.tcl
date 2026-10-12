
        namespace eval n {
            proc list args {return {other}}
            proc take args {return intended}
            proc other args {return actual}
            eval [list take {a b}]
        }
    
