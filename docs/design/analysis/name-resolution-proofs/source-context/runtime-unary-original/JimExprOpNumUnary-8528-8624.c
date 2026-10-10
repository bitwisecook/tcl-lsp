
/* Evaluate a numeric unary operator on an expression term. */
static int JimExprOpNumUnary(Jim_Interp *interp, struct JimExprNode *node)
{
    int intresult = 1;
    int rc, bA = 0;
    double dA, dC = 0;
    jim_wide wA, wC = 0;
    Jim_Obj *A;

    if ((rc = JimExprGetTerm(interp, node->left, &A)) != JIM_OK) {
        return rc;
    }

    if ((A->typePtr != &doubleObjType || A->bytes) && JimGetWideNoErr(interp, A, &wA) == JIM_OK) {
        switch (node->type) {
            case JIM_EXPROP_FUNC_INT:
            case JIM_EXPROP_FUNC_WIDE:
            case JIM_EXPROP_FUNC_ROUND:
            case JIM_EXPROP_UNARYPLUS:
                wC = wA;
                break;
            case JIM_EXPROP_FUNC_DOUBLE:
                dC = wA;
                intresult = 0;
                break;
            case JIM_EXPROP_FUNC_ABS:
                wC = wA >= 0 ? wA : -wA;
                break;
            case JIM_EXPROP_UNARYMINUS:
                wC = -wA;
                break;
            case JIM_EXPROP_NOT:
                wC = !wA;
                break;
            default:
                abort();
        }
    }
    else if ((rc = Jim_GetDouble(interp, A, &dA)) == JIM_OK) {
        switch (node->type) {
            case JIM_EXPROP_FUNC_INT:
            case JIM_EXPROP_FUNC_WIDE:
                wC = dA;
                break;
            case JIM_EXPROP_FUNC_ROUND:
                wC = dA < 0 ? (dA - 0.5) : (dA + 0.5);
                break;
            case JIM_EXPROP_FUNC_DOUBLE:
            case JIM_EXPROP_UNARYPLUS:
                dC = dA;
                intresult = 0;
                break;
            case JIM_EXPROP_FUNC_ABS:
#ifdef JIM_MATH_FUNCTIONS
                dC = fabs(dA);
#else
                dC = dA >= 0 ? dA : -dA;
#endif
                intresult = 0;
                break;
            case JIM_EXPROP_UNARYMINUS:
                dC = -dA;
                intresult = 0;
                break;
            case JIM_EXPROP_NOT:
                wC = !dA;
                break;
            default:
                abort();
        }
    }
    else if ((rc = Jim_GetBoolean(interp, A, &bA)) == JIM_OK) {
        switch (node->type) {
            case JIM_EXPROP_NOT:
                wC = !bA;
                break;
            case JIM_EXPROP_UNARYPLUS:
            case JIM_EXPROP_UNARYMINUS:
                rc = JIM_ERR;
                Jim_SetResultFormatted(interp,
                    "can't use non-numeric string as operand of \"%s\"",
                        node->type == JIM_EXPROP_UNARYPLUS ? "+" : "-");
                break;
            default:
                abort();
        }
    }

    if (rc == JIM_OK) {
        if (intresult) {
            Jim_SetResultInt(interp, wC);
        }
        else {
            Jim_SetResult(interp, Jim_NewDoubleObj(interp, dC));
        }
    }
