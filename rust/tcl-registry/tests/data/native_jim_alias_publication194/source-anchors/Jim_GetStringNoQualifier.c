static const char *Jim_GetStringNoQualifier(Jim_Obj *objPtr, int *length)
{
    int len;
    const char *str = Jim_GetString(objPtr, &len);
    if (len >= 2 && str[0] == ':' && str[1] == ':') {
        while (len && *str == ':') {
            len--;
            str++;
        }
    }
    *length = len;
    return str;
}

/* Hash a command-table key, ignoring any namespace prefix. */
static unsigned int JimCommandsHT_HashFunction(const void *key)
{
    int len;
    const char *str = Jim_GetStringNoQualifier((Jim_Obj *)key, &len);
    return Jim_GenHashFunction((const unsigned char *)str, len);
}

/* Compare command names, allowing namespace-qualified lookups to match local entries. */
static int JimCommandsHT_KeyCompare(void *privdata, const void *key1, const void *key2)
{
    int len1, len2;
    const char *str1 = Jim_GetStringNoQualifier((Jim_Obj *)key1, &len1);
    const char *str2 = Jim_GetStringNoQualifier((Jim_Obj *)key2, &len2);
    return len1 == len2 && *str1 == *str2 && memcmp(str1, str2, len1) == 0;
}
