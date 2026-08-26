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

//! The one async-runtime seam the server has: task spawning, blocking
//! offload, timers, and the parallelism hint.
//!
//! The server's protocol core (`LspService<Backend>`) is transport-free and
//! runtime-agnostic, but the handler bodies are not: they offload CPU-bound
//! analysis with `spawn_blocking`, detach background work with `spawn`, and
//! race passes against deadlines with `sleep_until`/`timeout`. Every one of
//! those calls means something different where there is exactly one thread and
//! no thread pool to offload to. This module is the single place that
//! difference is expressed, so no call site has to know which target it is
//! being compiled for.
//!
//! There are three arms, and `target_family = "wasm"` does **not** pick one:
//! both wasm32-unknown-unknown and wasm32-wasip1 answer to it, and they need
//! opposite things. The gates below are keyed on `target_os` for that reason.
//!
//! **The native contract is "nothing changed".** On every non-wasm target the
//! items below are plain re-exports of the Tokio ones the call sites used
//! before this module existed — same types, same semantics, same scheduling.
//! `available_parallelism` is the one function with a body, and it is the
//! `std::thread::available_parallelism().map_or(4, …)` expression that used to
//! be written out at each of its three call sites. Adding a wrapper type or a
//! behavioural tweak on the native arm defeats the point of the seam: the
//! native server must not be able to regress because of a browser port.
//!
//! **The browser contract is "same shapes, single thread".** A browser worker
//! has no threads *and* no OS beneath it: no clock to read, and nothing for a
//! timer driver to wait on, so every scheduling primitive is rebuilt on the
//! host's.
//!
//! * [`spawn_blocking`] runs the closure *inline*, right there on the calling
//!   task, and hands back an already-finished [`JoinHandle`]. There is no
//!   worker pool to move it to, and blocking the one thread is what a browser
//!   worker is for. The return type is still `JoinHandle<R>` awaiting to
//!   `Result<R, JoinError>`, so every call site is unchanged — a browser
//!   `spawn_blocking` simply never yields and never fails.
//! * [`spawn`] becomes `wasm_bindgen_futures::spawn_local`, with the task's
//!   output routed back through a oneshot channel so the handle still awaits
//!   to `Result<T, JoinError>`. A task dropped before it completes surfaces as
//!   a cancelled [`JoinError`], matching Tokio.
//! * [`JoinSet`] becomes an ordered-arrival `FuturesUnordered`. Tokio's
//!   `JoinSet` makes progress in the background; ours only advances while
//!   `join_next` is awaited. Every call site in this crate is
//!   spawn-all-then-drain, so the observable result is the same set of
//!   outputs.
//! * [`Instant`] reads the host clock. `std::time::Instant` compiles on
//!   wasm32-unknown-unknown but panics when called, which is why the two
//!   `std::time::Instant::now()` timing sites route through here.
//! * [`sleep`]/[`sleep_until`]/[`timeout`] use `gloo_timers`' `setTimeout`
//!   futures; Tokio's timer wheel needs a driver that nothing spins in a
//!   worker.
//! * [`available_parallelism`] is `1`. It is a browser worker; there is one
//!   thread, and the call sites use it to size concurrency limits.
//!
//! **The wasi contract is "Tokio, minus the thread pool".** wasip1 has no
//! threads either, but it does have an OS: a real monotonic clock, and a timer
//! wheel that runs. So [`spawn`], [`JoinSet`], [`yield_now`], [`Instant`],
//! [`sleep`], [`sleep_until`] and [`timeout`] are Tokio's own, unchanged, and
//! [`available_parallelism`] keeps the native expression (wasip1 answers `1`).
//! [`spawn_blocking`] is the single exception: there is no thread-spawn
//! syscall, so Tokio's own fails at run time rather than at compile time, and
//! the closure runs inline as it does in the browser.
//!
//! Two things a wasi host owes this arm. The runtime has to be
//! `Builder::new_current_thread().enable_time()` — `rt-multi-thread` cannot
//! start a worker (`os error 58`) — and it must always have a task or a timer
//! pending, because wasip1's `std` has no condvar and a runtime that parks
//! with nothing to wake it aborts ("condvar wait not supported") rather than
//! blocking.
//!
//! Reaching for the browser arm on wasi is what the `target_os` split exists
//! to prevent: wasm-bindgen binds JS only under `not(target_os = "wasi")`, so
//! elsewhere its imported statics compile to `panic!`, and a wasi build that
//! took the browser arm linked cleanly and then aborted on its first timer.
//!
//! What deliberately stays on Tokio for every target: `tokio::sync::*`
//! (`Mutex`, `Notify`, `Semaphore`, `oneshot` — pure futures-based sync with
//! no runtime dependency) and the `tokio::select!` / `join!` / `pin!` macros,
//! which are compile-time future combinators. Those work unchanged in a
//! worker.

