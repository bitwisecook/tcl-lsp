catch {fconfigure stdout -buffering none}
set code [catch {info patchlevel} value]
puts [list version $code $value]
set code [catch {switch -exact -- a {a {{return one}} b {return two}}} value]
set detail {}
if {$code == 1 && [info exists errorCode]} {set detail $errorCode}
puts [list double-braced-body code $code result $value errorCode $detail]
set code [catch {switch -exact -- a {a {return one} b {return two}}} value]
set detail {}
if {$code == 1 && [info exists errorCode]} {set detail $errorCode}
puts [list builtin-return-body code $code result $value errorCode $detail]
set body {{return one}}
puts [list decoded-body $body body-list-length [llength $body] first-command-word [lindex $body 0]]
