# switch -regexp/-glob/-exact with -matchvar and -indexvar
foreach s {abc a1b2 xyz {a*c}} {
    switch -regexp -matchvar mv -indexvar iv -- $s {
        {^(\w)(\d)} { puts "$s: digit $mv $iv" }
        {^a(.)c$} { puts "$s: abc $mv $iv" }
        default { puts "$s: default [info exists mv] [list $mv]" }
    }
}
foreach s {abc a*c {[x]} a} {
    switch -glob -- $s {
        {a\*c} { puts "$s: literal star" }
        {a*c} { puts "$s: glob" }
        {\[x\]} { puts "$s: brackets" }
        {[ab]} { puts "$s: set" }
    }
}
puts [switch -exact -- a* { a* {format exact} default {format no} }]
puts [switch -nocase -glob ABC { a* {format nocase} default {format no} }]
puts [catch {switch -glob -matchvar x a {a {}}} m]$m
puts [switch -regexp -- abc {B {format b} (?i)B {format ib}}]