#[cfg(not(target_family = "wasm"))]
pub use native::{
    AbortHandle, Instant, JoinError, JoinHandle, JoinSet, available_parallelism, sleep,
    sleep_until, spawn, spawn_blocking, timeout, yield_now,
};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use browser::{
    AbortHandle, Instant, JoinError, JoinHandle, JoinSet, available_parallelism, sleep,
    sleep_until, spawn, spawn_blocking, timeout, yield_now,
};

#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub use wasi::{
    AbortHandle, Instant, JoinError, JoinHandle, JoinSet, available_parallelism, sleep,
    sleep_until, spawn, spawn_blocking, timeout, yield_now,
};

/// Native arm: pure Tokio re-exports plus the parallelism hint.
#[cfg(not(target_family = "wasm"))]
mod native {
    pub use tokio::task::{
        AbortHandle, JoinError, JoinHandle, JoinSet, spawn, spawn_blocking, yield_now,
    };
    pub use tokio::time::{Instant, sleep, sleep_until, timeout};

    /// The host's usable parallelism, defaulting to 4 where the platform
    /// cannot report it.
    ///
    /// Callers cap this with their own `.min(…)`; this is only the raw hint.
    pub fn available_parallelism() -> usize {
        std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get)
    }
}

/// Browser arm: single-threaded stand-ins with the same shapes.
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod browser {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use futures_util::FutureExt as _;
    use futures_util::future::BoxFuture;
    use futures_util::stream::{FuturesUnordered, StreamExt as _};

    /// Why a task did not produce a value.
    ///
    /// A browser worker has no unwinding task boundary, so unlike Tokio's this
    /// `JoinError` is only ever a cancellation — a task whose future was
    /// dropped before it resolved. [`Self::is_panic`] therefore always reports
    /// `false`; the panic hook surfaces a real panic on the console instead.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct JoinError {
        _private: (),
    }

    impl JoinError {
        const fn cancelled() -> Self {
            Self { _private: () }
        }

        /// Whether the task ended by panicking. Always `false` in the browser.
        #[must_use]
        pub const fn is_panic(&self) -> bool {
            false
        }

        /// Whether the task was cancelled. Always `true` in the browser, since
        /// cancellation is the only way a task there fails to yield a value.
        #[must_use]
        pub const fn is_cancelled(&self) -> bool {
            true
        }
    }

    impl std::fmt::Display for JoinError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("task was cancelled")
        }
    }

    impl std::error::Error for JoinError {}

    /// A handle to a task started with [`spawn`] or [`spawn_blocking`].
    ///
    /// Awaits to `Result<T, JoinError>`, exactly as Tokio's does.
    #[derive(Debug)]
    pub struct JoinHandle<T> {
        state: HandleState<T>,
        aborted: Arc<AtomicBool>,
    }

    #[derive(Debug)]
    enum HandleState<T> {
        /// [`spawn_blocking`] ran the closure inline; the value is already here.
        Ready(Option<T>),
        /// [`spawn`] detached the future; the value arrives over a channel.
        Pending(tokio::sync::oneshot::Receiver<T>),
    }

    impl<T> JoinHandle<T> {
        /// Request cancellation of the task.
        ///
        /// A [`spawn_blocking`] task has already run by the time a handle
        /// exists, so this is a no-op for those; a [`spawn`] task stops at its
        /// next await point.
        pub fn abort(&self) {
            self.aborted.store(true, Ordering::Relaxed);
        }

        /// A cheap, cloneable cancellation handle for this task.
        #[must_use]
        pub fn abort_handle(&self) -> AbortHandle {
            AbortHandle {
                aborted: Arc::clone(&self.aborted),
            }
        }
    }

    impl<T: Unpin> Future for JoinHandle<T> {
        type Output = Result<T, JoinError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            match &mut this.state {
                HandleState::Ready(value) => {
                    Poll::Ready(value.take().ok_or_else(JoinError::cancelled))
                }
                HandleState::Pending(rx) => match Pin::new(rx).poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(value)) => Poll::Ready(Ok(value)),
                    Poll::Ready(Err(_)) => Poll::Ready(Err(JoinError::cancelled())),
                },
            }
        }
    }

    /// A cancellation handle detached from its [`JoinHandle`].
    #[derive(Debug, Clone)]
    pub struct AbortHandle {
        aborted: Arc<AtomicBool>,
    }

    impl AbortHandle {
        /// Request cancellation of the task this handle refers to.
        pub fn abort(&self) {
            self.aborted.store(true, Ordering::Relaxed);
        }
    }

    /// Run a blocking closure. In the browser it runs inline, on the calling
    /// task.
    ///
    /// The bounds match Tokio's so no call site needs to change, even though
    /// `Send` is meaningless on a single-threaded target.
    pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        JoinHandle {
            state: HandleState::Ready(Some(f())),
            aborted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Detach a future onto the worker's microtask queue.
    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let aborted = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&aborted);
        wasm_bindgen_futures::spawn_local(async move {
            let value = Abortable {
                inner: Box::pin(future),
                aborted: flag,
            }
            .await;
            if let Some(value) = value {
                let _ = tx.send(value);
            }
        });
        JoinHandle {
            state: HandleState::Pending(rx),
            aborted,
        }
    }

    /// Wraps a future so [`AbortHandle::abort`] stops it at its next poll.
    struct Abortable<F: Future> {
        inner: Pin<Box<F>>,
        aborted: Arc<AtomicBool>,
    }

    impl<F: Future> Future for Abortable<F> {
        type Output = Option<F::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            if this.aborted.load(Ordering::Relaxed) {
                return Poll::Ready(None);
            }
            this.inner.as_mut().poll(cx).map(Some)
        }
    }

    /// A set of tasks drained by [`JoinSet::join_next`].
    ///
    /// Unlike Tokio's, these futures advance only while `join_next` is being
    /// awaited. Every call site in this crate spawns the whole batch and then
    /// drains it, so the outputs are the same.
    pub struct JoinSet<T> {
        tasks: FuturesUnordered<BoxFuture<'static, T>>,
    }

    impl<T: Send + 'static> Default for JoinSet<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Send + 'static> JoinSet<T> {
        /// An empty set.
        #[must_use]
        pub fn new() -> Self {
            Self {
                tasks: FuturesUnordered::new(),
            }
        }

        /// Add a future to the set.
        ///
        /// Tokio's returns an `AbortHandle`; nothing in this crate uses it,
        /// and a handle that could not really cancel an un-detached future
        /// would only mislead, so this returns nothing.
        pub fn spawn<F>(&mut self, future: F)
        where
            F: Future<Output = T> + Send + 'static,
        {
            self.tasks.push(future.boxed());
        }

        /// How many tasks have not yet been drained.
        #[must_use]
        pub fn len(&self) -> usize {
            self.tasks.len()
        }

        /// Whether the set is empty.
        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.tasks.is_empty()
        }

        /// Await the next task to finish, or `None` once the set is drained.
        pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
            self.tasks.next().await.map(Ok)
        }

        /// Drop every task that has not yet been drained.
        pub fn abort_all(&mut self) {
            self.tasks = FuturesUnordered::new();
        }
    }

    /// A monotonic instant, measured in milliseconds since the worker started.
    ///
    /// `std::time::Instant` compiles on wasm32-unknown-unknown but panics the
    /// moment it is called, so timing sites use this instead.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub struct Instant {
        millis: f64,
    }

    impl Instant {
        /// The current instant.
        #[must_use]
        pub fn now() -> Self {
            Self { millis: now_ms() }
        }

        /// How long has passed since this instant. Saturates at zero.
        #[must_use]
        pub fn elapsed(&self) -> Duration {
            Self::now().duration_since(*self)
        }

        /// How long passed between `earlier` and this instant. Saturates at
        /// zero, matching `std::time::Instant`.
        #[must_use]
        pub fn duration_since(&self, earlier: Self) -> Duration {
            let delta = self.millis - earlier.millis;
            if delta <= 0.0 {
                Duration::ZERO
            } else {
                Duration::from_secs_f64(delta / 1000.0)
            }
        }

        /// This instant advanced by `duration`, or `None` on overflow (never,
        /// in practice — present for parity with Tokio's API).
        #[must_use]
        pub fn checked_add(&self, duration: Duration) -> Option<Self> {
            Some(Self {
                millis: self.millis + duration.as_secs_f64() * 1000.0,
            })
        }
    }

    impl std::ops::Add<Duration> for Instant {
        type Output = Self;

        fn add(self, rhs: Duration) -> Self {
            Self {
                millis: self.millis + rhs.as_secs_f64() * 1000.0,
            }
        }
    }

    impl std::ops::Sub<Self> for Instant {
        type Output = Duration;

        fn sub(self, rhs: Self) -> Duration {
            self.duration_since(rhs)
        }
    }

    /// Milliseconds from the host clock.
    ///
    /// `Date.now()` rather than `performance.now()`: reaching the latter needs
    /// a `web-sys` binding or a hand-written `extern` block, and this crate
    /// carries `#![forbid(unsafe_code)]`, which a `wasm_bindgen` extern block
    /// cannot satisfy. The server times only coarse budgets (tens of
    /// milliseconds and up) and logs elapsed spans, so millisecond wall-clock
    /// resolution is enough; the saturating subtraction in
    /// [`Instant::duration_since`] absorbs a backwards clock step.
    fn now_ms() -> f64 {
        js_sys::Date::now()
    }

    /// Sleep for `duration`, via `setTimeout`.
    ///
    /// The `setTimeout` future is awaited inside a detached task and the
    /// result comes back over a channel, rather than being awaited here
    /// directly. `gloo_timers`' `TimeoutFuture` owns a `Closure<dyn FnMut()>`
    /// and is therefore `!Send`, and every `LanguageServer` handler future in
    /// this crate must be `Send` on every target — holding the timer across
    /// this `await` would poison all of them. A `oneshot::Receiver` is `Send`,
    /// so the JS closure never crosses the boundary.
    pub async fn sleep(duration: Duration) {
        let millis = u32::try_from(duration.as_millis()).unwrap_or(u32::MAX);
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(millis).await;
            let _ = tx.send(());
        });
        let _ = rx.await;
    }

    /// Sleep until `deadline`, via `setTimeout`. Returns at once if the
    /// deadline has already passed.
    pub async fn sleep_until(deadline: Instant) {
        sleep(deadline.duration_since(Instant::now())).await;
    }

    /// Run `future` with a deadline, mirroring `tokio::time::timeout`.
    pub async fn timeout<F: Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
        let mut future = Box::pin(future);
        let mut timer = Box::pin(sleep(duration));
        std::future::poll_fn(move |cx| {
            if let Poll::Ready(value) = future.as_mut().poll(cx) {
                return Poll::Ready(Ok(value));
            }
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(Elapsed { _private: () }));
            }
            Poll::Pending
        })
        .await
    }

    /// The error [`timeout`] returns when its deadline passes first.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Elapsed {
        _private: (),
    }

    impl std::fmt::Display for Elapsed {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("deadline has elapsed")
        }
    }

    impl std::error::Error for Elapsed {}

    /// Yield to the worker's microtask queue.
    pub async fn yield_now() {
        let mut yielded = false;
        std::future::poll_fn(move |cx| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }

    /// A browser worker has exactly one thread.
    #[must_use]
    pub const fn available_parallelism() -> usize {
        1
    }
}

