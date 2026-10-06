set code [catch {proc {dict update} {args} {llength $args}; dict update d k x {}} result options]
puts $code
puts -nonewline $result
