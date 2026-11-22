package unknown [list apply {{name version args} { package ifneeded $name 3.0 [list package provide $name 3.0] }}]
puts [package require virtual]
