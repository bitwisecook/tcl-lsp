// SPDX-License-Identifier: AGPL-3.0-or-later
//! Supplied software Host stages, independent of external encoding observations.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_platform::{
    EncodingCatalogue, EncodingCatalogueUnavailable, Filesystem, Host, HostError, SystemEncoding,
};

/// Recorded software stage callbacks and outcomes for original adapter controls.
#[derive(Default)]
pub struct EncodingStages {
    /// Actual reached supplied stage names in order.
    pub events: RefCell<Vec<&'static str>>,
    /// Optional same-object/currency withdrawal after the real lookup stage.
    pub after_lookup: RefCell<Option<Box<dyn FnMut()>>>,
    /// Optional same-object/currency withdrawal after the real file read stage.
    pub after_read: RefCell<Option<Box<dyn FnMut()>>>,
    /// Optional same-object/currency withdrawal during mandatory close.
    pub after_close: RefCell<Option<Box<dyn FnMut()>>>,
    /// Optional software read failure, reached only after successful open/configuration.
    pub read_error: RefCell<Option<HostError>>,
    /// Whether the Host actually supplies catalogue knowledge.
    pub catalogue_available: Cell<bool>,
}

/// Thin supplied software capability adapter; native worker interfaces delegate.
pub struct EncodingHost {
    actual: Rc<dyn Host>,
    stages: Rc<EncodingStages>,
    default: SystemEncoding,
    script: Rc<Vec<u8>>,
}
impl EncodingHost {
    /// Supply an exact default and one byte-addressed file at `/source.tcl`.
    /// Its unknown-name catalogue deliberately establishes real software misses.
    #[must_use]
    pub fn new(
        actual: Rc<dyn Host>,
        stages: Rc<EncodingStages>,
        default: SystemEncoding,
        script: Vec<u8>,
    ) -> Self {
        stages.catalogue_available.set(true);
        Self {
            actual,
            stages,
            default,
            script: Rc::new(script),
        }
    }
}
impl Host for EncodingHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual
            .capabilities()
            .union(tcl_platform::Capabilities::FILESYSTEM)
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.stages.events.borrow_mut().push("formatter");
        self.actual.native_integer_formatter()
    }
    fn system_encoding(&self) -> SystemEncoding {
        self.default
    }
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.stages.events.borrow_mut().push("filesystem");
        Some(self)
    }
    fn encoding_catalogue(&self) -> Option<&dyn EncodingCatalogue> {
        self.stages.events.borrow_mut().push("catalogue");
        self.stages.catalogue_available.get().then_some(self)
    }
}
impl EncodingCatalogue for EncodingHost {
    fn lookup(&self, _name: &[u8]) -> Result<Option<SystemEncoding>, EncodingCatalogueUnavailable> {
        self.stages.events.borrow_mut().push("lookup");
        if let Some(callback) = &mut *self.stages.after_lookup.borrow_mut() {
            callback();
        }
        Ok(None)
    }
}
struct EncodingSourceFile {
    stages: Rc<EncodingStages>,
    script: Rc<Vec<u8>>,
}
impl tcl_platform::SourceFile for EncodingSourceFile {
    fn read_all(&mut self) -> Result<Vec<u8>, HostError> {
        self.stages.events.borrow_mut().push("read");
        let read = self
            .stages
            .read_error
            .borrow()
            .clone()
            .map_or_else(|| Ok(self.script.as_ref().clone()), Err);
        if let Some(callback) = &mut *self.stages.after_read.borrow_mut() {
            callback();
        }
        read
    }
    fn close(self: Box<Self>) -> Result<(), HostError> {
        self.stages.events.borrow_mut().push("close");
        if let Some(callback) = &mut *self.stages.after_close.borrow_mut() {
            callback();
        }
        Ok(())
    }
}

impl Filesystem for EncodingHost {
    fn exists(&self, path: &str) -> bool {
        path == "/source.tcl"
    }
    fn metadata(&self, _path: &str) -> Result<tcl_platform::Metadata, HostError> {
        Err(HostError::Unsupported)
    }
    fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
        self.read_bytes(path.as_bytes())
    }
    fn read_bytes(&self, path: &[u8]) -> Result<Vec<u8>, HostError> {
        self.stages.events.borrow_mut().push("read");
        let read = if path == b"/source.tcl" {
            Ok(self.script.as_ref().clone())
        } else {
            Err(HostError::NotFound)
        };
        if let Some(callback) = &mut *self.stages.after_read.borrow_mut() {
            callback();
        }
        read
    }
    fn open_source_bytes(
        &self,
        path: &[u8],
    ) -> Result<Box<dyn tcl_platform::SourceFile>, HostError> {
        self.stages.events.borrow_mut().push("open");
        if path != b"/source.tcl" {
            return Err(HostError::NotFound);
        }
        Ok(Box::new(EncodingSourceFile {
            stages: Rc::clone(&self.stages),
            script: Rc::clone(&self.script),
        }))
    }
    fn write(&self, _path: &str, _data: &[u8]) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    fn read_dir(&self, _path: &str) -> Result<Vec<String>, HostError> {
        Err(HostError::Unsupported)
    }
    fn create_dir_all(&self, _path: &str) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    fn remove(&self, _path: &str, _recursive: bool) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}
