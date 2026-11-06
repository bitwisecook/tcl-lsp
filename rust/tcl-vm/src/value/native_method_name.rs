// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
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
    pub(crate) fn retain_native_method_name<C>(
        &self,
        chain: Rc<NativeMethodChain>,
        context: &mut C,
        string: &mut impl FnMut(&mut C) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>,
        check: &mut impl FnMut(&mut C) -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        check(context)?;
        self.check_native_header()?;
        let reached = string(context);
        check(context)?;
        reached?;
        let retired = std::mem::replace(&mut *self.0.intrep.borrow_mut(), IntRep::Str);
        drop(retired);
        check(context)?;
        self.check_native_header()?;
        *self.0.intrep.borrow_mut() = IntRep::NativeMethodName(chain);
        Ok(())
    }
}
