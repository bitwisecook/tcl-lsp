# encoding convertfrom utf-8 over ill-formed byte sequences, default profile
foreach {label bytes} [list overlong \xc0\x80 ff \xff trunc a\xe2\x82 surr \xed\xa0\x80 cesu \xed\xa0\xbd\xed\xb8\x80] {
    set rc [catch {encoding convertfrom utf-8 $bytes} r]
    if {$rc} { puts "$label: error: $r"; continue }
    set cps {}
    foreach c [split $r ""] { lappend cps [format %04X [scan $c %c]] }
    puts "$label: [string length $r] $cps"
}
