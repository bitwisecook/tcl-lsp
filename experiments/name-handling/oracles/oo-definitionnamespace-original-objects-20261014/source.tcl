namespace eval ::dest {}
namespace eval ::outer {namespace eval dest {}; oo::class create C}
namespace eval ::other {namespace eval dest {}; oo::class create C}
set name dest
namespace eval ::outer {oo::define C definitionnamespace $::name}
puts [list outer [info class definitionnamespace ::outer::C]]
namespace eval ::other {oo::define C definitionnamespace $::name}
puts [list other [info class definitionnamespace ::other::C]]
puts [list primary [string match {*nsName*} [tcl::unsupported::representation $name]]]
set name [binary format H* 64657374007461696c]
set failed [catch {namespace eval ::outer {oo::define C definitionnamespace $::name}} message options]
puts [list rawzero-error $failed [binary encode hex $message] [dict get $options -errorcode]]
puts [list rawzero [info class definitionnamespace ::outer::C]]
namespace eval ::outer {oo::define C definitionnamespace {}}
puts [list reset [info class definitionnamespace ::outer::C]]
