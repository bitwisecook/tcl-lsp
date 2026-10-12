
        set auto_index(::late) {proc ::late {} {return loaded}}
        list [info commands ::late] [::late] [info commands ::late]
    
