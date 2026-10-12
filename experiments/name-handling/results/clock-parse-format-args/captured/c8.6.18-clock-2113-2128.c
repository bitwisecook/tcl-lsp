static void
ClockDeleteCmdProc(
    ClientData clientData)	/* Opaque pointer to the client data */
{
    ClockClientData *data = (ClockClientData *)clientData;
    int i;

    if (data->refCount-- <= 1) {
	for (i = 0; i < LIT__END; ++i) {
	    Tcl_DecrRefCount(data->literals[i]);
	}
	ckfree(data->literals);
	ckfree(data);
    }
}

