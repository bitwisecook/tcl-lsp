# A return -level 2 from a procedure the script file calls settles at the
# file as an error from 8.5; 8.4 has no -level.
proc p {} {return -code error -level 2 deep}
puts a
p
puts b
