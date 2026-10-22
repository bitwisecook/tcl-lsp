namespace eval lib {proc helper {} {return good}}
namespace eval app {namespace path ::lib; source child.tcl}
puts [app::run]
