cd /tmp/bd-fs-check
set lb [format %c 123]; set rb [format %c 125]
foreach pattern [list "\${lb}alpha,beta\${rb}" "\${lb}alpha,alpha\${rb}" "${lb}alpha,beta${rb}" "${lb}alpha,alpha${rb}" "${lb}${rb}" "${lb}alpha,${rb}" "${lb}dir/inner,beta${rb}" "nope/${lb}" "nope*/${lb}" "dir/${lb}" "d*/${lb}" "${lb}" "${rb}" {a} {a\lp*} {\.dot} {\.*} {dir\/inner} {*}] {
 set c [catch {glob -nocomplain -- $pattern} r]; if {$c} {puts [list $pattern $c $r $::errorCode]} else {puts [list $pattern $c [lsort $r]]}
}
