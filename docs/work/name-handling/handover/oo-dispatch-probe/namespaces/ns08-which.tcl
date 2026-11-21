namespace eval ::lib { namespace export f; proc f {} {}; variable v 1 }
namespace eval ::app { namespace import ::lib::f; puts [list [namespace which -command f] [namespace which f] [namespace which -variable ::lib::v] [namespace which -command nosuch]] }
