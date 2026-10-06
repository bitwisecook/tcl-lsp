namespace eval N {
 set code [catch {proc :f {} {return [namespace current]}} result]
 puts [list TAIL-CREATE $code $result]
 if {$code == 0} {puts [list TAIL-HOME [:f]]; puts [list TAIL-FULL [namespace which -command :f]]}
}
namespace eval a: {namespace eval b {proc p {} {return [list FIRST [namespace current]]}}}
namespace eval a {namespace eval :b {proc p {} {return [list SECOND [namespace current]]}}}
foreach outer {a: a} inner {b :b} {
 puts [list PATH $outer $inner [namespace eval $outer [list namespace eval $inner {
  set code [catch {p} result]
  list [namespace current] $code $result
 }]]]
}
puts [list ABSOLUTE [catch {::a:::b::p} result] $result]
