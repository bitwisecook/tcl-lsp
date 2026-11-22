proc helper {args} {return h:$args}
interp alias {} al {} helper pre
puts [al x]
