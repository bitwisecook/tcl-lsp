# 8.6: a non-BMP glob pattern that came out of a list element (foreach over [list ...]) versus one built by substitution
set e "😀"
foreach {p s} [list *😀* x${e}y] { puts [lsearch -glob [list $s] $p] }
foreach {p s} [list *😀* x${e}y] { puts [string match $p $s] }
foreach {p s} [list *😀* x${e}y] { puts [switch -glob -- $s $p {format 1} default {format 0}] }
set p *😀*
puts [lsearch -glob [list x${e}y] $p]
puts [lsearch -glob [list x${e}y] *$e*]
