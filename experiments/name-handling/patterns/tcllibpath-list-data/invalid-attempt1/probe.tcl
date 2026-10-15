foreach raw [list {{dir with space} {{brace}} {"quoted"} {} plain\ path { trailing }} {\U00010000} {{unfinished}] {
    set code [catch {llength $raw} result]
    puts [list LIST $raw $code $result]
    if {!$code} {
        foreach element $raw {
            puts [list ELEMENT [string length $element] $element]
        }
    }
}
