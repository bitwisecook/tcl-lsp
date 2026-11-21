package require overrides
proc ::foo {} {return orig}
overrides::withCmdOverrides {::foo {lambda {{} {return over}}}} { proc helper {} {return h}; proc ::bar {} {} ; rename ::bar baz }
puts global:[info commands ::helper]|ns:[info commands ::overrides::helper]|baz:[info commands ::baz]|nsbaz:[info commands ::overrides::baz]