/// Wasi arm: Tokio re-exports, minus the thread pool.
///
/// A caveat for transport authors beyond the host requirements above: the
/// server's cancellation machinery leans on unwinding (salsa's `Cancelled`)
/// and on condvar blocking (`set_text` waiting out live snapshots), and both
/// are fatal on wasip1 — panics abort the process and an idle condvar wait
/// traps. It is sound today only because the inline [`spawn_blocking`] means
/// no analysis snapshot ever survives across an await; a transport that
/// reintroduces real concurrency there turns cancellations into aborts.
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
mod wasi {
    pub use tokio::task::{AbortHandle, JoinError, JoinHandle, JoinSet, spawn, yield_now};
    pub use tokio::time::{Instant, sleep, sleep_until, timeout};

    /// Run a blocking closure inline, on the calling task, and hand back a
    /// Tokio [`JoinHandle`] already carrying its value.
    ///
    /// This is the one item wasip1 cannot take straight from Tokio. It has no
    /// thread-spawn syscall, so `tokio::task::spawn_blocking` compiles and
    /// then fails at run time (`os error 58`) the first time the pool tries to
    /// grow — the closure has to run here instead, as it does in the browser.
    /// Handing the finished value to `spawn` rather than fabricating a handle
    /// is what keeps the return type Tokio's own, so every call site and its
    /// `abort`/[`JoinError`] handling compiles untouched; the task resolves
    /// on its first poll. Two semantic edges follow from running inline:
    /// `abort()` cannot cancel work that has already run, and a panicking
    /// closure is fatal — wasip1 is panic=abort, so the native
    /// [`JoinError::is_panic`] recovery paths never see it; the process dies
    /// instead. The browser arm shares the panic property (its hook takes
    /// over), so neither wasm arm can settle a document after a worker panic.
    ///
    /// Like Tokio's, this must be called from within the runtime.
    pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let value = f();
        spawn(async move { value })
    }

    /// The host's usable parallelism, defaulting to 4 where the platform
    /// cannot report it.
    ///
    /// Kept as the native expression rather than a hardcoded `1`: wasip1
    /// answers it truthfully (one thread), and a future threaded WASI target
    /// should be believed rather than second-guessed here.
    pub fn available_parallelism() -> usize {
        std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get)
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use std::time::Duration;

    #[tokio::test]
    async fn spawn_blocking_round_trips_a_value() {
        let handle = super::spawn_blocking(|| 6 * 7);
        assert_eq!(handle.await.expect("task should not fail"), 42);
    }

    #[tokio::test]
    async fn spawn_round_trips_a_value() {
        let handle = super::spawn(async { "done" });
        assert_eq!(handle.await.expect("task should not fail"), "done");
    }

    #[tokio::test]
    async fn join_set_drains_every_task() {
        let mut set = super::JoinSet::new();
        for n in 0..4_u32 {
            set.spawn(async move { n * 2 });
        }
        let mut seen = Vec::new();
        while let Some(res) = set.join_next().await {
            seen.push(res.expect("task should not fail"));
        }
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 2, 4, 6]);
    }

    #[tokio::test]
    async fn timeout_reports_a_completed_future() {
        let value = super::timeout(Duration::from_secs(30), async { 1_u8 }).await;
        assert_eq!(value.expect("should not time out"), 1);
    }

    #[test]
    fn available_parallelism_is_at_least_one() {
        assert!(super::available_parallelism() >= 1);
    }
}
