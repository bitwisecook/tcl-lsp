proc f {} { return "[catch {if {08} {puts a} else {puts b}} m] $m" }
puts [f]
