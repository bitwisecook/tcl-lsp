oo::class create Bar957Ctrl {
    method getOptions {key} { return $key }
    method get {key} {
        if {1} {
            my getOptions $key
        }
    }
    method get2 {key} {
        foreach k $key {
            my getOptions $k
        }
    }
    method get3 {key} {
        switch -- $key {
            default {
                my getOptions $key
            }
        }
    }
}
set b [Bar957Ctrl new]
