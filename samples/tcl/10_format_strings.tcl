# 10: Format string highlighting, hover tooltips, and inlay hints
#
# Hover over any format specifier to see a decoded description table.
# Inlay hints appear inline next to each specifier (e.g., %d → int).

# sprintf-style format strings

set msg [format "Hello %s, you are %d years old (score: %.2f%%)" $name $age $score]
set hex [format "0x%08X" $value]
set padded [format "%-20s %5d" $label $count]
set sci [format "%12.4e" $measurement]
set multi [format {%2$s has %1$d items} $n $who]

# scan (inverse of format)
scan $input "%d %f %s" count weight label
scan $hex "%x" decimal_value

# clock format strings

set now [clock seconds]
set date [clock format $now -format "%Y-%m-%d"]
set time [clock format $now -format "%H:%M:%S"]
set full [clock format $now -format "%A, %B %e %Y at %I:%M %p"]
set iso  [clock format $now -format "%Y-%m-%dT%H:%M:%S%z"]
set week [clock format $now -format "Week %V of %G (day %u)"]

# binary format/scan

set packed [binary format c4s2i $a $b $c $d $e $f $g]
set msg [binary format a10A20 $short_str $padded_str]
set float_data [binary format f3d2 $x $y $z $w1 $w2]
set network [binary format Iua* $len $flags $payload]

binary scan $data cu4 b1 b2 b3 b4
binary scan $packet S2I len1 len2 payload_len
binary scan $raw a16H* header hex_rest

# regsub substitution specs

regsub {(\w+)\s+(\w+)} $str {\2, \1} swapped
regsub -all {<[^>]+>} $html {} stripped
regsub {^(\d{3})-(\d{4})$} $phone {(\1) \2} formatted
regsub -nocase {(error|warn|info)} $line {[\1]} tagged

# regexp patterns (full BRE/ARE sub-tokenization)

regexp {^[A-Z][a-z]+$} $word
regexp {^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$} $ip
regexp -nocase {^https?://([^/]+)} $url -> host
regexp {(\w+)=([^&]*)} $query -> key value
regexp {(?i)^(GET|POST|PUT|DELETE)\s+(/\S*)} $request -> method path

# glob patterns

string match {*.tcl} $filename
string match {test_*} $name
string match -nocase {readme*} $file
string match {[abc]*.log} $logfile
glob -directory /tmp {*.log}
glob -types f {src/**/*.tcl}
lsearch -glob $files {*.bak}
