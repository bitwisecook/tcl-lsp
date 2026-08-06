oo::class create Idx79Vector {
    variable _x _y
    constructor {args} {
        if {[llength $args] == 1} {
            set other [lindex $args 0]
            set _x [$other X]
        } else {
            lassign $args _x _y
        }
    }
    method X {} { return $_x }
    method Get {} { return "$_x $_y" }
    export X Get
}
