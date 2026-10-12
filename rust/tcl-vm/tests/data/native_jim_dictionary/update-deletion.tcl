set code [catch {rename {dict update} {}; set d {k 1}; dict update d k x {}} result options]
puts $code
puts -nonewline $result
