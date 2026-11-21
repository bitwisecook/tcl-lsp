proc helper {} { return global }
namespace eval ::app { proc run {} { return [helper] } }
puts [::app::run]
namespace eval ::app { proc helper {} { return local } }
puts [::app::run]
