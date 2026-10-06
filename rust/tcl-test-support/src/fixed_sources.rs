//! Read-only fixed source files at the original paths recorded by native probes.

use std::rc::Rc;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, HostError, Metadata, StdIo};

/// Overlay recorded source contents without changing their original path bytes.
/// Other host capabilities retain the caller's actual provider.
pub struct FixedSourceHost {
    base: Rc<dyn Host>,
    files: &'static [(&'static str, &'static [u8])],
}
impl FixedSourceHost {
    #[must_use]
    pub fn new(base: Rc<dyn Host>, files: &'static [(&'static str, &'static [u8])]) -> Self {
        Self { base, files }
    }
}
impl Host for FixedSourceHost {
    fn capabilities(&self) -> Capabilities {
        self.base.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.base.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.base.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.base.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.base.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.base.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.base.system_encoding()
    }
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        Some(self)
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.base.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.base.process()
    }
}
impl Filesystem for FixedSourceHost {
    fn exists(&self, path: &str) -> bool {
        self.files.iter().any(|(name, _)| *name == path)
            || self.base.filesystem().is_some_and(|fs| fs.exists(path))
    }
    fn metadata(&self, path: &str) -> Result<Metadata, HostError> {
        self.base
            .filesystem()
            .ok_or(HostError::Unsupported)?
            .metadata(path)
    }
    fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
        self.read_bytes(path.as_bytes())
    }
    fn read_bytes(&self, path: &[u8]) -> Result<Vec<u8>, HostError> {
        if let Some((_, source)) = self.files.iter().find(|(name, _)| name.as_bytes() == path) {
            return Ok(source.to_vec());
        }
        self.base
            .filesystem()
            .ok_or(HostError::Unsupported)?
            .read_bytes(path)
    }
    fn write(&self, _path: &str, _data: &[u8]) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    fn read_dir(&self, path: &str) -> Result<Vec<String>, HostError> {
        self.base
            .filesystem()
            .ok_or(HostError::Unsupported)?
            .read_dir(path)
    }
    fn create_dir_all(&self, _path: &str) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    fn remove(&self, _path: &str, _recursive: bool) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}
