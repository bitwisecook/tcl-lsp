# Issue #923 differential-audit finding idx 79 — an untracked receiver.
oo::class create Vector3d {
    variable _x _y
    constructor {args} {
        if {[llength $args] == 1} {
            set other [lindex $args 0]
            if {[info object isa typeof $other ::Vector3d]} {
                set _x [$other X]
                set _y [$other Y]
            }
        } else {
            lassign $args _x _y
        }
    }
    method X {} { return $_x }
    method Y {} { return $_y }
    method Get {} { return "$_x $_y" }
    export X Y Get
}
set v1 [Vector3d new 7 9]
set v2 [Vector3d new $v1]
puts "[$v1 Get] [$v2 Get]"
