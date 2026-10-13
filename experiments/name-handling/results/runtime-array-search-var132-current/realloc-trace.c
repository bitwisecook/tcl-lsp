#define _GNU_SOURCE
#include <execinfo.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
extern void *__libc_realloc(void *, size_t);
static __thread void *current_pointer;
static __thread size_t current_size;
static void abort_trace(int signal_number) {
    void *frames[40];
    int count = backtrace(frames, 40);
    char header[120];
    int length = snprintf(header, sizeof header, "NH_REALLOC pointer=%p size=%zu\n", current_pointer, current_size);
    write(2, header, length);
    backtrace_symbols_fd(frames, count, 2);
    signal(signal_number, SIG_DFL);
    raise(signal_number);
}
__attribute__((constructor)) static void install_trace(void) {
    signal(SIGABRT, abort_trace);
}
void *realloc(void *ptr, size_t size) {
    current_pointer = ptr;
    current_size = size;
    void *result = __libc_realloc(ptr, size);
    current_pointer = NULL;
    current_size = 0;
    return result;
}
