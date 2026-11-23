# An uncaught custom code at the top level of a script file: message and exit status.
proc p {} { return -code 5 five }
puts before
p
puts after
