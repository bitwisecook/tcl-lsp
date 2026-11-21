namespace eval ::a { namespace export f; proc f {} {return a} }
namespace eval ::b { namespace export f; namespace import ::a::f }
namespace eval ::c { namespace import ::b::f; puts [list [namespace origin f] [f]] }
