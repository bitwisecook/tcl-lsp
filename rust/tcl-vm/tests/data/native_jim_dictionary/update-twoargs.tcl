set code [catch {proc {dict update} {args} {llength $args}; dict update d {}} result options]
puts $code
puts -nonewline $result
