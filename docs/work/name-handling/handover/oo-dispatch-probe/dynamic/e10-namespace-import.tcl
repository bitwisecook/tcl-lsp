namespace eval ::lib { namespace export helper; proc helper {} { return h } }
namespace import ::lib::helper
puts [helper]
namespace forget ::lib::helper
puts [catch helper m]:$m ;# BAD forgotten
