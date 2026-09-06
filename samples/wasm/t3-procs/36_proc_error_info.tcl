# T3: an error out of a proc body carries the same errorInfo, errorCode and
# TIP 348 error stack whether the body ran compiled or interpreted - the
# `while executing` frame and the body-relative line come from the compiled
# statement's own site (issue #1774).
#
# The definitions come first so each one's `proc` statement is still dispatch
# proven and binds its compiled body; a `catch` before them would widen the
# world and leave the rest source-only.
proc boom {a} {
    set b 1
    error "bad $a"
}
proc r5 {} { return 5 }
proc rerr {} { return -code error -errorcode {MY CODE} nope }

puts [catch {boom Q} msg opts]
puts $msg
puts $::errorInfo
puts [dict get $opts -errorcode]
# Only the CALL entry: this issue's fix is that a compiled body's error gets
# an inner context at all, so `run_proc` will chain the proc call onto the
# stack. The INNER payload itself is a separate, pre-existing divergence -
# this runtime records the failing command where C records `returnImm`.
puts [lrange [dict get $opts -errorstack] end-1 end]
puts [r5]
puts [catch {rerr} m o]
puts "$m [dict get $o -errorcode]"
