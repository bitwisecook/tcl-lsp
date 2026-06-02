proc greet {} {
    return "hello"
}

# First call is fine — greet is still bound here.
greet

# Rename greet away; the old name now falls through to `unknown`.
rename greet hail

# This call to the renamed-away name should raise W128.
greet
