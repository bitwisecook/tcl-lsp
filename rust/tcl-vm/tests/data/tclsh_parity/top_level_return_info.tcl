# A return -code error with its own errorinfo leaving the script file: 8.4
# logs the return, from 8.5 the file frame follows the given errorinfo.
puts a
return -code error -errorinfo {SOURCE INFO} -errorcode {SOURCE CODE} FAILURE
puts b
