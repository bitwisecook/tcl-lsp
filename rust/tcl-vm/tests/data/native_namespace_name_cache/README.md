# Namespace-only original name lookup

The original name first reads an indexed procedure local. Namespace-only lookup
bypasses its `localVarName` cache, reads the namespace cell and installs
`parsedVarName` on that same object. The local remains independent; its alias
reads the namespace value.
