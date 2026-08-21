# The #1657 task-resumption wedge: evidence, reproduction, and a distillation attempt

Status: **open**. The wedge is characterised to a single mechanism-shaped
question but not yet root-caused. Per direction, nothing here has been filed
outside this repository.

## Environment

| | |
| --- | --- |
| Server | `tcl-lsp-server`, branch `claude/f6h-1657-pollshim` (instrumentation merged to `rust` via #1667, #1670, #1677, #1679) |
| rustc | 1.98.0 (88d9e12ae 2026-08-18) |
| tokio | 1.53.1 pinned; also reproduced with `--precise 1.51.1` |
| OS | Ubuntu 24.04.4 LTS, kernel `6.18.44-fc-v21` (VM), 4 vCPUs |
| Load | ext-host suite under `nproc/2` spinner processes |

## The symptom

Under the loaded VS Code ext-host suite, the whole server intermittently
answers nothing for tens of seconds to minutes, then recovers. Every request
handler waits on the document-sync barrier; the barrier's turn holder is
parked on the open-document map; the map's holder never finishes a
2-second-capped client send.

## The evidence chain, compressed

Full trail: issue #1657, checkpoints 1–25. The captures that matter:

1. **The holder is parked at a named await** (phase markers, #1667): the turn
   holder suspends at `documents.lock` with phase age equal to turn age.
2. **The map names its holder** (holder tag, #1670): `publish_diagnostics_result
   → cache_and_deliver`, holds of 45.8s–167.2s against a 2s send cap.
3. **Free map, frozen acquisition counter** (#1677): a waiter parked on a
   *free* map across a 250 ms sample window in which nobody acquired it — a
   waiter that was never resumed, not contention.
4. **The split-waker verdict** (#1679): the send runs under a hand-rolled
   timeout whose two halves are polled through separate recording wakers.
   Three captures, identical:

   ```text
   its publish send has run 45.8s against a 2.0s cap, polled 1 time(s) (last 45.8s ago),
   send half woken 1 time(s), timer half woken 1 time(s) —
   the timer FIRED 43.8s ago and the task was NEVER POLLED again
   ```

   In every capture `hold_age − timer_wake_age = 2.0s` exactly: **the timer
   fires precisely on schedule**. The wake is delivered (the recording waker
   runs before forwarding to the real waker). The task is never polled again —
   here for 43.8s — while sibling tasks on the same runtime keep being polled
   (the stall reporter itself ran a 10s timeout and a 250 ms sleep to produce
   the line, and the transport's handler futures were polled during the
   window).

## Reconstruction timeline (fixed-binary capture, checkpoint 24)

| t | event |
| --- | --- |
| 0 | holder (a spawned diagnostics worker) acquires the map, enters the capped publish send; poll 1 registers both halves |
| ~0 | send half delivers one wake (channel capacity moved) — **no poll follows** |
| 2.0s | timer half delivers its wake, exactly on the cap — **no poll follows** |
| 2.0s → 45.8s+ | nothing: two wakes delivered through two independent wakers, zero polls, sibling tasks proceeding |
| (production) | the wedge ends when the next client message arrives |

## What is excluded, and by what

| hypothesis | excluded by |
| --- | --- |
| lock cycles through the barrier | full transitive waits-for reachability (checkpoint 1) |
| salsa `cancel_others` | census: zero outstanding snapshots in every capture |
| slow/blocked holder (long hold) | phase clock: holder frozen *inside* a 2s-capped send |
| unbounded send (pre-#1670) | send caps; then holds of 70–167s *through* the caps |
| mutex contention / skipped waiter | acquisition counter frozen over a free map |
| timer subsystem / tokio 1.53 time-driver rework | timer fires at exactly cap in every capture |
| tokio 1.52+ scheduler changes | **byte-identical wedge on tokio 1.51.1** (checkpoint 25) |
| transport parent starvation | handler futures polled during the stall window; the holder is a spawned task the transport does not own |

## The open question, stated precisely

A task whose waker is invoked — twice, from two sources — is never scheduled
onto any queue a worker runs, for tens of seconds, while sibling tasks
proceed; workers show parked in futex throughout; recovery coincides with the
next external input.

The frame that fits every observation is a **lost worker unpark**: waker runs
→ task is queued → the final unpark of a parked worker is lost → the task
sits runnable until some *other* event unparks a worker (in production, the
next client message — which is why the wedge lasts "until the next request"
and the server always recovers). Candidates below every line of this
codebase: an old latent tokio park/unpark race, or futex/timer delivery at
the kernel/VM layer (`6.18.44-fc-v21`). Neither is proven.

## What reliably reproduces it: the in-repo loaded loop

The full server under the loaded ext-host suite. This is the measured
reproducer; rates below are from this investigation's logs.

```sh
# from the repo root, with the instrumented server built:
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2 make rust-server
# loop: burn nproc/2 cores in spinners, run the ext-host suite serially,
# grep the server log for "[stall] document-sync barrier"; repeat.
# (the loop script used in the investigation is quoted in #1657 checkpoint 9)
```

Observed capture rates (each "run" ≈ 4-minute suite, ~1000 diagnostics
publishes):

| build | runs to capture, per loop |
| --- | --- |
| tokio 1.53.1, phases only | 12, then 7, then 8 |
| tokio 1.53.1, holder tag | 12 |
| tokio 1.53.1, poll shim | 1, then 2, then 1 |
| **tokio 1.51.1** (downgrade experiment) | **2** |

Earlier in the investigation two 11–12-run clean stretches occurred; the rate
is bursty. A clean streak is not evidence of health — that lesson recurred
three times.

The stall line to look for (the instrumentation prints the verdict):

```text
[stall] document-sync barrier has not advanced ... suspended at `did_open: documents.lock` ...
the open-document map is held by cache_and_deliver: publish send — Xs in total, Xs at this point;
its publish send has run Xs against a 2.0s cap ... — the timer FIRED ... NEVER POLLED again ...
```

## The standalone distillation — **does not reproduce**, stated plainly

Per direction, a minimal self-contained sample was built from the convicted
shape and instrumented identically, self-verdicting (exit 1 + verdict block on
wedge, exit 0 on a measured clean pass, exit 2 on harness fault), including an
**unpark probe**: on a detected wedge, a std thread outside the runtime spawns
no-op tasks; if the wedged holder then gets polled, only the worker unpark was
missing.

It has **not** wedged in 5,400 trials on this box on tokio 1.53.1:

| configuration | trials | wedges |
| --- | --- | --- |
| idle box | 400 | 0 |
| 2 spinner threads (nproc/2, as the loop uses) | 2,000 | 0 |
| 5 spinners + `spawn_blocking` churn + live I/O driver traffic + 512-task bursts | 3,000 | 0 |

Compare ~1-in-2 loaded suite runs (~1000 publishes each) in the real server:
if the distilled shape carried the trigger, 5,400 trials should have fired
several times. It did not, so **the trigger involves something the
distillation lacks**. Candidates, in rough order of suspicion: the sheer
process-tree oversubscription of the real environment (VS Code + node + server
on 4 vCPUs, cgroup scheduling), the server's real I/O topology (stdio through
a pipe to a busy peer), or an unidentified server-specific ingredient. The
sample is still shipped below: it is the faithful distillation, its harness
prints the same verdict the server prints, and a machine or environment where
it *does* fire would localise the trigger sharply.

What was tried, so nobody repeats it blind: sweeping the receiver drain delay
across the cap boundary (7 ms steps over 0–550 ms); mutex waiters parked
behind the holder; 24-task churn; 512-task ready bursts (local-queue
overflow); `spawn_blocking` completions (non-worker-thread wake path); live
duplex I/O traffic; 2 and 5 spinner threads; 4 runtime workers as production.

### `Cargo.toml`

```toml
[package]
name = "wedge-repro"
version = "0.1.0"
edition = "2021"

# Pin exactly what tcl-lsp ships. The wedge also reproduces with tokio pinned
# to =1.51.1, so the version is not the variable — but the sample documents the
# environment it was verified in.
[dependencies]
tokio = { version = "=1.53.1", features = ["rt-multi-thread", "sync", "time", "macros", "io-util"] }
futures = "0.3"

[profile.release]
debug = true
```

### `src/main.rs`

```rust
//! Minimal reproducer for the tcl-lsp #1657 task-resumption wedge.
//!
//! Distilled from the convicted production shape: on a multi-thread tokio
//! runtime under task churn, a spawned task holds a `tokio::sync::Mutex`
//! while awaiting a deadline-capped `futures::mpsc` channel send against a
//! slow receiver. In the production captures the holder's waker was invoked —
//! by the send half AND by the timer half, through two independently
//! instrumented wakers — and the task was never polled again for tens of
//! seconds, while sibling tasks kept running. Byte-identical signature on
//! tokio 1.53.1 and 1.51.1.
//!
//! Exit codes:
//! * 0 — all trials completed, no wedge (the trial count is printed so a
//!   clean pass is a measured claim, not silence);
//! * 1 — wedge detected; a verdict block prints polls/wakes with ages,
//!   mirroring the server's `[stall]` line, plus the unpark-probe outcome;
//! * 2 — harness fault (a trial neither completed nor wedged in time).
//!
//! The unpark probe: on a detected wedge the watchdog — a plain std thread,
//! deliberately outside the runtime — spawns no-op tasks into the runtime.
//! If the wedged holder then gets polled, the wake chain was intact and only
//! the final worker unpark was missing (the lost-unpark verdict). If it stays
//! wedged, the queued task itself was lost.
//!
//! The race is probabilistic: trials sweep the receiver's drain delay across
//! the cap boundary so the send-half and timer-half wakes land in varying
//! proximity. Observed rates belong in the accompanying document.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use futures::SinkExt;
use futures::StreamExt;

const BUDGET: Duration = Duration::from_millis(250);
const WEDGE_AFTER: Duration = Duration::from_secs(3);
const TRIAL_LIMIT: Duration = Duration::from_secs(20);
const TRIALS: u64 = 400;
const CHURN_TASKS: usize = 24;
/// Large enough to overflow a worker's 256-slot local queue.
const BURST: usize = 512;
const WAITERS: usize = 3;

#[derive(Default, Clone, Copy)]
struct WakeRec {
    count: u64,
    last: Option<Instant>,
}

/// One trial's holder telemetry — the same split accounting as the server.
struct Telemetry {
    /// `Some(start)` while a guarded send is in flight.
    started: Mutex<Option<Instant>>,
    polls: AtomicU64,
    last_poll: Mutex<Option<Instant>>,
    send_wakes: Arc<Mutex<WakeRec>>,
    timer_wakes: Arc<Mutex<WakeRec>>,
}

impl Telemetry {
    fn new() -> Self {
        Self {
            started: Mutex::new(None),
            polls: AtomicU64::new(0),
            last_poll: Mutex::new(None),
            send_wakes: Arc::new(Mutex::new(WakeRec::default())),
            timer_wakes: Arc::new(Mutex::new(WakeRec::default())),
        }
    }

    /// Arm for a fresh trial. Counters reset BEFORE `started` is set, so the
    /// watchdog can never pair a new trial with the previous trial's numbers.
    fn arm(&self) {
        self.polls.store(0, Ordering::Relaxed);
        *self.last_poll.lock().unwrap() = None;
        *self.send_wakes.lock().unwrap() = WakeRec::default();
        *self.timer_wakes.lock().unwrap() = WakeRec::default();
        *self.started.lock().unwrap() = Some(Instant::now());
    }

    fn disarm(&self) {
        *self.started.lock().unwrap() = None;
    }
}

struct Recorder {
    real: Waker,
    rec: Arc<Mutex<WakeRec>>,
}

impl Recorder {
    fn note(&self) {
        // Record BEFORE forwarding, so a poll caused by this wake can never be
        // observed ahead of the wake that caused it.
        let mut r = self.rec.lock().unwrap();
        r.count += 1;
        r.last = Some(Instant::now());
    }
}

impl Wake for Recorder {
    fn wake(self: Arc<Self>) {
        self.note();
        self.real.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.note();
        self.real.wake_by_ref();
    }
}

/// `tokio::time::timeout`, hand-rolled so each half's wakes are recorded
/// separately — identical to the server's instrumented delivery send.
struct SplitTimeout<F> {
    sleep: Pin<Box<tokio::time::Sleep>>,
    fut: Pin<Box<F>>,
    t: Arc<Telemetry>,
}

impl<F: Future> Future for SplitTimeout<F> {
    type Output = Result<F::Output, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let t = Arc::clone(&self.t);
        t.polls.fetch_add(1, Ordering::Relaxed);
        *t.last_poll.lock().unwrap() = Some(Instant::now());
        let send_waker = Waker::from(Arc::new(Recorder {
            real: cx.waker().clone(),
            rec: Arc::clone(&t.send_wakes),
        }));
        let mut send_cx = Context::from_waker(&send_waker);
        if let Poll::Ready(v) = self.fut.as_mut().poll(&mut send_cx) {
            return Poll::Ready(Ok(v));
        }
        let timer_waker = Waker::from(Arc::new(Recorder {
            real: cx.waker().clone(),
            rec: Arc::clone(&t.timer_wakes),
        }));
        let mut timer_cx = Context::from_waker(&timer_waker);
        if self.sleep.as_mut().poll(&mut timer_cx).is_ready() {
            return Poll::Ready(Err(()));
        }
        Poll::Pending
    }
}

fn age(i: Option<Instant>) -> String {
    i.map_or_else(|| "never".into(), |i| format!("{:.1}s ago", i.elapsed().as_secs_f64()))
}

/// The out-of-runtime arbiter. Detects the wedge, prints the verdict, runs
/// the unpark probe, exits with the appropriate code.
fn watchdog(t: Arc<Telemetry>, handle: tokio::runtime::Handle) {
    loop {
        std::thread::sleep(Duration::from_millis(100));
        let Some(t0) = *t.started.lock().unwrap() else {
            continue;
        };
        let send = *t.send_wakes.lock().unwrap();
        let timer = *t.timer_wakes.lock().unwrap();
        let last_poll = *t.last_poll.lock().unwrap();
        let polls = t.polls.load(Ordering::Relaxed);
        let last_wake = match (send.last, timer.last) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };

        // Wedge shape 1: woken, then never polled again.
        let woken_not_polled = matches!(
            (last_wake, last_poll),
            (Some(w), Some(p)) if w > p && w.elapsed() > WEDGE_AFTER
        );
        // Wedge shape 2: the cap passed long ago and the timer wake was never
        // delivered at all.
        let timer_never_fired =
            timer.count == 0 && t0.elapsed() > BUDGET + WEDGE_AFTER;

        if woken_not_polled || timer_never_fired {
            println!("WEDGE DETECTED after {:.1}s:", t0.elapsed().as_secs_f64());
            println!(
                "  guarded send: polled {polls} time(s) (last {}), cap {:.2}s",
                age(last_poll),
                BUDGET.as_secs_f64(),
            );
            println!(
                "  send half woken {} time(s) ({}); timer half woken {} time(s) ({})",
                send.count,
                age(send.last),
                timer.count,
                age(timer.last),
            );
            if woken_not_polled {
                println!(
                    "  VERDICT: wake delivered, task never polled again — \
                     the scheduler never ran this task"
                );
            } else {
                println!(
                    "  VERDICT: the cap passed and the timer wake was never \
                     delivered — timer subsystem"
                );
            }

            // Unpark probe: poke the runtime from outside.
            let polls_before = t.polls.load(Ordering::Relaxed);
            for _ in 0..4 {
                handle.spawn(async {});
            }
            std::thread::sleep(Duration::from_secs(2));
            let polls_after = t.polls.load(Ordering::Relaxed);
            if polls_after > polls_before {
                println!(
                    "  UNPARK PROBE: spawning from outside UNWEDGED it \
                     (polls {polls_before} -> {polls_after}) — the task was \
                     queued and runnable; only the worker unpark was missing"
                );
            } else {
                println!(
                    "  UNPARK PROBE: still wedged after external spawns \
                     (polls {polls_before} -> {polls_after}) — the queued \
                     task itself was lost"
                );
            }
            std::process::exit(1);
        }

        if t0.elapsed() > TRIAL_LIMIT {
            println!(
                "HARNESS FAULT: trial neither completed nor wedged in {:.0}s \
                 (polls {polls}, send wakes {}, timer wakes {})",
                TRIAL_LIMIT.as_secs_f64(),
                send.count,
                timer.count,
            );
            std::process::exit(2);
        }
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    // Oversubscribe the box, as the production wedge's loaded repro loop did
    // (it burned half the cores in spinner processes). Self-contained here:
    // plain std threads, outside the runtime, spinning for the whole run.
    let spinners = env_u64(
        "WEDGE_SPIN",
        (std::thread::available_parallelism().map_or(4, |n| n.get()) / 2) as u64,
    );
    for _ in 0..spinners {
        std::thread::spawn(|| loop {
            std::hint::spin_loop();
        });
    }
    let trials = env_u64("WEDGE_TRIALS", TRIALS);
    println!(
        "wedge-repro: {trials} trials, {spinners} spinner thread(s), cap {:?}",
        BUDGET
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("runtime");
    let t = Arc::new(Telemetry::new());
    {
        let t = Arc::clone(&t);
        let handle = rt.handle().clone();
        std::thread::spawn(move || watchdog(t, handle));
    }

    rt.block_on(async move {
        // Steady churn, mirroring the loaded ext-host suite: tasks that
        // yield, briefly sleep, and keep every worker busy-ish.
        for i in 0..CHURN_TASKS {
            tokio::spawn(async move {
                let mut n = 0u64;
                loop {
                    tokio::task::yield_now().await;
                    n += 1;
                    if n % 64 == i as u64 % 64 {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            });
        }
        // Ready-task bursts, big enough to overflow a worker's local queue.
        tokio::spawn(async {
            loop {
                for _ in 0..BURST {
                    tokio::spawn(async {});
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        // spawn_blocking churn — the server runs analysis on the blocking pool
        // constantly, and completions wake runtime tasks from non-worker
        // threads, a distinct unpark path.
        tokio::spawn(async {
            loop {
                let _ = tokio::task::spawn_blocking(|| std::hint::black_box(7u64) * 6).await;
                tokio::time::sleep(Duration::from_millis(3)).await;
            }
        });
        // Keep the I/O driver genuinely busy, as the server's stdin/stdout do:
        // a duplex pipe with a writer task and a reader task.
        let (mut a, mut b) = tokio::io::duplex(64);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt as _;
            loop {
                if a.write_all(&[1u8; 16]).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt as _;
            let mut buf = [0u8; 16];
            while b.read_exact(&mut buf).await.is_ok() {}
        });

        let map = Arc::new(tokio::sync::Mutex::new(0u64));
        for trial in 0..trials {
            // Sweep the receiver's drain delay across the cap boundary.
            let drain_delay =
                Duration::from_millis((trial * 7) % (2 * BUDGET.as_millis() as u64 + 50));

            // Zero-buffer channel: the pre-fill occupies the sender's one
            // slot, so the guarded send genuinely parks — as the production
            // send parks on tower-lsp's channel(1).
            let (mut tx, mut rx) = futures::channel::mpsc::channel::<u64>(0);
            tx.try_send(0).expect("pre-fill fits the sender slot");
            assert!(
                tx.try_send(0).is_err(),
                "the channel must be full, or the guarded send will not park"
            );

            // Slow receiver — the "stalled client".
            let receiver = tokio::spawn(async move {
                tokio::time::sleep(drain_delay).await;
                while rx.next().await.is_some() {}
            });

            // Waiters queued on the map, as in every production capture.
            let mut waiters = Vec::new();
            for _ in 0..WAITERS {
                let map = Arc::clone(&map);
                waiters.push(tokio::spawn(async move {
                    *map.lock().await += 1;
                }));
            }

            // The holder: lock the map, then the capped guarded send.
            let holder = {
                let map = Arc::clone(&map);
                let t = Arc::clone(&t);
                tokio::spawn(async move {
                    let _guard = map.lock().await;
                    t.arm();
                    let out = SplitTimeout {
                        sleep: Box::pin(tokio::time::sleep(BUDGET)),
                        fut: Box::pin(async move { tx.send(1).await }),
                        t: Arc::clone(&t),
                    }
                    .await;
                    t.disarm();
                    out
                })
            };

            holder.await.expect("holder must not panic").ok();
            for w in waiters {
                w.await.expect("waiter must not panic");
            }
            receiver.await.expect("receiver must not panic");

            if (trial + 1) % 50 == 0 {
                println!("trial {}/{trials} clean", trial + 1);
            }
        }
        println!("CLEAN PASS: {trials} trials, no wedge");
    });
}
```

### Running it

```sh
cargo build --release
./target/release/wedge-repro                 # defaults: 400 trials, nproc/2 spinners
WEDGE_TRIALS=3000 WEDGE_SPIN=5 ./target/release/wedge-repro
echo $?   # 0 clean pass, 1 wedge (verdict printed), 2 harness fault
```

Expected on a healthy scheduler: `CLEAN PASS`, exit 0, in a few minutes.
A wedge prints the same discrimination the server's stall line prints, plus
the unpark-probe outcome.

## Where the next session picks up

1. The instruments are all merged; any future CI or loaded-loop wedge
   self-diagnoses to the verdict line without new work.
2. The discriminating observation still missing: during a live wedge, does an
   *external* poke (a client request, or the probe's out-of-runtime spawn)
   end it immediately? The production recovery pattern says yes; nobody has
   triggered one on demand yet. The server-side equivalent of the unpark
   probe — a std-thread watchdog that spawns a no-op task when the stall
   reporter fires — would turn the ~100s recovery into a sub-second one *and*
   confirm the lost-unpark frame in the same stroke. That is a candidate
   mitigation, not a fix, and it is deliberately not implemented ahead of
   direction.
3. If confirmation is wanted at the OS layer: perf/ftrace on futex syscalls
   around a live wedge would show whether the unpark futex op is issued and
   lost or never issued.

## Checkpoint index (issue #1657)

Phases/census #1667: checkpoints 1–4 · holder tag + send caps #1670: 5–8 ·
contention discriminator #1677: 13–17 · structure finding: 18 · poll shim +
send telemetry #1679: 20–24 · tokio 1.51.1 result: 25.
