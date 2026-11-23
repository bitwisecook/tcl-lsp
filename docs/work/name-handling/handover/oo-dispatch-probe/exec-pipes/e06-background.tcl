# & runs in the background and returns the process ids.
set pids [exec echo bg > bg.txt &]
puts "count=[llength $pids] int=[string is integer -strict [lindex $pids 0]]"
after 300
puts "out=[exec cat bg.txt]"
set two [exec echo x | cat > bg2.txt &]
puts "pipeline-pids=[llength $two]"
after 300
file delete bg.txt bg2.txt
puts "amp-not-last=[catch {exec echo a & echo b} m] $m"
