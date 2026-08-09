// tcl-lsp -- a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Keeps the LSP stdin reader independent of handler progress.
//!
//! `tower-lsp-server` 0.23 places a 100-item channel in front of
//! `buffer_unordered(max_concurrency)`. Its sole stdin reader awaits a send to
//! that channel. When every handler slot is pending, the channel fills and the
//! reader stops routing input, including responses needed by those handlers.
//!
//! Production uses [`UNBOUNDED_TRANSPORT_CONCURRENCY`] for the transport's
//! `buffer_unordered` limit, so it always absorbs the bounded channel into its
//! own pending-future set. [`DeferredConcurrency`] separately admits only four
//! ordinary inner handler futures at a time, while transport-control
//! notifications bypass that limit. Input routing is effectively unbounded,
//! while shared application state sees the same ordinary concurrency as the
//! upstream default.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::Semaphore;
use tower::Service;
use tower_lsp_server::jsonrpc::Request;

/// Makes `buffer_unordered` absorb every queued future rather than waiting for
/// a handler to complete before it reads another item from the input queue.
///
/// `FuturesUnordered` does not allocate this capacity up front. In practice
/// the only bound is the number of messages received from the local editor,
/// which is deliberately not finite: any finite threshold recreates the same
/// liveness cycle at a different queue length.
pub const UNBOUNDED_TRANSPORT_CONCURRENCY: usize = usize::MAX;

/// The ordinary application-handler concurrency retained from
/// `tower-lsp-server`'s default transport configuration. Transport-control
/// notifications do not consume these permits.
pub const DEFAULT_HANDLER_CONCURRENCY: usize = 4;

/// A service wrapper which keeps ordinary LSP handlers at `limit` concurrent
/// while admitting transport-control notifications immediately.
///
/// The inner service's `poll_ready` and `call` run at the same points as they do
/// in the upstream transport. That eager `call` is important: `LspService`
/// registers cancellable request IDs and applies `exit` synchronously. Only
/// polling the returned ordinary-handler future waits for a permit. `exit` and
/// `$/cancelRequest` also bypass the polling limit because both exist to stop
/// work already in progress.
#[derive(Debug)]
pub struct DeferredConcurrency<S> {
    inner: S,
    permits: Arc<Semaphore>,
}

impl<S> DeferredConcurrency<S> {
    /// Wrap `inner`, polling at most `limit` returned futures concurrently.
    ///
    /// # Panics
    ///
    /// Panics when `limit` is zero, because no ordinary handler could ever
    /// make progress.
    #[must_use]
    pub fn new(inner: S, limit: usize) -> Self {
        assert!(limit > 0, "handler concurrency must be non-zero");
        Self {
            inner,
            permits: Arc::new(Semaphore::new(limit)),
        }
    }
}

