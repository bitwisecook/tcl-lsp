# info coroutine outside, inside, and after the creating name is namespace-relative.
proc body {} { yield [info coroutine]; return [info coroutine] }
puts "top=[list [info coroutine]]"
puts [coroutine c1 body]
namespace eval ns { puts [coroutine c2 ::body] }
puts [info commands ::ns::c2]
puts [ns::c2]
puts [c1]
