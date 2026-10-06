// SPDX-License-Identifier: AGPL-3.0-or-later
//! Loaded Tcl 8.4 updater boundary. No Tcl object layout or refcount macros
//! cross this boundary: the native interpreter result owns each constructed
//! object until the checked string observation has been copied.

use std::path::Path;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};

/// An explicitly loaded, digest-checked Tcl 8.4 integer updater.
/// The capability is thread-confined, like the native interpreter it owns.
pub struct LoadedNativeIntegerFormatter {
    #[cfg(target_os = "linux")]
    native: native::Loaded,
    _private: (),
}

impl LoadedNativeIntegerFormatter {
    /// Load the caller-selected library, verify its supplied SHA-256, actual
    /// version and symbol ownership, and initialize its native Tcl runtime.
    /// `executable` is the caller's genuine executable identity for
    /// `Tcl_FindExecutable`; it is not inferred from a Tcl language profile.
    pub fn load(
        library: &Path,
        expected_sha256: [u8; 32],
        executable: &Path,
    ) -> Result<Self, NativeIntegerFormatterUnavailable> {
        #[cfg(target_os = "linux")]
        {
            native::Loaded::load(library, expected_sha256, executable).map(|native| Self {
                native,
                _private: (),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (library, expected_sha256, executable);
            Err(NativeIntegerFormatterUnavailable::Target)
        }
    }
}

impl NativeIntegerFormatter for LoadedNativeIntegerFormatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        #[cfg(target_os = "linux")]
        {
            self.native.build
        }
        #[cfg(not(target_os = "linux"))]
        {
            // This target cannot construct a LoadedNativeIntegerFormatter.
            unreachable!("unsupported native formatter target")
        }
    }

    fn format(
        &self,
        kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        #[cfg(target_os = "linux")]
        {
            self.native.format(kind, value)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (kind, value);
            Err(NativeIntegerFormatterUnavailable::Target)
        }
    }
}

#[cfg(target_os = "linux")]
mod native {
    use super::{
        NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable, NativeIntegerKind, Path,
    };
    use sha2::{Digest, Sha256};
    use std::cell::RefCell;
    use std::ffi::{CStr, CString};
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::rc::Rc;
    use std::sync::{Mutex, OnceLock};

    // NODELETE's process-owned Tcl image can retain registered callbacks.
    // Keep the corresponding proc-fd pathname identity live for the SAME
    // lifetime: reusing a closed fd number must never let the loader return an
    // older cached image under a newly verified file's pathname.
    static PROCESS_IMAGES: OnceLock<Mutex<Vec<File>>> = OnceLock::new();

    type Version = unsafe extern "C" fn(*mut i32, *mut i32, *mut i32, *mut i32);
    type FindExecutable = unsafe extern "C" fn(*const libc::c_char);
    type CreateInterp = unsafe extern "C" fn() -> *mut libc::c_void;
    type DeleteInterp = unsafe extern "C" fn(*mut libc::c_void);
    type NewLong = unsafe extern "C" fn(libc::c_long) -> *mut libc::c_void;
    type NewWide = unsafe extern "C" fn(i64) -> *mut libc::c_void;
    type SetResult = unsafe extern "C" fn(*mut libc::c_void, *mut libc::c_void);
    type GetString = unsafe extern "C" fn(*mut libc::c_void, *mut i32) -> *const libc::c_char;

    struct Library(*mut libc::c_void);
    impl Drop for Library {
        fn drop(&mut self) {
            // SAFETY: exactly one owner closes the successful dlopen handle,
            // after the interpreter and all function uses have ended.
            unsafe { libc::dlclose(self.0) };
        }
    }

