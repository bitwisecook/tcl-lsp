static int JimExprOpAnd(Jim_Interp *interp, struct JimExprNode *node)
{
    /* evaluate left */
    int result = JimExprGetTermBoolean(interp, node->left);

    if (result == 1) {
        /* true so evaluate right */
        result = JimExprGetTermBoolean(interp, node->right);
    }
    if (result == -1) {
        return JIM_ERR;
    }
    Jim_SetResultInt(interp, result);
    return JIM_OK;
}
