const Tcl_HashKeyType tclObjHashKeyType = {
    TCL_HASH_KEY_TYPE_VERSION,	/* version */
    TCL_HASH_KEY_DIRECT_COMPARE,/* allows compare keys by pointers */
    TclHashObjKey,		/* hashKeyProc */
    TclCompareObjKeys,		/* compareKeysProc */
    AllocObjEntry,		/* allocEntryProc */
    TclFreeObjEntry		/* freeEntryProc */
};