impl<S> Service<Request> for DeferredConcurrency<S>
where
    S: Service<Request>,
    S::Future: Send + 'static,
    S::Response: 'static,
    S::Error: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let permits = Arc::clone(&self.permits);
        let is_transport_control = matches!(request.method(), "exit" | "$/cancelRequest");
        // Construct eagerly, before admission. Besides matching Tower's
        // readiness contract, this lets LspService register a request with its
        // cancellation map before a later `$/cancelRequest` is read.
        let future = self.inner.call(request);
        Box::pin(async move {
            let permit = if is_transport_control {
                None
            } else {
                Some(
                    permits
                        .acquire_owned()
                        .await
                        .expect("handler semaphore is owned by the queued futures"),
                )
            };
            let result = future.await;
            drop(permit);
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tower_lsp_server::ls_types::{Hover, HoverParams, InitializeParams, InitializeResult};
    use tower_lsp_server::{LanguageServer, LspService};

    #[derive(Clone)]
    struct PeakService {
        active: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    }

    struct InitiallyPendingService {
        readiness_polls: Arc<AtomicUsize>,
        calls: Arc<AtomicUsize>,
    }

    struct ControlAwareService {
        ordinary_started: Arc<AtomicUsize>,
        controls_called: Arc<AtomicUsize>,
    }

    struct PendingHoverServer {
        hover_polls: Arc<AtomicUsize>,
    }

    impl LanguageServer for PendingHoverServer {
        async fn initialize(
            &self,
            _params: InitializeParams,
        ) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
            Ok(InitializeResult::default())
        }

        async fn shutdown(&self) -> tower_lsp_server::jsonrpc::Result<()> {
            Ok(())
        }

        async fn hover(
            &self,
            _params: HoverParams,
        ) -> tower_lsp_server::jsonrpc::Result<Option<Hover>> {
            self.hover_polls.fetch_add(1, Ordering::SeqCst);
            std::future::pending().await
        }
    }

    fn request(method: &'static str) -> Request {
        Request::build(method).finish()
    }

    impl Service<Request> for InitiallyPendingService {
        type Response = ();
        type Error = Infallible;
        type Future = std::future::Ready<Result<(), Infallible>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            let poll = self.readiness_polls.fetch_add(1, Ordering::SeqCst);
            if poll == 0 {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(()))
        }
    }

    impl Service<Request> for PeakService {
        type Response = ();
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<(), Infallible>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            let active = Arc::clone(&self.active);
            let peak = Arc::clone(&self.peak);
            Box::pin(async move {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    impl Service<Request> for ControlAwareService {
        type Response = ();
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<(), Infallible>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            if matches!(request.method(), "exit" | "$/cancelRequest") {
                self.controls_called.fetch_add(1, Ordering::SeqCst);
                Box::pin(std::future::ready(Ok(())))
            } else {
                let ordinary_started = Arc::clone(&self.ordinary_started);
                Box::pin(async move {
                    ordinary_started.fetch_add(1, Ordering::SeqCst);
                    std::future::pending().await
                })
            }
        }
    }

    #[tokio::test]
    async fn limits_polling_without_limiting_admission() {
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut service = DeferredConcurrency::new(
            PeakService {
                active: Arc::clone(&active),
                peak: Arc::clone(&peak),
            },
            2,
        );
        let mut futures = Vec::new();
        for _ in 0..12 {
            std::future::poll_fn(|cx| service.poll_ready(cx))
                .await
                .unwrap();
            futures.push(service.call(request("test")));
        }
        futures_util::future::join_all(futures).await;
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn forwards_inner_readiness_before_call() {
        let readiness_polls = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut service = DeferredConcurrency::new(
            InitiallyPendingService {
                readiness_polls: Arc::clone(&readiness_polls),
                calls: Arc::clone(&calls),
            },
            1,
        );

        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        service.call(request("test")).await.unwrap();
        assert_eq!(readiness_polls.load(Ordering::SeqCst), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn transport_controls_bypass_saturated_handlers() {
        let ordinary_started = Arc::new(AtomicUsize::new(0));
        let controls_called = Arc::new(AtomicUsize::new(0));
        let mut service = DeferredConcurrency::new(
            ControlAwareService {
                ordinary_started: Arc::clone(&ordinary_started),
                controls_called: Arc::clone(&controls_called),
            },
            2,
        );

        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        let first = tokio::spawn(service.call(request("ordinary/one")));
        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        let second = tokio::spawn(service.call(request("ordinary/two")));
        while ordinary_started.load(Ordering::SeqCst) != 2 {
            tokio::task::yield_now().await;
        }

        for method in ["$/cancelRequest", "exit"] {
            std::future::poll_fn(|cx| service.poll_ready(cx))
                .await
                .unwrap();
            tokio::time::timeout(Duration::from_millis(100), service.call(request(method)))
                .await
                .expect("transport control was queued behind saturated handlers")
                .unwrap();
        }
        assert_eq!(controls_called.load(Ordering::SeqCst), 2);

        first.abort();
        second.abort();
    }

    #[tokio::test]
    async fn cancellation_reaches_a_request_queued_for_a_permit() {
        let hover_polls = Arc::new(AtomicUsize::new(0));
        let (inner, _socket) = LspService::new(|_client| PendingHoverServer {
            hover_polls: Arc::clone(&hover_polls),
        });
        let mut service = DeferredConcurrency::new(inner, 1);

        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        let initialize = Request::build("initialize")
            .id(0)
            .params(serde_json::json!({ "capabilities": {} }))
            .finish();
        service.call(initialize).await.unwrap();

        let hover_params = serde_json::json!({
            "textDocument": { "uri": "file:///queued-cancel.tcl" },
            "position": { "line": 0, "character": 0 }
        });
        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        let first = tokio::spawn(
            service.call(
                Request::build("textDocument/hover")
                    .id(1)
                    .params(hover_params.clone())
                    .finish(),
            ),
        );
        while hover_polls.load(Ordering::SeqCst) != 1 {
            tokio::task::yield_now().await;
        }

        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        let queued = tokio::spawn(
            service.call(
                Request::build("textDocument/hover")
                    .id(2)
                    .params(hover_params)
                    .finish(),
            ),
        );
        tokio::task::yield_now().await;
        assert_eq!(
            hover_polls.load(Ordering::SeqCst),
            1,
            "the queued request must not be polled before a permit is free"
        );

        std::future::poll_fn(|cx| service.poll_ready(cx))
            .await
            .unwrap();
        service
            .call(
                Request::build("$/cancelRequest")
                    .params(serde_json::json!({ "id": 2 }))
                    .finish(),
            )
            .await
            .unwrap();
        first.abort();

        let response = tokio::time::timeout(Duration::from_secs(1), queued)
            .await
            .expect("queued cancellation did not resolve")
            .expect("queued handler task panicked")
            .expect("LSP service exited while cancelling queued request")
            .expect("cancelled request must return an error response");
        let response = serde_json::to_value(response).unwrap();
        assert_eq!(response["error"]["code"], -32800);
        assert_eq!(
            hover_polls.load(Ordering::SeqCst),
            1,
            "a request cancelled while queued must never poll its handler"
        );
    }
}
