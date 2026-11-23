# TIP 278 ("creative writing"): unqualified var in namespace eval no longer finds global
set ::x global
namespace eval ns { set x local }
puts "::x=$::x ns::x=[expr {[info exists ns::x] ? $ns::x : {<none>}}]"
