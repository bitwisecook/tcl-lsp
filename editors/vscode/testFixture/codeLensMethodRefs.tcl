oo::class create Bar {
    variable _options
    constructor {args} {
        set _options $args
    }

    method get {key} {
        return [dict get $_options $key]
    }

    method unused {} {
        return {}
    }
}
set b [Bar new]
puts [$b get foo]
