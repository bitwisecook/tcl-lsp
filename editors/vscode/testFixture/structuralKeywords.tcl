if 1 {
 puts a
} elseif 2 {
 puts b
} else {
 puts c
}
try {
 set x 1
} on error {e} {
 puts $e
} finally {
 puts d
}
dict set frame proc "asasdas asd"
