/* Fixed WASIp1 libc/environment observation, without Tcl or Rust getters. */
#include <errno.h>
#include <float.h>
#include <limits.h>
#include <math.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct input_case {
    const char *name;
    const unsigned char *bytes;
    size_t length;
};
static const unsigned char input_0[] = {0};
static const unsigned char input_1[] = {98, 97, 100, 0};
static const unsigned char input_2[] = {48, 120, 49, 48, 0};
static const unsigned char input_3[] = {48, 56, 0};
static const unsigned char input_4[] = {32, 9, 45, 49, 55, 116, 97, 105, 108, 0};
static const unsigned char input_5[] = {43, 49, 55, 0};
static const unsigned char input_6[] = {45, 48, 0};
static const unsigned char input_7[] = {45, 49, 0};
static const unsigned char input_8[] = {50, 49, 52, 55, 52, 56, 51, 54, 52, 55, 0};
static const unsigned char input_9[] = {50, 49, 52, 55, 52, 56, 51, 54, 52, 56, 0};
static const unsigned char input_10[] = {45, 50, 49, 52, 55, 52, 56, 51, 54, 52, 56, 0};
static const unsigned char input_11[] = {45, 50, 49, 52, 55, 52, 56, 51, 54, 52, 57, 0};
static const unsigned char input_12[] = {49, 56, 52, 52, 54, 55, 52, 52, 48, 55, 51, 55, 48, 57, 53, 53, 49, 54, 49, 53, 0};
static const unsigned char input_13[] = {49, 56, 52, 52, 54, 55, 52, 52, 48, 55, 51, 55, 48, 57, 53, 53, 49, 54, 49, 54, 0};
static const unsigned char input_14[] = {78, 97, 78, 0};
static const unsigned char input_15[] = {73, 110, 102, 0};
static const unsigned char input_16[] = {45, 73, 110, 102, 0};
static const unsigned char input_17[] = {49, 101, 53, 48, 48, 48, 0};
static const unsigned char input_18[] = {49, 101, 45, 53, 48, 48, 48, 0};
static const unsigned char input_19[] = {49, 0, 88, 0};
static const unsigned char input_20[] = {0, 49, 0};
static const struct input_case cases[] = {
    {"empty", input_0, 0},
    {"bad", input_1, 3},
    {"hex16", input_2, 4},
    {"legacy_octal08", input_3, 2},
    {"space_sign_tail", input_4, 9},
    {"positive_sign", input_5, 3},
    {"negative_zero", input_6, 2},
    {"negative_one", input_7, 2},
    {"signed32_max", input_8, 10},
    {"signed32_overflow", input_9, 10},
    {"signed32_min", input_10, 11},
    {"signed32_underflow", input_11, 11},
    {"unsigned64_max", input_12, 20},
    {"unsigned64_overflow", input_13, 20},
    {"nan", input_14, 3},
    {"positive_infinity", input_15, 3},
    {"negative_infinity", input_16, 4},
    {"double_overflow", input_17, 6},
    {"double_underflow", input_18, 7},
    {"counted_one_nul_x", input_19, 3},
    {"counted_initial_nul_one", input_20, 2},
};
static void hex(const unsigned char *bytes, size_t count) {
    size_t i;
    for (i = 0; i < count; ++i) printf("%02x", (unsigned int)bytes[i]);
}
static size_t prefix_length(const struct input_case *input) {
    size_t length = 0;
    while (length < input->length && input->bytes[length] != 0) ++length;
    return length;
}
static void observe(size_t index, int seed_kind, int seed, int worker) {
    const struct input_case *input = &cases[index];
    const char *start = (const char *)input->bytes;
    char *end = NULL;
    int before, call_errno, after;
    long signed_value = 0;
    unsigned long unsigned_value = 0;
    unsigned long long wide_value = 0;
    double double_value = 0;
    errno = seed;
    before = errno;
    if (worker == 4) errno = 0;
    call_errno = errno;
    switch (worker) {
    case 0: signed_value = strtol(start, &end, 0); break;
    case 1: unsigned_value = strtoul(start, &end, 0); break;
    case 2: wide_value = strtoull(start, &end, 0); break;
    case 3:
    case 4: double_value = strtod(start, &end); break;
    default: return;
    }
    after = errno; /* Observe immediately, before printf or float observers. */
    printf("ROW\tcase=%zu\tworker=%d\tseed_kind=%d\tseed=%d\tbefore=%d"
           "\tcall_errno=%d\tafter=%d\tend=%td\tinput_len=%zu\tprefix_len=%zu",
           index, worker, seed_kind, seed, before, call_errno, after,
           end - start, input->length, prefix_length(input));
    if (worker == 0) printf("\tvalue=%ld", signed_value);
    if (worker == 1) printf("\tvalue=%lu", unsigned_value);
    if (worker == 2) printf("\tvalue=%llu", wide_value);
    if (worker == 3 || worker == 4) {
        printf("\tvalue=%a\tdouble_bytes=", double_value);
        hex((const unsigned char *)&double_value, sizeof(double_value));
        printf("\tis_nan=%d\tis_infinite=%d\tsignbit=%d",
               isnan(double_value) != 0, isinf(double_value) != 0,
               signbit(double_value) != 0);
    }
    printf("\n");
}
static void adapter_boundary(int seed_kind, int seed, size_t offset) {
    /* Same counted-prefix admission geometry as the existing environment:
       never call libc from an offset beyond the first original NUL. */
    const unsigned char bytes[] = {'1', 0, 'X', 0};
    const size_t counted_length = 3, prefix = 1;
    const char *start = (const char *)bytes;
    char *end = NULL;
    int before, after;
    unsigned long long value = 0;
    errno = seed;
    before = errno;
    if (offset > prefix) {
        after = errno;
        printf("ADAPTER\tseed_kind=%d\tseed=%d\toffset=%zu\tcall=rejected"
               "\tbefore=%d\tafter=%d\tinput_len=%zu\tprefix_len=%zu\n",
               seed_kind, seed, offset, before, after, counted_length, prefix);
        return;
    }
    value = strtoull(start + offset, &end, 0);
    after = errno;
    printf("ADAPTER\tseed_kind=%d\tseed=%d\toffset=%zu\tcall=executed"
           "\tbefore=%d\tafter=%d\tvalue=%llu\tend=%td"
           "\tinput_len=%zu\tprefix_len=%zu\n",
           seed_kind, seed, offset, before, after, value, end - start,
           counted_length, prefix);
}
int main(void) {
    const int seeds[] = {0, EDOM, ERANGE};
    size_t index, offset;
    int seed_kind, worker;
    printf("ABI\tCHAR_BIT=%d\tchar=%zu\tint=%zu\tlong=%zu\tlong_long=%zu"
           "\tdouble=%zu\tpointer=%zu\tCHAR_MIN=%d\tCHAR_MAX=%d"
           "\tINT_MIN=%d\tINT_MAX=%d\tUINT_MAX=%u"
           "\tLONG_MIN=%ld\tLONG_MAX=%ld\tULONG_MAX=%lu"
           "\tLLONG_MIN=%lld\tLLONG_MAX=%lld\tULLONG_MAX=%llu"
           "\tEDOM=%d\tERANGE=%d\tDBL_MANT_DIG=%d\tDBL_MIN_EXP=%d"
           "\tDBL_MAX_EXP=%d\tSTDC_VERSION=%ld\n",
           CHAR_BIT, sizeof(char), sizeof(int), sizeof(long), sizeof(long long),
           sizeof(double), sizeof(void *), CHAR_MIN, CHAR_MAX, INT_MIN, INT_MAX,
           UINT_MAX, LONG_MIN, LONG_MAX, ULONG_MAX, LLONG_MIN, LLONG_MAX,
           ULLONG_MAX, EDOM, ERANGE, DBL_MANT_DIG, DBL_MIN_EXP, DBL_MAX_EXP,
           (long)__STDC_VERSION__);
    for (seed_kind = 0; seed_kind < 3; ++seed_kind) {
        int from_pointer, from_macro, before_reset, after_reset, same_cell;
        *__errno_location() = seeds[seed_kind];
        from_pointer = errno;
        errno = 123;
        from_macro = *__errno_location();
        same_cell = &errno == __errno_location();
        errno = seeds[seed_kind];
        before_reset = errno;
        errno = 0;
        after_reset = errno;
        printf("ERRNO_CELL\tseed_kind=%d\tseed=%d\tvia_pointer=%d"
               "\tvia_macro=%d\tsame_cell=%d\tbefore_reset=%d\tafter_reset=%d\n",
               seed_kind, seeds[seed_kind], from_pointer, from_macro, same_cell,
               before_reset, after_reset);
        if (from_pointer != seeds[seed_kind] || from_macro != 123 || !same_cell
            || after_reset != 0) return 2;
    }
    for (index = 0; index < sizeof(cases) / sizeof(cases[0]); ++index) {
        printf("CASE\tcase=%zu\tname=%s\tinput_len=%zu\tprefix_len=%zu\tinput=",
               index, cases[index].name, cases[index].length,
               prefix_length(&cases[index]));
        hex(cases[index].bytes, cases[index].length);
        printf("\n");
        for (seed_kind = 0; seed_kind < 3; ++seed_kind)
            for (worker = 0; worker < 5; ++worker)
                observe(index, seed_kind, seeds[seed_kind], worker);
    }
    for (seed_kind = 0; seed_kind < 3; ++seed_kind)
        for (offset = 0; offset <= 3; ++offset)
            adapter_boundary(seed_kind, seeds[seed_kind], offset);
    printf("SUMMARY\tcases=%zu\trows=%zu\terrno_cells=3\tadapter_rows=12\n",
           sizeof(cases) / sizeof(cases[0]),
           (sizeof(cases) / sizeof(cases[0])) * 3 * 5);
    return ferror(stdout) ? 3 : 0;
}
