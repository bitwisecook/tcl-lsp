proc capture {label script} {
    set code [catch {uplevel #0 $script} result]
    puts [list ROW $label $code $result]
}
proc escape {value} {return [string map [list "\\" "\\\\" "*" "\\*" "?" "\\?" "\[" "\\\[" "\]" "\\\]"] $value]}
capture patchlevel {::info patchlevel}
capture version {::info version}
capture jim-version {::set ::jim_version}
capture tcl-version {::set ::tcl_version}
foreach ns {::N[0] ::N0 ::W* ::WZ ::Q? ::Qa {::B\slash}} {
    capture [list setup $ns] [list ::namespace eval $ns {}]
    foreach tail {p p* pZ p? pA p[0] p0 {p\tail} :p} {
        set name ${ns}::$tail
        capture [list define $name] [list ::proc $name {} [list ::return $name]]
        capture [list raw $name] [list ::info commands $name]
        capture [list whole $name] [list ::info commands [escape $name]]
        capture [list tail $name] [list ::info commands ${ns}::[escape $tail]]
        capture [list which $name] [list ::namespace which -command $name]
        capture [list call $name] [list $name]
    }
}
foreach name {:p p* p? p[0] {p\tail} :::p ::p} {
    capture [list define-root $name] [list ::proc $name {} [list ::return $name]]
    capture [list raw-root $name] [list ::info commands $name]
    capture [list whole-root $name] [list ::info commands [escape $name]]
    capture [list which-root $name] [list ::namespace which -command $name]
    capture [list call-root $name] [list $name]
}
