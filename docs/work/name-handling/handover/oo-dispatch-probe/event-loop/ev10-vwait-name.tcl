# Which variable vwait waits on inside namespace eval: the after scripts set ::v first and ::ns::v later.
namespace eval ns {
    after 0 {set ::v global; lappend ::order global}
    after 50 {set ::ns::v ns; lappend ::order ns}
    after 100 {set ::stop 1}
    vwait v
    lappend ::order woke
}
vwait stop
puts $order
