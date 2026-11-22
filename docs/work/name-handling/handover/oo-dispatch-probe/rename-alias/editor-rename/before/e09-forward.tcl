proc helper {args} {return fw:$args}
oo::class create C { forward f helper x }
puts [[C new] f y]
