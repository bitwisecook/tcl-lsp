namespace eval ::outer {oo::object create src}
namespace eval ::outer {oo::copy src dst ns::::child}
puts [list copied [info object namespace ::outer::dst] [namespace exists ::outer::ns::child]]
set c [catch {namespace eval ::outer {oo::copy src another ns::::child}} m]
puts [list existing $c $m]
namespace eval ::outer {oo::class create C}
set c [catch {namespace eval ::outer {C createWithNamespace named allocated::::child}} m]
puts [list created $c $m]
if {!$c} {puts [list namespace [info object namespace ::outer::named]]}
