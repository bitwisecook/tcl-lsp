static int SetDictFromAny(Jim_Interp *interp, struct Jim_Obj *objPtr)
{
    int listlen;

    if (objPtr->typePtr == &dictObjType) {
        return JIM_OK;
    }

    if (Jim_IsList(objPtr) && Jim_IsShared(objPtr)) {
        /* A shared list, so get the string representation now to avoid
         * losing duplicate keys from the string rep when converting to
         * a dict.
         */
        Jim_String(objPtr);
    }

    /* Convert a non-list object to a list and then to a dict
     * since we will need the list of key, value pairs anyway
     */
    listlen = Jim_ListLength(interp, objPtr);
    if (listlen % 2) {
        Jim_SetResultString(interp, "missing value to go with key", -1);
        return JIM_ERR;
    }
    else {
        /* Allocate space in the hash table for twice the number of elements */
        Jim_Dict *dict = JimDictNew(interp, 0, listlen);
        int i;

        /* Take ownership of the list array */
        dict->table = objPtr->internalRep.listValue.ele;
        dict->maxLen = objPtr->internalRep.listValue.maxLen;

        /* Now add all the elements to the hash table */
        for (i = 0; i < listlen; i += 2) {
            int tvoffset = JimDictAdd(dict, dict->table[i]);
            if (tvoffset) {
                /* A duplicate key, so replace the value but and don't add a new entry */
                /* Discard the old value */
                Jim_DecrRefCount(interp, dict->table[tvoffset]);
                /* Set the new value */
                dict->table[tvoffset] = dict->table[i + 1];
                /* Discard the duplicate key */
                Jim_DecrRefCount(interp, dict->table[i]);
            }
            else {
                if (dict->len != i) {
                    /* Need to move later entries down to fill the hole created by
                     * a previous duplicate entry.
                     */
                    dict->table[dict->len++] = dict->table[i];
                    dict->table[dict->len++] = dict->table[i + 1];
                }
                else {
                    dict->len += 2;
                }
            }
        }

        objPtr->typePtr = &dictObjType;
        objPtr->internalRep.dictValue = dict;

        return JIM_OK;
    }
}

/* Dict object API */
