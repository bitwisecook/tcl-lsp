# ASCII source. Every literal backslash in this file is intentional.
# Question: which source escape bytes do bare, quoted and braced original
# Tcl words produce, independently of format command grammar or handlers?
puts "VERSION|[info patchlevel]"
proc r2286_escape_record {label script} {
    set status [catch {uplevel 1 $script} result]
    set resultHex {}
    binary scan $result H* resultHex
    if {$status == 0} {set code {}} else {set code $::errorCode}
    puts "ROW|$label|$status|[string length $result]|$resultHex|$code"
}
r2286_escape_record bare-x25b {set value \x25b}
r2286_escape_record quoted-x25b {set value "\x25b"}
r2286_escape_record braced-x25b {set value {\x25b}}
r2286_escape_record bare-x25-ub {set value \x25\u0062}
r2286_escape_record quoted-x25-ub {set value "\x25\u0062"}
r2286_escape_record braced-x25-ub {set value {\x25\u0062}}
r2286_escape_record bare-x4142 {set value \x4142}
r2286_escape_record quoted-x4142 {set value "\x4142"}
r2286_escape_record bare-x25z {set value \x25z}
r2286_escape_record bare-x25-upper {set value \x25B}
r2286_escape_record bare-x0 {set value \x0}
r2286_escape_record bare-cu {set value c\u0075}
r2286_escape_record braced-cu {set value {c\u0075}}
