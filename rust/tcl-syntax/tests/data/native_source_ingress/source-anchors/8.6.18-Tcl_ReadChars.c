Tcl_ReadChars(
    Tcl_Channel chan,		/* The channel to read. */
    Tcl_Obj *objPtr,		/* Input data is stored in this object. */
    int toRead,		/* Maximum number of characters to store, or
				 * -1 to read all available data (up to EOF or
				 * when channel blocks). */
    int appendFlag)		/* If non-zero, data read from the channel
				 * will be appended to the object. Otherwise,
				 * the data will replace the existing contents
				 * of the object. */
{
    Channel *chanPtr = (Channel *) chan;
    ChannelState *statePtr = chanPtr->state;
				/* State info for channel */

    /*
     * This operation should occur at the top of a channel stack.
     */

    chanPtr = statePtr->topChanPtr;

    if (CheckChannelErrors(statePtr, TCL_READABLE) != 0) {
	/*
	 * Update the notifier state so we don't block while there is still
	 * data in the buffers.
	 */

	UpdateInterest(chanPtr);
	return -1;
    }

    return DoReadChars(chanPtr, objPtr, toRead, appendFlag);
}
/*
 *---------------------------------------------------------------------------
 *
 * DoReadChars --
 *
 *	Reads from the channel until the requested number of characters have
 *	been seen, EOF is seen, or the channel would block. EOL and EOF
 *	translation is done. If reading binary data, the raw bytes are wrapped
 *	in a Tcl byte array object. Otherwise, the raw bytes are converted to
 *	UTF-8 using the channel's current encoding and stored in a Tcl string
 *	object.
 *
 * Results:
 *	The number of characters read, or -1 on error. Use Tcl_GetErrno() to
 *	retrieve the error code for the error that occurred.
 *
 * Side effects:
 *	May cause input to be buffered.
 *
 *---------------------------------------------------------------------------
 */

static int
DoReadChars(
    Channel *chanPtr,		/* The channel to read. */
    Tcl_Obj *objPtr,		/* Input data is stored in this object. */
    int toRead,		/* Maximum number of characters to store, or
				 * -1 to read all available data (up to EOF or
