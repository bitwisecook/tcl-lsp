namespace eval ::lib { proc h {} {return lib} }
namespace eval ::app { namespace eval inner { proc h {} {return inner} } ; namespace path {inner ::lib}; puts [namespace path]; proc r {} {h}; puts [r] }
namespace delete ::app::inner
puts [namespace eval ::app {namespace path}]:[::app::r]
