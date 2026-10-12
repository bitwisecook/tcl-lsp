# Compiled foreach allocator ownership diagnosis

Scope: the exact [retained input](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/compiled-foreach-input.tcl)
and [abort receipt](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/runtime-compiled-foreach-record.json).
The frozen C8.6 core-only runner exits on SIGABRT (`-6`) with
`realloc(): invalid pointer`; its input hash is
`adbf44511437984ac055a50a009bf85591101864b471fe7a52068612829b5083`.
There is no retained native backtrace or observed failing object address.

Inspection was limited to `native_each.rs` and three direct owners: `obj.rs`,
`list.rs`, and `native_list_conversion.rs`. All four current source hashes
match the retained final build-source inventory. That establishes source
currency for this inspection, not independent authentication of binary bytes.

The concrete candidate is iterator-counter representation ownership, rather
than an inferred borrowed-list-member free:

1. `native_each.rs:137–152` constructs `StackIteration.counts` with
   `Owned::fresh(new_string_bytes(b""))`. `obj.rs:2376–2386` gives that empty
   plain string canonical static storage, length zero, NULL type, and rep zero.
2. `StackIteration::advance` calls `change_type(counts, NULL, iteration)`;
   `native_each.rs:393` supplies one, two, then three for this three-item list.
   The NULL type is retained while the rep is repurposed as a counter.
3. `obj.rs:1123–1150` treats an old NULL type as a plain string and invokes
   `shrink_string_to_exact`. Its contract requires the rep to be the allocation
   capacity, not an iteration number.
4. `obj.rs:1270–1294` excludes NULL bytes but does not exclude canonical empty
   static bytes. On the third advance, the old rep is two while exact size is
   `length + 1 == 1`. It therefore selects `realloc` on the canonical static
   pointer with an alleged two-byte old allocation. That pointer is not owned
   allocator storage. The first two advances avoid this branch (old reps zero
   and one), so the three-item input has the relevant threshold.

This chain violates the object owner's buffer-capacity invariant and explains
a concrete route to the retained allocator report. It is source evidence;
the actual abort has not yet been attributed to that object with a backtrace.
`free_string_buffer` separately checks canonical empty identity at
`obj.rs:2437`, but that cleanup guard does not protect the earlier shrink.
The auxiliary object also stores a recipe pointer under a NULL type, so its
payload representation needs the same owner review; no destructor failure is
established here.

The stack group retains a genuine `Owned` list header through the loop, and
the list-copy path shares its member backing. `BodyListArguments` supplies
allocation/frame selection checks. Those inspected paths provide no equally
concrete member-retirement explanation for this exact input; they do not prove
all callback/reentrancy lifetimes sound.

Minimal next evidence, after the source freeze is released: instrument only
`StackIteration::advance` and the shrink boundary to record object identity,
iteration, type, rep, length and canonical-empty identity before any realloc.
Run the unchanged input once in a fresh isolated process and retain the first
failing boundary/backtrace. No global allocator log is needed. A focused
iterator-counter test should cover advances one/two/three without procedure
publication; a three-element empty-body compiled loop can distinguish this
candidate from command replacement. Keep abort-prone diagnosis process-isolated.
Confirm the test selected the compiled StackLists path, rather than a generic loop.

Any repair must preserve authenticated iterator/object roles and the allocation
capacity contract; changing the error text or bypassing checks is insufficient.
No source edit, build, additional oracle run or successful repair is claimed.
Stock C/Jim completion and the separate C8.4 namespace failure supply no proof
of this runtime object's ownership. F5 remains unmeasured.
