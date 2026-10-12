
        namespace eval n {proc take x {return $x}; set ::prefix [list [namespace code take] fixed]}
        catch {eval $prefix}
    
