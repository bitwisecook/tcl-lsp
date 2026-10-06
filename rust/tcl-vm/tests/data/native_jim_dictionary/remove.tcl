set code [catch {dict remove {a 1 b 2} a} result options]
puts $code
puts -nonewline $result
