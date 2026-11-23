# stdin read (run with "hi" piped in) and a read channel opened before a later write (snapshot vs live file)
puts "stdin: [gets stdin]"
set w [open c23.txt w]; puts $w one; flush $w
set r [open c23.txt r]
puts $w two; flush $w
puts "live: [list [gets $r] [gets $r]]"
close $r; close $w
