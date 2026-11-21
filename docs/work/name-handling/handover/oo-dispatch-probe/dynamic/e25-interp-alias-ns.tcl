namespace eval ::lib { proc helper {} { return h } }
interp alias {} ::app::h {} ::lib::helper
namespace eval ::app {}
puts [::app::h]
