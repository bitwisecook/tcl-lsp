/* Release the cache made by GetPropertyName(). */
static inline void
ReleasePropertyNameCache(
    Tcl_Interp *interp,
    GPNCache **cachePtr)
{
    if (*cachePtr) {
	GPNCache *tablePtr = *cachePtr;
	if (tablePtr->listPtr) {
	    Tcl_DecrRefCount(tablePtr->listPtr);
	}
	TclStackFree(interp, tablePtr);
	*cachePtr = NULL;
    }
