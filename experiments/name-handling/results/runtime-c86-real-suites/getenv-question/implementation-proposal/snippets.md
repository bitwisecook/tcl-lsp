# Unexecuted insertion sketches

These small snippets illustrate the proposed owners; they are not a patch or
compiled evidence. Production Rust remains unchanged.

The dependency-free platform capability and Env default:

```rust
pub trait NativeEnvironmentBytes {
    fn get_bytes(&self, key: &[u8]) -> Option<Vec<u8>>;
}

// Inside the existing Env trait:
fn native_environment_bytes(&self) -> Option<&dyn NativeEnvironmentBytes> {
    None
}
```

The Unix NativeEnv view uses its actual existing override and process owners:

```rust
#[cfg(unix)]
impl tcl_platform::NativeEnvironmentBytes for NativeEnv {
    fn get_bytes(&self, key: &[u8]) -> Option<Vec<u8>> {
        use std::os::unix::ffi::OsStrExt as _;
        if let Ok(text) = std::str::from_utf8(key)
            && let Some(value) = self.overrides.borrow().get(text)
        {
            return Some(value.as_bytes().to_vec());
        }
        std::env::var_os(std::ffi::OsStr::from_bytes(key))
            .map(|value| value.as_os_str().as_bytes().to_vec())
    }
}

// Inside the existing NativeEnv impl Env, on Unix only:
#[cfg(unix)]
fn native_environment_bytes(&self) -> Option<&dyn tcl_platform::NativeEnvironmentBytes> {
    Some(self)
}
```

The runtime adapter keeps original byte conversion and operation currency:

```rust
fn environment_value(
    interp: &mut Interp,
    original: *mut TclObj,
) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
    use tcl_syntax::value::{ValueError, ValueOps};
    let name = interp.native_string_bytes(&original)?;
    let host = interp.host();
    let environment = host.env().native_environment_bytes().ok_or_else(|| {
        tcl_cmd_core::CmdError::from(ValueError::CommandProtocolUnavailable(
            "native environment byte getter",
        ))
    })?;
    let value = environment
        .get_bytes(tcl_core_types::c_string_extent(&name))
        .unwrap_or_default();
    interp.check_entered_native_operation()
        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
    Ok(value)
}
```

At successful publication the object adapter uses `ValueOps::new_bytes` and
`set_result`. At an arity failure it preserves the existing stated-error ABI:

```rust
let code = interp.wrong_args_for_invocation(argv, b"name");
if !interp.host_refusal_pending() {
    interp.note_c_api_error();
}
return code;
```

Each of two actual registrations consumes one clone of the same pool:

```rust
let pool = std::rc::Rc::new(ClockLiteralPool::new());
let data = std::rc::Rc::into_raw(std::rc::Rc::clone(&pool)).cast_mut().cast();
// Pass data with the actual worker's ObjCommand and shared once-only callback.
// Repeat for the second actual worker, then drop the constructor's pool handle.

unsafe extern "C" fn delete_clock_pool(data: *mut std::ffi::c_void) {
    unsafe { drop(std::rc::Rc::from_raw(data.cast::<ClockLiteralPool>())) };
}
```

The future pool regression must retain both result lists, delete ParseFormatArgs
and observe literal refcount3 while getenv still owns the pool, delete getenv
and observe refcount2, then verify retained values. It must also check the
opposite deletion order and failed creation cleanup. This is genuine ownership
by two implemented workers; five absent registrations acquire no owners.
