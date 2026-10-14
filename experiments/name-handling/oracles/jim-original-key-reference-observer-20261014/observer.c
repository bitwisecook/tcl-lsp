#include "jim.h"
#include <stdio.h>

int main(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_EvalSource(interp, "original-key.tcl", 1, "proc P {} {}") != JIM_OK) return 2;
    Jim_Obj *head = Jim_NewStringObj(interp, "P", 1);
    Jim_IncrRefCount(head);
    Jim_HashEntry *entry = Jim_FindHashEntry(&interp->commands, head);
    if (!entry) return 3;
    Jim_Obj *key = Jim_GetHashEntryKey(entry);
    int before = key->refCount;
    Jim_Cmd *command = Jim_GetCommand(interp, head, JIM_ERRMSG);
    printf("cold same_key %d refs_before %d refs_after %d\n", command && command->cmdNameObj == key, before, key->refCount);
    int hot_before = key->refCount;
    Jim_Cmd *hot = Jim_GetCommand(interp, head, JIM_ERRMSG);
    printf("hot same_command %d same_key %d refs_before %d refs_after %d\n", hot == command, hot && hot->cmdNameObj == key, hot_before, key->refCount);
    Jim_IncrRefCount(key);
    Jim_Obj *renamed = Jim_NewStringObj(interp, "Q", 1);
    Jim_IncrRefCount(renamed);
    if (Jim_RenameCommand(interp, head, renamed) != JIM_OK) return 4;
    printf("rename still_old_key %d old_key_refs %d old_key_name %s\n", command->cmdNameObj == key, key->refCount, Jim_String(key));
    entry = Jim_FindHashEntry(&interp->commands, renamed);
    if (!entry) return 5;
    Jim_Obj *new_key = Jim_GetHashEntryKey(entry);
    int renamed_before = new_key->refCount;
    Jim_Cmd *cold = Jim_GetCommand(interp, renamed, JIM_ERRMSG);
    printf("renamed_cold same_command %d same_new_key %d refs_before %d refs_after %d name %s\n", cold == command, cold && cold->cmdNameObj == new_key, renamed_before, new_key->refCount, Jim_String(new_key));
    Jim_DecrRefCount(interp, key);
    Jim_DecrRefCount(interp, head);
    Jim_DecrRefCount(interp, renamed);
    Jim_FreeInterp(interp);
    return 0;
}
