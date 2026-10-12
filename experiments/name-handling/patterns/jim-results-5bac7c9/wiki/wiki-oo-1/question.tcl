
        oo::class create C {
            method hidden {x} {return $x}
            unexport hidden
            method prefix {} {namespace code [list my hidden fixed]}
        }
        set obj [C new]
        set prefix [$obj prefix]
        list [catch {$obj hidden fixed}] [eval $prefix]
    
