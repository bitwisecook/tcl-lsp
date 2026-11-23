# the two glob matchers: string match / lsearch -glob versus switch -glob / info / array names, over non-BMP and case-folded text
set e "😀"
proc sw {p s} { switch -glob -- $s [list $p {return 1} default {return 0}] }
proc swn {p s} { switch -nocase -glob -- $s [list $p {return 1} default {return 0}] }
foreach {p s} [list ? $e ?? $e {[😀-😀]} $e *😀* x${e}y {[a-z]} \xe9 \xe9 \xc9 ] {
    puts [list [string length $s] [string match $p $s] [sw $p $s] [lsearch -glob [list $s] $p] [string match -nocase $p $s] [swn $p $s]]
}
array set a [list $e 1 b 2]
puts [lsort [array names a ?]]
puts [llength [info vars ?]]
set ${e} 1
puts [llength [info vars ?]]
