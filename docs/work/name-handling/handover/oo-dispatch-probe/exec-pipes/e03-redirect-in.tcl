# < file, << value, <@ channel
set f [open in.txt w]; puts $f "from file"; close $f
puts "lt=[exec cat < in.txt]"
puts "lt-joined=[exec cat <in.txt]"
puts "heredoc=[exec cat << {from value}]"
puts "heredoc-joined=[exec cat <<joined]"
set ch [open in.txt r]
puts "lt-at=[exec cat <@ $ch]"
close $ch
file delete in.txt
