
        namespace eval one {variable x one; proc take {} {variable x; return $x}}
        namespace eval two {variable x two}
        rename one::take two::take
        two::take
    
