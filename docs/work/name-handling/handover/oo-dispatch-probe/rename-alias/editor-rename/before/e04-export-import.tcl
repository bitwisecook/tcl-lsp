namespace eval ::lib { namespace export helper; proc helper {} {return libh} }
namespace eval ::app { namespace import ::lib::helper; puts [helper] }
