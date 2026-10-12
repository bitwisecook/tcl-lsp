
// Make a blank method record or look up the existing one.
static inline Method *
GetOrCreateMethod(
    Tcl_HashTable *tablePtr,
    Tcl_Obj *namePtr,
    int *isNew)
{
    Tcl_HashEntry *hPtr = Tcl_CreateHashEntry(tablePtr, namePtr, isNew);
    if (*isNew) {
	Method *mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	memset(mPtr, 0, sizeof(Method));
	mPtr->refCount = 1;
	mPtr->namePtr = namePtr;
	Tcl_IncrRefCount(namePtr);
	Tcl_SetHashValue(hPtr, mPtr);
	return mPtr;
    } else {
	return (Method *) Tcl_GetHashValue(hPtr);
    }
}

static bool
ExportMethod(
    Class *clsPtr,
    Tcl_Obj *namePtr)
{
    int isNew;
    Method *mPtr = GetOrCreateMethod(&clsPtr->classMethods, namePtr, &isNew);
    if (isNew || !(mPtr->flags & (PUBLIC_METHOD | PRIVATE_METHOD))) {
	mPtr->flags |= PUBLIC_METHOD;
	mPtr->flags &= ~TRUE_PRIVATE_METHOD;
	isNew = 1;
    }
    return isNew;
}

static bool
UnexportMethod(
    Class *clsPtr,
    Tcl_Obj *namePtr)
{
    int isNew;
    Method *mPtr = GetOrCreateMethod(&clsPtr->classMethods, namePtr, &isNew);
    if (isNew || mPtr->flags & (PUBLIC_METHOD | TRUE_PRIVATE_METHOD)) {
	mPtr->flags &= ~(PUBLIC_METHOD | TRUE_PRIVATE_METHOD);
	isNew = 1;
    }
    return isNew;
}

// Make the table of methods in the instance if it doesn't already exist.
static inline void
InitMethodTable(
    Object *oPtr)
{
    if (!oPtr->methodsPtr) {
	oPtr->methodsPtr = (Tcl_HashTable *) Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitObjHashTable(oPtr->methodsPtr);
	oPtr->flags &= ~USE_CLASS_CACHE;
    }
}

static bool
ExportInstanceMethod(
    Object *oPtr,
    Tcl_Obj *namePtr)
{
    InitMethodTable(oPtr);

    int isNew;
    Method *mPtr = GetOrCreateMethod(oPtr->methodsPtr, namePtr, &isNew);
    if (isNew || !(mPtr->flags & (PUBLIC_METHOD | PRIVATE_METHOD))) {
	mPtr->flags |= PUBLIC_METHOD;
	mPtr->flags &= ~TRUE_PRIVATE_METHOD;
	isNew = 1;
    }
    return isNew;
}

static bool
UnexportInstanceMethod(
    Object *oPtr,
    Tcl_Obj *namePtr)
{
    InitMethodTable(oPtr);

    int isNew;
    Method *mPtr = GetOrCreateMethod(oPtr->methodsPtr, namePtr, &isNew);
    if (isNew || mPtr->flags & (PUBLIC_METHOD | TRUE_PRIVATE_METHOD)) {
	mPtr->flags &= ~(PUBLIC_METHOD | TRUE_PRIVATE_METHOD);
	isNew = 1;
    }
    return isNew;
}

bool
