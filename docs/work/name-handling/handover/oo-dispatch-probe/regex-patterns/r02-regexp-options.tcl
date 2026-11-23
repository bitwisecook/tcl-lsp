# regexp options
proc t {label args} { if {[catch {uplevel 1 $args} r]} { puts "$label: error: $r" } else { puts "$label: $r" } }
t all-count regexp -all {a} "banana"
t all-inline regexp -all -inline {a(n)?} "banana"
t indices regexp -indices -inline {(b)(x)?} "abc"
t all-indices regexp -all -indices -inline {an} "banana"
t start regexp -inline -start 2 {a.} "banana"
t start-caret regexp -inline -start 2 {^n} "banana"
t start-neg regexp -inline -start -5 {b} "banana"
t start-past regexp -start 99 {a} "banana"
t line-dot regexp -inline {a.b} "a\nb"
t line-dot-on regexp -line -inline {a.b} "a\nb"
t lineanchor regexp -lineanchor -all -inline {^x} "x\nx"
t linestop regexp -linestop -inline {a[^z]b} "a\nb"
t expanded regexp -expanded -inline { a  b # comment
 } "ab"
t about regexp -about {(a)(b)\1}
t nocase regexp -nocase -inline {ABC} "xabcx"
t zero-len-all regexp -all -inline {x*} "abc"
t zero-len-count regexp -all {x*} "abc"
t dash-dash regexp -- {-a} "-a"
t bad-opt regexp -foo a a
t all-inline-groups regexp -all -inline {(\w)(\d)} "a1b2"
