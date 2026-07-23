proc helper {} {}
proc caller {} { helper }
caller
rename helper {}
