/*
 * tcl-lsp — a language server and toolchain for Tcl
 * Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 * 
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 * 
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Same local element width/alignment as the selected Jim pointer vector.
 * This does not establish a foreign Jim build, private object layout or libc. */
_Static_assert(sizeof(size_t) == sizeof(void *), "pointer-sized qsort index");
_Static_assert(_Alignof(size_t) == _Alignof(void *), "pointer-aligned qsort index");

typedef int (*tcl_index_compare)(void *, size_t, size_t, int *);
struct tcl_sort_frame {
    tcl_index_compare compare;
    void *context;
    size_t *vector;
    size_t *frontier;
    size_t length;
    int refused;
    struct tcl_sort_frame *previous;
};
static _Thread_local struct tcl_sort_frame *tcl_sort_current;

static int tcl_compare_indices(const void *left, const void *right)
{
    struct tcl_sort_frame *frame = tcl_sort_current;
    int order = 0;
    if (frame->refused) {
        return 0;
    }
    /* The Rust callback has RETURNED before any snapshot or inert continuation.
     * No longjmp ever crosses Rust, or skips libc's scratch cleanup. */
    if (frame->compare(frame->context, *(const size_t *)left,
            *(const size_t *)right, &order) != 0) {
        memcpy(frame->frontier, frame->vector, frame->length * sizeof(size_t));
        frame->refused = 1;
        return 0;
    }
    return order;
}

/* All storage belongs to the synchronous caller; neither pointer escapes.
 * Nested same-thread invocations restore the actual previous comparator frame. */
int tcl_native_sort_indices(size_t *vector, size_t length, size_t *frontier,
        void *context, tcl_index_compare compare)
{
    struct tcl_sort_frame frame = {compare, context, vector, frontier,
        length, 0, tcl_sort_current};
    if (length <= 1) {
        return 0;
    }
    tcl_sort_current = &frame;
    qsort(vector, length, sizeof(size_t), tcl_compare_indices);
    tcl_sort_current = frame.previous;
    return frame.refused;
}
