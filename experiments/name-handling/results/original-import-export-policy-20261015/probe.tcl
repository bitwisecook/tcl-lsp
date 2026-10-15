namespace eval ::Source {proc p {} {return SOURCE}}
namespace eval ::Target {}
set c [catch {namespace eval ::Target {namespace import ::Source::*}} m]
puts [list NOEXPORT_IMPORT $c $m]
puts [list NOEXPORT_CALL [catch {::Target::p} m] $m]
namespace eval ::Self {proc p {} {return SELF}}
puts [list SELF_IMPORT [catch {namespace eval ::Self {namespace import ::Self::*}} m] $m]
puts [list SELF_CALL [catch {::Self::p} m] $m]
