# Tcl 9 encoding profiles and -failindex
if {[catch {encoding profiles} p]} { puts "profiles: error: $p"; return }
puts "profiles: [lsort $p]"
foreach prof {strict tcl8 replace} {
    set rc [catch {encoding convertfrom -profile $prof utf-8 "a\xffb"} r]
    set cps {}
    if {!$rc} { foreach c [split $r ""] { lappend cps [format %04X [scan $c %c]] } } else { set cps $r }
    puts "from $prof: $rc $cps"
    set rc [catch {encoding convertto -profile $prof ascii "a\u00e9b"} r]
    if {!$rc} { binary scan $r H* r }
    puts "to $prof: $rc $r"
}
set r [encoding convertfrom -failindex fi utf-8 "ab\xffcd"]
puts "failindex from: $fi [string length $r]"
set r [encoding convertto -failindex fi ascii "ab\u00e9cd"]
puts "failindex to: $fi [string length $r]"
set r [encoding convertfrom -failindex fi utf-8 "abcd"]
puts "failindex none: $fi"
puts "bad profile: [catch {encoding convertfrom -profile bogus utf-8 a} m] $m"
