oo::class create C {
    variable v
    method go {} { set v 1; return [list [my varname v] [my eval {set v}]] }
}
set o [C new]
puts [$o go]
puts [catch {$o eval {set v}} m]:$m ;# BAD eval is unexported
