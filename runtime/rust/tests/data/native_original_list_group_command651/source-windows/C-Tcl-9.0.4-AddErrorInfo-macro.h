#define Tcl_AddErrorInfo(interp, message) \
	Tcl_AppendObjToErrorInfo(interp, Tcl_NewStringObj(message, -1))
#define Tcl_AddObjErrorInfo(interp, message, length) \
	Tcl_AppendObjToErrorInfo(interp, Tcl_NewStringObj(message, length))
