# '#' as a list element and as a proc argument
proc h {#} {return [set #]}
puts [h x]
puts [lindex {# a} 0]
