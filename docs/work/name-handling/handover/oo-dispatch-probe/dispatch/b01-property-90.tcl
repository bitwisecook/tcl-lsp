oo::configurable create C {
    property size
}
set o [C new]
$o configure -size 4
puts [$o configure -size]
puts [catch {$o configure -colour 1} m]:$m ;# BAD unknown property
