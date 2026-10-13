set ::fixture [lindex $argv 0]
puts [list PROVIDER [info patchlevel]]
proc emit {id types} {
    set ::errorCode {NH UNSET}
    set command [list glob -nocomplain -tails -directory $::fixture -types $types -- *]
    set code [catch $command result]
    if {$code == 0} {set result [lsort $result]}
    puts [list ROW $id $code $result $::errorCode]
}
emit empty {}
emit file f
emit directory d
emit file-directory {f d}
emit directory-file {d f}
emit duplicated-union {f d f}
emit union-with-unrepresented {b c f p s}
emit directory-with-unrepresented {b c d p s}
emit unrepresented-only {b c p s}
emit readable r
emit readable-writable {r w}
emit executable x
emit file-readable {f r}
emit file-executable {f x}
emit union-executable {f d x}
emit union-readable-writable {f d r w}
emit file-readonly {f readonly}
emit union-readonly {f d readonly}
emit bad-single z
emit bad-two ff
emit malformed-list \{
