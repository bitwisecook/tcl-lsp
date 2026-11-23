# encoding convertto/convertfrom round trips and hex bytes
proc hex {b} { binary scan $b H* h; return $h }
foreach enc {utf-8 iso8859-1 cp1252 ascii} {
    if {[catch {encoding convertto $enc "A\u00e9"} r]} {
        puts "$enc to: error: $r"
    } else {
        puts "$enc to: [hex $r]"
    }
}
puts "from utf-8: [encoding convertfrom utf-8 \xc3\xa9] len=[string length [encoding convertfrom utf-8 \xc3\xa9]]"
puts "from iso8859-1: [string length [encoding convertfrom iso8859-1 \xe9]]"
puts "identity names: [expr {[lsearch -exact [encoding names] identity] >= 0}]"
puts "binary names: [expr {[lsearch -exact [encoding names] binary] >= 0}]"
puts "utf-16 names: [expr {[lsearch -exact [encoding names] utf-16] >= 0}]"
puts "unicode names: [expr {[lsearch -exact [encoding names] unicode] >= 0}]"
puts "utf-32 names: [expr {[lsearch -exact [encoding names] utf-32] >= 0}]"
puts "system: [encoding system]"
