# naming.runtime.original-literal-owner-retirement

Kind: `implementation-contract`

## Problem statement

An original literal owner can contain a nested artifact whose destructor releases another registration in the same software pool. Dropping the last owner while holding the mutable pool borrow prevents that nested release from accessing its genuine owner.

## Question

How does the shared literal registration owner retire the final table/index entry and return its original owner for destruction after the mutable pool borrow ends?

## Conclusion

NativeLiteralWorld::release returns Option<V>. On final local-array release it retires the registration/index entry and invalidates the observation before returning the original owner; a retained registration returns None. Concrete RefCell adapters bind that returned owner in one statement and drop it in a later statement after the RefMut ends. The API neither clones the final owner nor calls a getter to authenticate it. Runtime and VM local literal arrays use this shared software ownership contract; local member destruction remains a separate owner action. It provides no native destructor chronology, private table/header/refcount identity, original compilation/activation admission or executed Rust result.

## Scope

Implementation source and software allocation controls only. All seven external providers are not tested for this API contract. Native public namespace teardown values and separately pinned literal/object observations keep their own scopes and do not establish these Rust registrations or borrow lifetimes. Correct RefCell usage is two statements: let retired = world.borrow_mut().release(registration); followed by drop(retired); an inline drop(world.borrow_mut().release(...)) retains the temporary RefMut through destruction. Two linked software allocation controls exercise nested destruction after final release: the shared registration owner and an actual Runtime original Bytecode with nested literal registrations. They check their own Rust registrations and RefCell borrow boundary; separately observed native namespace outputs do not certify destructor chronology or private registration identity.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_literal.rs](../../../../rust/tcl-runtime-api/src/native_literal.rs), `NativeLiteralWorld::release`: Retire final registration/index/observation before returning its original owner for destruction outside the mutable pool borrow; no extra clone/getter or native identity claim.
- [runtime/rust/src/interp/native_literal_pool.rs](../../../../runtime/rust/src/interp/native_literal_pool.rs), `NativeRuntimeLiteralArray::drop`: Release each retained registration into a local binding, then drop the returned owner after the RefMut ends; actual local member owners drop independently.
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs), `NativeLiteralWorld::release`: Transport the shared final owner to concrete VM callers so they can end the pool borrow before destructor reentry.
- [rust/tcl-runtime-api/src/native_literal.rs](../../../../rust/tcl-runtime-api/src/native_literal.rs), `native_literal::empty_world_tests::detached_final_owner_can_release_nested_registration_without_pool_borrow` (linked): The software final owner detaches without running a destructor; its later drop releases an independently retained child registration after the mutable RefCell borrow ends. Exact two-owner drop count and empty software inventory are asserted, without native destructor or table-identity evidence.
- [runtime/rust/src/interp/native_literal_pool.rs](../../../../runtime/rust/src/interp/native_literal_pool.rs), `interp::native_literal_pool::tests::retiring_registered_bytecode_releases_nested_literals_outside_world_borrow` (linked): An authenticated Runtime C8.4 NativeCore constructor retains actual compiler epochs, original Bytecode cache and nested registrations in the same software world. Releasing the final owner outside the mutable borrow empties that inventory. This is the software allocation control definition, not an executed positive result or separately observed native destructor chronology.

## Evidence

- [rust/tcl-runtime-api/src/native_literal.rs](../../../../rust/tcl-runtime-api/src/native_literal.rs)
- [runtime/rust/src/interp/native_literal_pool.rs](../../../../runtime/rust/src/interp/native_literal_pool.rs)
- [rust/tcl-vm/src/literal_pool.rs](../../../../rust/tcl-vm/src/literal_pool.rs)
