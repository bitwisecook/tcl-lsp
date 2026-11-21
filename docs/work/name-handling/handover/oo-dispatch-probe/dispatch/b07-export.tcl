oo::class create C {
    method _hidden {} { return h }
    export _hidden
    method shown {} { return s }
    unexport shown
}
set o [C new]
puts [$o _hidden]
puts [catch {$o shown} m]:$m ;# BAD unexported
