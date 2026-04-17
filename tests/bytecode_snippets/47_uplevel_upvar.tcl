proc setvar {name value} {
    upvar 1 $name var
    set var $value
}
proc getvar {name} {
    upvar 1 $name var
    return $var
}
