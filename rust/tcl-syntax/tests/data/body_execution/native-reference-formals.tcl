proc attempt {params body} {
    set made [catch [list proc p $params $body] result]
    set target BEFORE
    if {$made == 0} {set code [catch {p target} result]} else {set code -}
    puts [list $params $made $code $result $target]
}
attempt {&x} {set x AFTER}
attempt {&a(k)} {set a(k) AFTER}
attempt {&::alias} {set ::alias AFTER}
attempt {&n::alias} {set n::alias AFTER}
proc p {&x} {set x AFTER}
proc arraycaller {} {set target(k) BEFORE; set code [catch {p target(k)} result]; list $code $result $target(k)}
puts [list elementtarget [arraycaller]]
proc p {&x} {}
proc emptycaller {} {list [catch {p missing} result] $result}
puts [list emptybody [emptycaller]]
proc p {{&x DEFAULT}} {list [info locals] [set &x]}
puts [list default [p]]
