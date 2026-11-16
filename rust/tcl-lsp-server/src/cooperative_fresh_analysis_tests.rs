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

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[tokio::test]
async fn an_interrupted_fresh_seed_is_not_returned_or_published_after_stop() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let signal = Arc::clone(&entered);
    let cooperation = Arc::new(analysis_cooperation::AnalysisCooperation::new(move || {
        signal.notify_one();
        true
    }));
    let backend = tests::test_backend().with_analysis_cooperation(Arc::clone(&cooperation));
    let uri: Uri = "file:///interrupted-analysis.tcl".parse().unwrap();
    let source: Arc<str> = Arc::from("proc p {x} {return $x}\np 1\n");
    let pending = backend.fresh_analysis_seed(&uri, source, "tcl8.6".to_owned());
    tokio::pin!(pending);
    tokio::select! {
        _ = pending.as_mut() => panic!("interrupted analysis returned a result"),
        () = entered.notified() => {}
    }
    cooperation.stop();
    assert!(pending.as_mut().now_or_never().is_none());
    assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 0);
    assert!(backend.pull_diag_cache.lock().await.is_empty());
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str())
    );
}

#[tokio::test]
async fn drained_incomplete_input_retries_and_quiet_analysis_completes_with_its_full_result() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let ready = Arc::new(AtomicBool::new(true));
    let probes = Arc::new(AtomicUsize::new(0));
    let signal = Arc::clone(&entered);
    let queued = Arc::clone(&ready);
    let counted = Arc::clone(&probes);
    let cooperation = Arc::new(analysis_cooperation::AnalysisCooperation::new(move || {
        counted.fetch_add(1, Ordering::Relaxed);
        let ready = queued.load(Ordering::Acquire);
        if ready {
            signal.notify_one();
        }
        ready
    }));
    let backend = tests::test_backend().with_analysis_cooperation(Arc::clone(&cooperation));
    let uri: Uri = "file:///resumed-analysis.tcl".parse().unwrap();
    let source: Arc<str> = Arc::from("proc quiet_complete {x} {return $x}\nquiet_complete 1\n");
    let pending = backend.fresh_analysis_seed(&uri, Arc::clone(&source), "tcl8.6".to_owned());
    tokio::pin!(pending);
    tokio::select! {
        _ = pending.as_mut() => panic!("analysis ignored ready input"),
        () = entered.notified() => {}
    }
    assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 0);
    ready.store(false, Ordering::Release);
    cooperation.drained_incomplete();
    let seed = pending.await.unwrap();
    assert!(tcl_compiler::source_graph::current_analysis(&source, &seed.analysis).is_some());
    assert!(seed.analysis.all_procs.contains_key("::quiet_complete"));
    assert!(probes.load(Ordering::Relaxed) > 1);
    let cached = backend
        .fresh_analysis_seed(&uri, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&seed.analysis, &cached.analysis));
    assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 1);
}
