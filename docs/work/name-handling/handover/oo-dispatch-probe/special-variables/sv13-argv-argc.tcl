# argv and argc are ordinary variables after startup.
set argv {a b c}
puts "argc=$argc llength=[llength $argv]"
set argc 9
puts "argc=$argc"
