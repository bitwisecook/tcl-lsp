// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original `TclOO` method-name primary owning its actual shared call chain.
use super::{IntRep, Rc, Value};
use crate::cmd_oo::native_method_cache::NativeMethodChain;
impl Value {
    pub(crate) fn native_method_name_chain(&self) -> Option<Rc<NativeMethodChain>> {
        if let IntRep::NativeMethodName(chain) = &*self.0.intrep.borrow() {
            Some(Rc::clone(chain))
        } else {
            None
        }
    }
    pub(crate) fn clear_native_method_name(&self) {
        if matches!(&*self.0.intrep.borrow(), IntRep::NativeMethodName(_)) {
            let retired = std::mem::replace(&mut *self.0.intrep.borrow_mut(), IntRep::Str);
            drop(retired);
        }
    }
    pub(crate) fn retain_native_method_name(
        &self,
        chain: Rc<NativeMethodChain>,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut impl FnMut() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        check()?;
        self.check_native_header()?;
        let reached = self
            .native_string_bytes(protocol)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable);
        check()?;
        reached?;
        let retired = std::mem::replace(&mut *self.0.intrep.borrow_mut(), IntRep::Str);
        drop(retired);
        check()?;
        self.check_native_header()?;
        *self.0.intrep.borrow_mut() = IntRep::NativeMethodName(chain);
        Ok(())
    }
}
