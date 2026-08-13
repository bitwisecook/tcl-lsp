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

//! Prove salsa early-cutoff + cascade breadth on a minimal item
//! graph. A *body* edit must re-execute exactly one `item_analysis` (O(1)); a
//! *signature* edit must re-execute the cross-item aggregate but not unrelated
//! item bodies. The salsa event callback counts `WillExecute` per query.

use salsa::Setter as _;
use std::sync::{Arc, Mutex};

#[salsa::db]
#[derive(Clone)]
struct ProbeDb {
    storage: salsa::Storage<Self>,
    log: Arc<Mutex<Vec<String>>>,
}

#[salsa::db]
impl salsa::Database for ProbeDb {}

impl ProbeDb {
    fn new() -> Self {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let storage = salsa::Storage::new(Some(Box::new(move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                sink.lock().unwrap().push(format!("{database_key:?}"));
            }
        })));
        Self { storage, log }
    }
    fn drain(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }
}

#[salsa::input]
struct Item {
    #[returns(ref)]
    body: String,
    #[returns(copy)]
    sig: u32,
}

#[salsa::tracked(returns(copy))]
fn item_analysis(db: &dyn salsa::Database, item: Item) -> usize {
    item.body(db).len()
}

#[salsa::tracked(returns(copy))]
fn file_diag(db: &dyn salsa::Database, a: Item, b: Item) -> usize {
    item_analysis(db, a) + item_analysis(db, b) + a.sig(db) as usize + b.sig(db) as usize
}

fn count(log: &[String], needle: &str) -> usize {
    log.iter().filter(|s| s.contains(needle)).count()
}

#[test]
fn early_cutoff_and_cascade_breadth() {
    let mut db = ProbeDb::new();
    let a = Item::new(&db, "proc a {} { aaa }".to_owned(), 0);
    let b = Item::new(&db, "proc b {} { bbb }".to_owned(), 1);

    let _ = file_diag(&db, a, b);
    let init = db.drain();
    assert_eq!(
        count(&init, "item_analysis"),
        2,
        "initial: both items: {init:?}"
    );
    assert_eq!(count(&init, "file_diag"), 1, "initial: aggregate");

    // BODY edit to `a`: only item_analysis(a) + aggregate re-run; b reused.
    a.set_body(&mut db)
        .to("proc a {} { CHANGED body }".to_owned());
    let _ = file_diag(&db, a, b);
    let after_body = db.drain();
    assert_eq!(
        count(&after_body, "item_analysis"),
        1,
        "body edit -> ONE item: {after_body:?}"
    );
    assert_eq!(count(&after_body, "file_diag"), 1, "body edit -> aggregate");

    // No-op demand: nothing reruns.
    let _ = file_diag(&db, a, b);
    assert_eq!(
        count(&db.drain(), "item_analysis"),
        0,
        "no-op demand reruns nothing"
    );

    // SIGNATURE edit: aggregate re-runs (reads a.sig), no item body re-runs.
    a.set_sig(&mut db).to(99);
    let _ = file_diag(&db, a, b);
    let after_sig = db.drain();
    assert_eq!(
        count(&after_sig, "item_analysis"),
        0,
        "sig edit -> NO body: {after_sig:?}"
    );
    assert_eq!(
        count(&after_sig, "file_diag"),
        1,
        "sig edit -> aggregate cascade"
    );

    // TRACKED-OUTPUT early cutoff: change `a`'s body to DIFFERENT content of the
    // SAME length. item_analysis(a) re-executes (its body input changed) but
    // returns the same value (len), and a.sig is unchanged — so file_diag's
    // inputs are all value-unchanged and it CUTS OFF (does not re-execute).
    // (salsa input setters always bump the revision, so the cutoff is on the
    // tracked *output* value, not the input — hence the per-item build must only
    // set an item's body input when the item-tree diff says it actually changed.)
    let cur = item_analysis(&db, a); // = current body len
    let same_len: String = "z".repeat(cur);
    a.set_body(&mut db).to(same_len);
    let _ = file_diag(&db, a, b);
    let after_eqlen = db.drain();
    assert_eq!(
        count(&after_eqlen, "item_analysis"),
        1,
        "same-len body re-runs the item: {after_eqlen:?}"
    );
    assert_eq!(
        count(&after_eqlen, "file_diag"),
        0,
        "unchanged item output cuts off the aggregate: {after_eqlen:?}"
    );
}
