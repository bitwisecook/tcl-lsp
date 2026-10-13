puts [list provider [info patchlevel]]
set names [list puts ::puts Puts PUTS { puts} {puts } { puts } "\tputs" "puts\n" { }]
set ordinal 0
foreach name $names {
    puts [list before $ordinal [namespace which -command $name]]
    incr ordinal
}
set ordinal 2
foreach name [lrange $names 2 end] {
    proc $name {} [list return $ordinal]
    incr ordinal
}
set ordinal 0
foreach name $names {
    puts [list after $ordinal [namespace which -command $name]]
    if {$ordinal >= 2} {
        set code [catch [list $name] value]
        puts [list invoke $ordinal $code $value]
    }
    incr ordinal
}
