# auto_qualify: the candidate names unknown tries for a command in a namespace.
puts [auto_qualify foo ::]
puts [auto_qualify foo ::a::b]
puts [auto_qualify ::foo ::a::b]
puts [auto_qualify x::foo ::a]
