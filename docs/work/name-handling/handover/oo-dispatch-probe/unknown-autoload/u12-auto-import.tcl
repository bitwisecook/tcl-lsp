# namespace import of a pattern triggers auto_import over auto_index.
set auto_index(::lib::util) {namespace eval ::lib { proc util {} { return util } ; namespace export util }}
namespace eval ::lib { namespace export util }
namespace import ::lib::util
puts "imported: [info commands util]"
puts [util]
