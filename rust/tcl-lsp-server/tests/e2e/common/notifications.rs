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

//! The buffered notifications and server-initiated requests: log barriers, cursors and snapshots.

use super::*;

impl Lsp {
    /// A marker into the notification log for `await_log(..., since)`.
    pub fn notification_cursor(&self) -> usize {
        self.shared.notifications.lock().unwrap().len()
    }

    /// Block until a `window/logMessage` whose text contains all `needles`
    /// (searching entries at index `since` onward). Returns the message text.
    pub fn await_log(&self, needles: &[&str], timeout: Duration, since: usize) -> String {
        self.try_await_log(needles, timeout, since)
            .unwrap_or_else(|| {
                panic!(
                    "no window/logMessage containing all of {needles:?} within {timeout:?}{}\n{}",
                    latency_barrier_timeout_note(),
                    self.log_barrier_failure_context(since)
                )
            })
    }

    /// Bounded captured evidence for a failed log barrier, without another request.
    fn log_barrier_failure_context(&self, since: usize) -> String {
        const LIMIT: usize = 16_384;
        let notes = self.shared.notifications.lock().unwrap();
        let mut recent = String::new();
        for note in notes.iter().skip(since).rev().take(16).rev() {
            let text = note.to_string();
            for character in text.chars().take(1_024) {
                recent.push(character);
            }
            recent.push('\n');
        }
        drop(notes);
        let stderr = self.shared.stderr.lock().unwrap();
        let mut start = stderr.len().saturating_sub(LIMIT);
        while !stderr.is_char_boundary(start) {
            start += 1;
        }
        format!(
            "Recent captured notifications (at most16,1024 chars each):\n{recent}Server stderr tail (at most {LIMIT} bytes):\n{}",
            &stderr[start..]
        )
    }

    /// [`Lsp::await_log`] for a `window/logMessage` of type `Warning` alone: a
    /// message the server logs at another severity does not satisfy it.
    pub fn await_warning_log(&self, needles: &[&str], timeout: Duration, since: usize) -> String {
        self.try_await_log_of_type(needles, Some(LOG_MESSAGE_WARNING), timeout, since)
            .unwrap_or_else(|| {
                panic!(
                    "no warning window/logMessage containing all of {needles:?} within \
                     {timeout:?}{}",
                    latency_barrier_timeout_note()
                )
            })
    }

    /// Like [`Lsp::await_log`] but returns `None` on timeout instead of
    /// panicking, for callers that can make progress another way when the
    /// marker does not arrive — e.g. a convergence loop that simply re-issues
    /// its request rather than failing on a round the server never settled.
    pub fn try_await_log(
        &self,
        needles: &[&str],
        timeout: Duration,
        since: usize,
    ) -> Option<String> {
        self.try_await_log_of_type(needles, None, timeout, since)
    }

    /// [`Lsp::try_await_log`], limited to log messages of `message_type` when
    /// one is given.
    fn try_await_log_of_type(
        &self,
        needles: &[&str],
        message_type: Option<i64>,
        timeout: Duration,
        since: usize,
    ) -> Option<String> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        self.try_await_log_of_type_before(needles, message_type, deadline, since)
    }

    pub(super) fn try_await_log_of_type_before(
        &self,
        needles: &[&str],
        message_type: Option<i64>,
        deadline: Instant,
        since: usize,
    ) -> Option<String> {
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            for note in notes.iter().skip(since) {
                if note.get("method").and_then(Value::as_str) != Some("window/logMessage") {
                    continue;
                }
                let logged_type = note
                    .get("params")
                    .and_then(|p| p.get("type"))
                    .and_then(Value::as_i64);
                if message_type.is_some() && logged_type != message_type {
                    continue;
                }
                let msg = note
                    .get("params")
                    .and_then(|p| p.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if needles.iter().all(|n| msg.contains(n)) {
                    return Some(msg.to_owned());
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a notification with `method` arrives; return it.
    pub fn await_notification(&self, method: &str, timeout: Duration) -> Value {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            if let Some(note) = notes
                .iter()
                .find(|n| n.get("method").and_then(Value::as_str) == Some(method))
            {
                return note.clone();
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Release the notifications lock before the (sleeping) probe so
                // the reader thread isn't blocked while we diagnose the timeout.
                drop(notes);
                panic!(
                    "no {method:?} notification within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a server-initiated *request* (has an `id`, unlike a
    /// notification — e.g. `workspace/semanticTokens/refresh`) with `method`
    /// arrives (searching entries at index `since` onward); return it. The
    /// harness auto-replies to it regardless (see `auto_reply`), so this only
    /// observes that the server asked, without blocking that reply.
    pub fn await_server_request(&self, method: &str, timeout: Duration, since: usize) -> Value {
        self.try_await_server_request(method, timeout, since)
            .unwrap_or_else(|| panic!("no server-initiated {method:?} request within {timeout:?}"))
    }

    /// Like [`Lsp::await_server_request`] but returns `None` on timeout instead
    /// of panicking, for callers that treat "no such request arrived" as a
    /// normal, expected outcome rather than a failure — e.g. converging
    /// semantic tokens, where the *absence* of a `workspace/semanticTokens/refresh`
    /// means the first response was already the settled enriched stream.
    pub fn try_await_server_request(
        &self,
        method: &str,
        timeout: Duration,
        since: usize,
    ) -> Option<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        self.try_await_server_request_before(method, deadline, since)
    }

    pub(super) fn try_await_server_request_before(
        &self,
        method: &str,
        deadline: Instant,
        since: usize,
    ) -> Option<Value> {
        let mut reqs = self.shared.server_requests.lock().unwrap();
        loop {
            if let Some(req) = reqs
                .iter()
                .skip(since)
                .find(|n| n.get("method").and_then(Value::as_str) == Some(method))
            {
                return Some(req.clone());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (guard, _) = self
                .shared
                .requests_cv
                .wait_timeout(reqs, remaining)
                .unwrap();
            reqs = guard;
        }
    }

    /// A marker into the server-request log for `await_server_request(...,
    /// since)`, mirroring [`Lsp::notification_cursor`].
    pub fn server_request_cursor(&self) -> usize {
        self.shared.server_requests.lock().unwrap().len()
    }

    /// A snapshot of all buffered server-initiated requests.
    pub fn server_requests(&self) -> Vec<Value> {
        self.shared.server_requests.lock().unwrap().clone()
    }

    /// Drop buffered notifications so a later `await_*` only sees fresh ones.
    pub fn clear_notifications(&self) {
        self.shared.notifications.lock().unwrap().clear();
    }

    /// A snapshot of all buffered notifications.
    pub fn notifications(&self) -> Vec<Value> {
        self.shared.notifications.lock().unwrap().clone()
    }

    pub fn stderr_text(&self) -> String {
        self.shared.stderr.lock().unwrap().clone()
    }
}
