
        namespace eval n {set x old; set ::prefix [namespace code {set x}]}
        namespace delete n
        namespace eval n {set x recreated}
        eval $prefix
    