    fn digest(file: &mut File) -> Result<[u8; 32], NativeIntegerFormatterUnavailable> {
        file.seek(SeekFrom::Start(0))
            .map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 16384];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        Ok(hash.finalize().into())
    }

    fn symbol(
        library: &Library,
        name: &CStr,
        image_base: &mut Option<*mut libc::c_void>,
        file: &File,
    ) -> Result<*mut libc::c_void, NativeIntegerFormatterUnavailable> {
        // SAFETY: handle remains live, name is NUL terminated, Dl_info is an
        // initialized writable local. Every admitted symbol must belong to
        // the SAME actual shared-library image, never an interposed dependency.
        let pointer = unsafe { libc::dlsym(library.0, name.as_ptr()) };
        if pointer.is_null() {
            return Err(NativeIntegerFormatterUnavailable::Build);
        }
        let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
        if unsafe { libc::dladdr(pointer, &raw mut info) } == 0 || info.dli_fbase.is_null() {
            return Err(NativeIntegerFormatterUnavailable::Build);
        }
        if info.dli_fname.is_null() {
            return Err(NativeIntegerFormatterUnavailable::Build);
        }
        // SAFETY: dladdr returns the live image's NUL-terminated pathname.
        let image_name = unsafe { CStr::from_ptr(info.dli_fname) };
        let image_path = Path::new(std::ffi::OsStr::from_bytes(image_name.to_bytes()));
        let image_metadata =
            std::fs::metadata(image_path).map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
        let verified_metadata = file
            .metadata()
            .map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
        if image_metadata.dev() != verified_metadata.dev()
            || image_metadata.ino() != verified_metadata.ino()
        {
            return Err(NativeIntegerFormatterUnavailable::Build);
        }
        match *image_base {
            Some(base) if base != info.dli_fbase => Err(NativeIntegerFormatterUnavailable::Build),
            Some(_) => Ok(pointer),
            None => {
                *image_base = Some(info.dli_fbase);
                Ok(pointer)
            }
        }
    }

    fn pin_verified_image(file: &File) -> Result<CString, NativeIntegerFormatterUnavailable> {
        // Resolve the pinned open file, rather than looking up the supplied
        // pathname a second time after verifying it.
        let image_file = file
            .try_clone()
            .map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
        let image_fd = image_file.as_raw_fd();
        PROCESS_IMAGES
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .map_err(|_| NativeIntegerFormatterUnavailable::Build)?
            .push(image_file);
        CString::new(format!("/proc/self/fd/{image_fd}"))
            .map_err(|_| NativeIntegerFormatterUnavailable::Build)
    }

    fn native_version(version: Version) -> [i32; 4] {
        let (mut major, mut minor, mut patchlevel, mut release) = (0i32, 0i32, 0i32, 0i32);
        // SAFETY: the caller resolved Version from its live verified image;
        // all output fields are live disjoint native integers.
        unsafe {
            version(
                &raw mut major,
                &raw mut minor,
                &raw mut patchlevel,
                &raw mut release,
            );
        }
        [major, minor, patchlevel, release]
    }

    pub(super) struct Loaded {
        pub(super) build: NativeIntegerFormatterBuild,
        interpreter: RefCell<*mut libc::c_void>,
        delete: DeleteInterp,
        long: NewLong,
        wide: NewWide,
        set_result: SetResult,
        get_string: GetString,
        _library: Library,
        _file: File,
        // Rc marker prevents Send/Sync across native Tcl thread boundaries.
        _thread: std::marker::PhantomData<Rc<()>>,
    }

    impl Loaded {
        pub(super) fn load(
            path: &Path,
            expected: [u8; 32],
            executable: &Path,
        ) -> Result<Self, NativeIntegerFormatterUnavailable> {
            let mut file =
                File::open(path).map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
            if digest(&mut file)? != expected {
                return Err(NativeIntegerFormatterUnavailable::Build);
            }
            let pinned = pin_verified_image(&file)?;
            let executable = CString::new(executable.as_os_str().as_bytes())
                .map_err(|_| NativeIntegerFormatterUnavailable::Build)?;
            // SAFETY: pinned refers to a live verified file; RTLD_LOCAL keeps
            // its symbols private and DEEPBIND prevents another loaded Tcl
            // from donating same-named internal operations to this image.
            // NODELETE keeps registered Tcl process/thread callbacks mapped
            // after this provider's interpreter/handle retires. This boundary
            // does not claim Tcl_Finalize or native global-state teardown.
            let handle = unsafe {
                libc::dlopen(
                    pinned.as_ptr(),
                    libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_DEEPBIND | libc::RTLD_NODELETE,
                )
            };
            if handle.is_null() {
                return Err(NativeIntegerFormatterUnavailable::Build);
            }
            let library = Library(handle);
            let mut base = None;
            // SAFETY: each symbol is checked to belong to the same image, and
            // these signatures are the public Tcl 8.4 ABI. Version is checked
            // before invoking any object constructor or interpreter function.
            let (version, find, create, delete, long, wide, set_result, get_string) = unsafe {
                (
                    std::mem::transmute::<*mut libc::c_void, Version>(symbol(
                        &library,
                        c"Tcl_GetVersion",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, FindExecutable>(symbol(
                        &library,
                        c"Tcl_FindExecutable",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, CreateInterp>(symbol(
                        &library,
                        c"Tcl_CreateInterp",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, DeleteInterp>(symbol(
                        &library,
                        c"Tcl_DeleteInterp",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, NewLong>(symbol(
                        &library,
                        c"Tcl_NewLongObj",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, NewWide>(symbol(
                        &library,
                        c"Tcl_NewWideIntObj",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, SetResult>(symbol(
                        &library,
                        c"Tcl_SetObjResult",
                        &mut base,
                        &file,
                    )?),
                    std::mem::transmute::<*mut libc::c_void, GetString>(symbol(
                        &library,
                        c"Tcl_GetStringFromObj",
                        &mut base,
                        &file,
                    )?),
                )
            };
            let actual = native_version(version);
            if actual[0..2] != [8, 4] || digest(&mut file)? != expected {
                return Err(NativeIntegerFormatterUnavailable::Build);
            }
            // SAFETY: FindExecutable initializes the verified native image;
            // CreateInterp returns its actual interpreter ownership handle.
            let interpreter = unsafe {
                find(executable.as_ptr());
                create()
            };
            if interpreter.is_null() {
                return Err(NativeIntegerFormatterUnavailable::Operation);
            }
            Ok(Self {
                build: NativeIntegerFormatterBuild {
                    version: actual,
                    sha256: expected,
                    long_bits: libc::c_long::BITS,
                },
                interpreter: RefCell::new(interpreter),
                delete,
                long,
                wide,
                set_result,
                get_string,
                _library: library,
                _file: file,
                _thread: std::marker::PhantomData,
            })
        }

        pub(super) fn format(
            &self,
            kind: NativeIntegerKind,
            value: i64,
        ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
            let interpreter = self
                .interpreter
                .try_borrow_mut()
                .map_err(|_| NativeIntegerFormatterUnavailable::Operation)?;
            // SAFETY: thread-confined interpreter and verified image are live.
            // SetObjResult takes native ownership before the updater is called.
            // The result remains alive until the next call or interpreter drop.
            unsafe {
                let object = match kind {
                    NativeIntegerKind::Long => (self.long)(
                        libc::c_long::try_from(value)
                            .map_err(|_| NativeIntegerFormatterUnavailable::Operation)?,
                    ),
                    NativeIntegerKind::Wide => (self.wide)(value),
                };
                if object.is_null() {
                    return Err(NativeIntegerFormatterUnavailable::Operation);
                }
                (self.set_result)(*interpreter, object);
                let mut length = 0;
                let bytes = (self.get_string)(object, &raw mut length);
                let length = usize::try_from(length)
                    .map_err(|_| NativeIntegerFormatterUnavailable::Operation)?;
                if bytes.is_null() {
                    return Err(NativeIntegerFormatterUnavailable::Operation);
                }
                Ok(std::slice::from_raw_parts(bytes.cast::<u8>(), length).to_vec())
            }
        }
    }

    impl Drop for Loaded {
        fn drop(&mut self) {
            // SAFETY: last capability owner retires the native result and
            // interpreter before the live-library field is released.
            unsafe { (self.delete)(*self.interpreter.get_mut()) };
        }
    }
}
