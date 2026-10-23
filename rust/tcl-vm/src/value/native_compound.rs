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
