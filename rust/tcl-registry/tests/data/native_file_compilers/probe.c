#include "tclInt.h"
#include <stdio.h>
int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    const char *names[] = {"file","::tcl::file::atime","::tcl::file::attributes","::tcl::file::channels","::tcl::file::copy","::tcl::file::delete","::tcl::file::dirname","::tcl::file::executable","::tcl::file::exists","::tcl::file::extension","::tcl::file::home","::tcl::file::isdirectory","::tcl::file::isfile","::tcl::file::join","::tcl::file::link","::tcl::file::lstat","::tcl::file::mkdir","::tcl::file::mtime","::tcl::file::nativename","::tcl::file::normalize","::tcl::file::owned","::tcl::file::pathtype","::tcl::file::readable","::tcl::file::readlink","::tcl::file::rename","::tcl::file::rootname","::tcl::file::separator","::tcl::file::size","::tcl::file::split","::tcl::file::stat","::tcl::file::system","::tcl::file::tail","::tcl::file::tempdir","::tcl::file::tempfile","::tcl::file::tildeexpand","::tcl::file::type","::tcl::file::volumes","::tcl::file::writable", NULL};
    puts("registration\thook");
    for (int index = 0; names[index] != NULL; ++index) {
        Command *command = (Command *) Tcl_FindCommand(interp, names[index], NULL, TCL_GLOBAL_ONLY);
        printf("%s\t%d\n", names[index], command ? command->compileProc != NULL : -1);
    }
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
