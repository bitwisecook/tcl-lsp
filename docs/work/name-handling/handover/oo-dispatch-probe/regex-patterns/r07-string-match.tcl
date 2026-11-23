# string match patterns
proc t {p s} { puts [format "%-10s %-8s %s" $p $s [string match $p $s]] }
t {a*}     abc
t {a?c}    abc
t {[a-c]x} bx
t {[c-a]x} bx
t {[!a]x}  bx
t {[^a]x}  bx
t {\*}     *
t a\\      a\\
t \[a     \[a
t {[a]]}   {a]}
t {[]a]}   {]}
t {{a,b}}  a
t {{a,b}}  {{a,b}}
t {[-a]}   -
t {[a-]}   -
t {*}      {}
t {**a}    a
t {[\]]}   {]}
t {\a}     a
puts [string match -nocase {A[B-C]} ab]
puts [string match {[A-z]} _]
puts [string match "\[\xe9\]" \xe9]
