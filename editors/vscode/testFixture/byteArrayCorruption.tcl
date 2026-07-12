set packet [binary format c* {128 195 255}]
set upper [string toupper $packet]
set slice [string range $packet 0 1]
