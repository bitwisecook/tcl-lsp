proc helper {args} {return global:$args}
namespace eval ::a { proc helper {args} {return a:$args}; interp alias {} ::a::al {} helper x }
puts [::a::al y]
namespace eval ::a { puts [al z] }
