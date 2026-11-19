cd /tmp/bd-fs-check
foreach opts [list {-tails -path al *} {-path al *} {-tails -dir . .} {-dir . alpha} {-dir . /alpha} {-path ./al *} {-path /tmp/bd-fs-check/al *} {-tails -path /tmp/bd-fs-check/al *} {-nocomplain -- ./alpha} {-nocomplain -- dir//inner} {-nocomplain -- alpha/} {-nocomplain -- dir/} {-types hidden -- .} {-types hidden -- alpha} {-dir /tmp/bd-fs-check -- {}}] {
 set c [catch [linsert $opts 0 glob] r]; puts [list $opts $c $r]
}
