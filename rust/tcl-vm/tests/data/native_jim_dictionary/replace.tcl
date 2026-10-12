set code [catch {dict replace {a 1} a 2 b 3} result options]
puts $code
puts -nonewline $result
