
        oo::class create Meta {superclass oo::class; method new args {return ordinary}}
        Meta create C
        set value [C new]
        list $value [info object isa object $value]
    
