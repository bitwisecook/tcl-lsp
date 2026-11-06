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

//! Allocation-only backing views for the original compound string updater.

use super::Value;
use std::rc::Rc;

pub(super) enum CompoundStringBacking {
    List(crate::NativeListItems),
    Dictionary(Rc<Vec<(Value, Value)>>),
}
impl CompoundStringBacking {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::List(items) => items.len(),
            Self::Dictionary(pairs) => pairs.len() * 2,
        }
    }
    pub(super) fn member(&self, index: usize) -> &Value {
        match self {
            Self::List(items) => &items[index],
            Self::Dictionary(pairs) => {
                let (key, value) = &pairs[index / 2];
                if index.is_multiple_of(2) { key } else { value }
            }
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum CompoundStringNode {
    Root,
    Member { backing: usize, index: usize },
}
impl CompoundStringNode {
    pub(super) fn value<'a>(
        self,
        root: &'a Value,
        backings: &'a [CompoundStringBacking],
    ) -> &'a Value {
        match self {
            Self::Root => root,
            Self::Member { backing, index } => backings[backing].member(index),
        }
    }
}
