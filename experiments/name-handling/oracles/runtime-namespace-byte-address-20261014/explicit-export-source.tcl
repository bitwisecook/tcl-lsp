namespace eval ::outer {oo::object create src; oo::copy src dst ns::::child}
puts [list copied [info object namespace ::outer::dst]]
set c [catch {namespace eval ::outer {oo::copy src another ns::::child}} m]
puts [list existing $c $m]
namespace eval ::outer {oo::class create C; oo::objdefine C export createWithNamespace; C createWithNamespace named allocated::::child}
puts [list created [info object namespace ::outer::named]]
set c [catch {namespace eval ::outer {C createWithNamespace another allocated::::child}} m]
puts [list duplicate $c $m]
