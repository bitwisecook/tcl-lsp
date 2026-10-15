proc hex {value} {
    binary scan $value H* result
    return $result
}
proc observe {label script} {
    set code [catch {uplevel 1 $script} result]
    puts "RESULT|$label|$code|[hex $result]"
}
set dir [lindex $argv 0]
puts "PROVIDER|[info patchlevel]"
foreach {label tail} {
    spaces { spaced.tcl }
    quotes {"quoted".tcl}
    sigil {cash$name.tcl}
    dash {-literal.tcl}
} {
    set loader [list source [file join $dir $tail]]
    puts "LOADER|$label|[hex $loader]"
    observe $label $loader
}
set loader [list source [file join $dir {} empty.tcl]]
puts "LOADER|empty_component|[hex $loader]"
observe empty_component $loader
set loader [list source [file join $dir {a b} {part".tcl}]]
puts "LOADER|nested_quotes|[hex $loader]"
observe nested_quotes $loader
set loader [list source [file join $dir { spaced.tcl }]]
observe encoding [linsert $loader 1 -encoding utf-8]
observe nopkg [linsert $loader 1 -nopkg]
observe conflicting_options [linsert $loader 1 -encoding utf-8 -nopkg]
observe too_many [concat $loader EXTRA]
set loader [list source [file join $dir {}]]
observe only_empty_component $loader
