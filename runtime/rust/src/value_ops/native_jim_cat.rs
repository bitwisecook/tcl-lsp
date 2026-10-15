// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A completed actual Jim cat construction, borrowed until its result transfers.

use super::*;

pub(crate) struct OriginalJimStringCatConstruction<'a> {
    inputs: &'a [obj::NativeObjectLifetime],
    output: obj::NativeObjectLifetime,
    source: Option<crate::interp::OriginalSourceConstruction>,
}

impl<'a> OriginalJimStringCatConstruction<'a> {
    pub(super) fn completed(
        inputs: &'a [obj::NativeObjectLifetime],
        output: *mut TclObj,
        source: Option<crate::interp::OriginalSourceConstruction>,
    ) -> Self {
        Self {
            inputs,
            output: obj::NativeObjectLifetime::retain(output),
            source,
        }
    }

    pub(crate) fn checked_parts(
        &self,
        actual: &Interp,
    ) -> Result<
        (
            Vec<*mut TclObj>,
            *mut TclObj,
            Option<crate::interp::OriginalSourceConstruction>,
        ),
        ValueError,
    > {
        if let Some(source) = &self.source {
            source.check(actual)?;
        }
        let inputs = self
            .inputs
            .iter()
            .map(obj::NativeObjectLifetime::checked_ptr)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((inputs, self.output.checked_ptr()?, self.source.clone()))
    }
}
