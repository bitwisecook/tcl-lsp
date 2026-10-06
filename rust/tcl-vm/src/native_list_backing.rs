// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed native List storage: header ownership and lifetime leases are distinct.

use crate::Value;
use std::{cell::Cell, ops::Deref, rc::Rc};

struct ListStorage {
    items: Rc<Vec<Value>>,
    owns_members: bool,
    canonical: Rc<Cell<bool>>,
    headers: Cell<usize>,
}

impl Drop for ListStorage {
    fn drop(&mut self) {
        if self.headers.get() == 0 && self.owns_members {
            for value in self.items.iter() {
                value.release_native_lifetime_pin();
            }
        }
    }
}

/// Original members and live canonical state retained without copying children.
/// A lifetime view is not a native header and grants no native ABI authority.
pub struct NativeListItems {
    storage: Rc<ListStorage>,
    header: bool,
}

impl NativeListItems {
    pub(crate) fn new(items: Vec<Value>, canonical: bool) -> Self {
        Self {
            storage: Rc::new(ListStorage {
                items: Rc::new(
                    items
                        .into_iter()
                        .map(Value::into_native_reference)
                        .collect(),
                ),
                owns_members: true,
                canonical: Rc::new(Cell::new(canonical)),
                headers: Cell::new(1),
            }),
            header: true,
        }
    }

    pub(crate) fn invocation_view(items: Rc<Vec<Value>>) -> Self {
        Self {
            storage: Rc::new(ListStorage {
                items,
                owns_members: false,
                canonical: Rc::new(Cell::new(false)),
                headers: Cell::new(1),
            }),
            header: true,
        }
    }

    /// Whether this carrier owns an authenticated whole member table.
    #[must_use]
    pub fn is_whole_backing(&self) -> bool {
        self.storage.owns_members
    }

    /// Borrow exact original storage without introducing native ownership.
    #[must_use]
    pub fn lifetime_view(&self) -> Self {
        Self {
            storage: Rc::clone(&self.storage),
            header: false,
        }
    }

    pub(crate) fn native_header(&self) -> Result<Self, tcl_syntax::value::ValueError> {
        let count = self.storage.headers.get();
        if count == 0 {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native List backing",
            ));
        }
        self.storage
            .headers
            .set(count.checked_add(1).expect("native List owners exhausted"));
        Ok(Self {
            storage: Rc::clone(&self.storage),
            header: true,
        })
    }

    /// Borrow retained members only while their original native headers remain live.
    pub fn elements(&self) -> Result<&[Value], tcl_syntax::value::ValueError> {
        for value in self.storage.items.iter() {
            value.check_native_header()?;
        }
        Ok(self.storage.items.as_slice())
    }

    /// Physical native header owners, excluding lifetime-only inspection views.
    #[cfg(test)]
    pub(crate) fn native_header_reference_count(&self) -> usize {
        self.storage.headers.get()
    }

    /// Whether more than one actual List header retains this whole backing.
    #[must_use]
    pub fn native_is_shared(&self) -> bool {
        self.storage.headers.get() > 1
    }

    /// Whether genuine native headers still retain this original storage.
    #[must_use]
    pub fn has_native_header(&self) -> bool {
        self.storage.headers.get() != 0
    }

    /// Current canonical flag of this same original backing.
    #[must_use]
    pub fn canonical(&self) -> bool {
        self.storage.canonical.get()
    }

    /// Live flag transport, independent from string storage authority.
    #[must_use]
    pub fn canonical_state(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.storage.canonical)
    }

    /// Exact backing identity comparison; names and member equality grant none.
    #[must_use]
    pub fn same_backing(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.storage, &other.storage)
    }
}

impl Clone for NativeListItems {
    fn clone(&self) -> Self {
        if self.header {
            self.native_header().expect("live native List header")
        } else {
            self.lifetime_view()
        }
    }
}

impl Drop for NativeListItems {
    fn drop(&mut self) {
        if self.header {
            let count = self
                .storage
                .headers
                .get()
                .checked_sub(1)
                .expect("native List owner underflow");
            self.storage.headers.set(count);
            if count == 0 && self.storage.owns_members {
                for value in self.storage.items.iter() {
                    value.retain_native_lifetime_pin();
                    value.retire_unowned_native_header();
                }
            }
        }
    }
}

impl Deref for NativeListItems {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.storage.items
    }
}

impl AsRef<Vec<Value>> for NativeListItems {
    fn as_ref(&self) -> &Vec<Value> {
        &self.storage.items
    }
}

impl std::fmt::Debug for NativeListItems {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeListItems")
            .field("len", &self.len())
            .field("canonical", &self.canonical())
            .field("header", &self.header)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::NativeListItems;
    use crate::Value;

    #[test]
    fn lifetime_views_do_not_own_native_headers_or_member_references() {
        let child = Value::string("member");
        let header = NativeListItems::new(vec![child.clone()], false);
        assert_eq!(child.native_object_reference_count(), 2);
        let view = header.lifetime_view();
        let repeated = view.clone();
        assert!(view.same_backing(&repeated));
        assert!(!view.native_is_shared());
        assert_eq!(child.native_object_reference_count(), 2);
        let duplicate = header.clone();
        assert!(view.native_is_shared());
        assert_eq!(child.native_object_reference_count(), 2);
        drop(header);
        assert!(!view.native_is_shared());
        drop(duplicate);
        assert!(!view.has_native_header());
        assert_eq!(child.native_object_reference_count(), 1);
        assert_eq!(
            view[0].native_object_identity(),
            child.native_object_identity()
        );
        drop(view);
        drop(repeated);
        assert_eq!(child.native_object_reference_count(), 1);
    }

    #[test]
    fn repeated_member_slots_each_own_one_native_reference() {
        let child = Value::string("member");
        let header = NativeListItems::new(vec![child.clone(), child.clone()], false);
        assert_eq!(child.native_object_reference_count(), 3);
        let view = header.lifetime_view();
        drop(header);
        assert_eq!(child.native_object_reference_count(), 1);
        drop(view);
        assert_eq!(child.native_object_reference_count(), 1);
    }
}
