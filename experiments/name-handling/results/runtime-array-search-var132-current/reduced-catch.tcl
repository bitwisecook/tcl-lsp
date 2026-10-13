set code [catch {
    apply {{} {
	array set a {aa 11 bb 22 cc 33 dd 44 ee 55 ff 66}
	set s [array startsearch a]
	unset a([array nextelement a $s])
	array nextelement a $s
    }}
} result options]
puts [list $code $result [dict get $options -errorcode]]
