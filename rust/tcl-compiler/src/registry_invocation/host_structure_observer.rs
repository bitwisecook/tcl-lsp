//! WORK-only bounded source-structure observation, never query authority.
use std::cell::RefCell;

thread_local! {
    static TRACE: RefCell<Option<Vec<(&'static str, bool)>>> = const { RefCell::new(None) };
}

/// Thread-owned WORK observation scope; no query or execution authority.
#[doc(hidden)]
pub struct HostStructureTraceGuard {
    previous: Option<Vec<(&'static str, bool)>>,
    active: bool,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

/// Begin one explicitly observed, synchronous source-structure selection.
#[doc(hidden)]
#[must_use]
pub fn begin_host_structure_trace() -> HostStructureTraceGuard {
    let active = matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1"));
    let previous = if active {
        TRACE.with(|trace| trace.replace(Some(Vec::new())))
    } else {
        None
    };
    HostStructureTraceGuard {
        previous,
        active,
        _thread: std::marker::PhantomData,
    }
}

impl HostStructureTraceGuard {
    /// Disable this scope before its bounded rows are formatted or published.
    #[must_use]
    pub fn finish(mut self) -> Vec<(&'static str, bool)> {
        if !self.active {
            return Vec::new();
        }
        let rows = TRACE.with(|trace| trace.replace(self.previous.take()).unwrap_or_default());
        self.active = false;
        rows
    }
}

impl Drop for HostStructureTraceGuard {
    fn drop(&mut self) {
        if self.active {
            TRACE.with(|trace| {
                trace.replace(self.previous.take());
            });
        }
    }
}

pub(crate) fn flag(stage: &'static str, value: bool) -> bool {
    // TEMP-ISSUER-TRACE
    if std::env::var_os("TCL_LSP_TEMP_SOURCE_ADVICE_TRACE").is_some() {
        eprintln!("TEMP_ISSUER flag stage={stage} value={value}");
    }
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        TRACE.with(|trace| {
            if let Some(rows) = trace.borrow_mut().as_mut() {
                if rows.len() < 32 {
                    rows.push((stage, value));
                } else {
                    rows[31] = ("truncated", false);
                }
            }
        });
    }
    value
}

pub(super) fn option<T>(stage: &'static str, value: Option<T>) -> Option<T> {
    flag(stage, value.is_some());
    value
}
