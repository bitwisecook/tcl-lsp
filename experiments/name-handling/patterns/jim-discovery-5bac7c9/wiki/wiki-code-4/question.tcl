
        namespace eval n {set ::prefix [namespace code {set x}]}
        namespace delete n
        catch {eval $prefix}
    
