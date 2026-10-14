proc explicit {name} {uplevel 1 [list ::set $name EXPLICIT]}
proc caller {} {
    foreach name [list {} {$name} {a b} {{literal}}] {
        set $name BEFORE
        explicit $name
        puts [list explicit $name [set $name]]
        proc omitted [list [list name $name]] {upvar 1 $name local; set local DEFAULT}
        omitted
        puts [list default $name [set $name]]
        unset $name
    }
}
caller
