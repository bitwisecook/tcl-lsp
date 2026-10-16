proc physical {label value} {
    if {[llength [info commands ::tcl::unsupported::representation]]} {
        puts [list $label [::tcl::unsupported::representation $value]]
    }
}
proc catch_site {body} {catch $body result; set result}
proc for_site {body} {for $body 0 {} {}; set written}
set payload VALUE
foreach site {catch_site for_site} {
    set body [list set written $payload]
    physical BEFORE $body
    set code [catch {$site $body} result]
    puts [list RESULT $site $code $result]
    physical AFTER $body
}
if {[llength [info commands ::tcl::unsupported::disassemble]]} {
    puts [::tcl::unsupported::disassemble proc catch_site]
    puts [::tcl::unsupported::disassemble proc for_site]
}
