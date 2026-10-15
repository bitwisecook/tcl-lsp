export TCLLIBPATH='{new path [literal];$value}'"${TCLLIBPATH:+ $TCLLIBPATH}"
printf '%s\000' "$TCLLIBPATH"
