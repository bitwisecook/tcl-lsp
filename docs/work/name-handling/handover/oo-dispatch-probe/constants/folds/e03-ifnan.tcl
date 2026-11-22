proc f {} { return "[catch {if {NaN} {puts a} else {puts b}} m] $m" }
puts [f]
