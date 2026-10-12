
        oo::class create C {method .send {} {return old}; export .send}
        set obj [C new]
        set before [$obj .send]
        oo::define C method .send {} {return replacement}
        list $before [catch {$obj .send}]
    
