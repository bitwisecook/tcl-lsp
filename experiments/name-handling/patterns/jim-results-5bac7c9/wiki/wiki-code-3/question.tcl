
        namespace eval n {proc take {} {return old}; set ::prefix [namespace code take]}
        rename n::take n::saved
        proc n::take {} {return replacement}
        eval $prefix
    
