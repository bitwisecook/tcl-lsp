
        oo::class create C {
            method hidden {} {return internal}
            unexport hidden
            forward send my hidden
        }
        set obj [C new]
        list [catch {$obj hidden}] [$obj send]
    
