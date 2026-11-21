namespace eval ::geo { namespace export area; proc area {w h} { expr {$w*$h} }; namespace ensemble create }
puts [geo area 2 3] ;# CALL area
