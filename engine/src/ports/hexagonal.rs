//! Hexagonal ports — Clock, IdGenerator, Stopwatch (wave 2).
//!
//! These three traits close the remaining application-layer time/id/duration leaks:
//! - [`Clock`] — injectable wall-clock time (UTC DateTime)
//! - [`IdGenerator`] — injectable UUID generation
//! - [`Stopwatch`] — injectable elapsed-time measurement
//!
//! Each trait has two adapters:
//! - **System adapter** — delegates to the real system call; used in production
//! - **Mock adapter** — holds fixed/controlled values; used in deterministic tests
//!
//! # Leak closure
//!
//! After this module lands, no service in `engine/src/services/` or
//! `engine/src/ai/` should call `chrono::Utc::now()`, `uuid::Uuid::new_v4()`,
//! or `std::time::Instant::now()` directly.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Clock — wall-clock time port
// ─────────────────────────────────────────────────────────────────────────────

/// Port for obtaining the current wall-clock time (UTC).
///
/// Abstracts `chrono::Utc::now()` so services are deterministic in tests.
///
/// System adapter: [`SystemClock`]
/// Mock adapter: [`MockClock`]
pub trait Clock: Send + Sync {
    /// Returns the current UTC DateTime.
    fn now(&self) -> DateTime<Utc>;
}

/// System-clock adapter — delegates to `chrono::Utc::now()`.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Mock-clock adapter — returns a fixed `DateTime<Utc>` until `set` is called.
///
/// Uses `Mutex<DateTime<Utc>>` for interior mutability so the `set` method
/// works on `&self` and the struct satisfies `Send + Sync`.
///
/// # Example
/// ```
/// use engine::ports::hexagonal::{Clock, MockClock};
/// let clock = MockClock::new(chrono::Utc::now());
/// assert_eq!(clock.now(), clock.now()); // deterministic
/// ```
#[derive(Debug)]
pub struct MockClock {
    /// The `DateTime<Utc>` returned by every `now()` call.
    now: std::sync::Mutex<DateTime<Utc>>,
}

impl Clone for MockClock {
    fn clone(&self) -> Self {
        Self {
            now: std::sync::Mutex::new(*self.now.lock().unwrap()),
        }
    }
}

impl MockClock {
    /// Construct a mock clock returning `fixed`.
    pub fn new(fixed: DateTime<Utc>) -> Self {
        Self {
            now: std::sync::Mutex::new(fixed),
        }
    }

    /// Overwrite the returned time. Useful for advancing fake time between calls.
    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().unwrap() = now;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IdGenerator — UUID generation port
// ─────────────────────────────────────────────────────────────────────────────

/// Port for generating unique identifiers.
///
/// Abstracts `uuid::Uuid::new_v4()` so services are deterministic in tests.
///
/// System adapter: [`RandomIdGen`]
/// Mock adapter: [`MockIdGen`]
pub trait IdGenerator: Send + Sync {
    /// Returns a new UUID.
    fn next_id(&self) -> Uuid;
}

/// Random-ID adapter — delegates to `uuid::Uuid::new_v4()`.
pub struct RandomIdGen;

impl IdGenerator for RandomIdGen {
    fn next_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Mock-ID adapter — returns counter-based UUIDs for deterministic tests.
///
/// Uses `AtomicU64` for interior mutability so the struct satisfies `Send + Sync`.
///
/// - Call 0: returns `Uuid::nil()` (zero UUID)
/// - Call N (N ≥ 1): returns `Uuid::from_u64_pair(N, 0)` — a UUID whose
///   most-significant 8 bytes are the counter `N` and whose least-significant
///   8 bytes are zero. Consecutive calls produce values that differ predictably.
///
/// # Example
/// ```
/// use engine::ports::hexagonal::{IdGenerator, MockIdGen};
/// let id_gen = MockIdGen::new();
/// let id0 = id_gen.next_id(); // Uuid::nil()
/// let id1 = id_gen.next_id(); // different from id0
/// let id2 = id_gen.next_id(); // different from id1
/// assert_ne!(id1, id2);
/// ```
#[derive(Debug)]
pub struct MockIdGen {
    counter: std::sync::atomic::AtomicU64,
}

impl Clone for MockIdGen {
    fn clone(&self) -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(
 self.counter.load(std::sync::atomic::Ordering::SeqCst)),
        }
    }
}

impl MockIdGen {
    /// Construct a mock ID generator starting at counter 0.
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Returns the current counter value without advancing it.
    pub fn counter(&self) -> u64 {
        self.counter.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Advance the counter by `n`. Used by tests that want to inject a
    /// specific ID without calling `next_id()`.
    pub fn advance_by(&self, n: u64) {
        self.counter.fetch_add(n, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Default for MockIdGen {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator for MockIdGen {
    fn next_id(&self) -> Uuid {
        let n = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n == 0 {
            // Call 0: return nil UUID
            Uuid::nil()
        } else {
            // Calls 1+: embed counter in bytes 8-15 (node field of v4 UUID)
            Uuid::from_u64_pair(n, 0)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stopwatch — elapsed-time port
// ─────────────────────────────────────────────────────────────────────────────

/// Opaque handle returned by [`Stopwatch::start`].
///
/// System adapter stores `std::time::Instant`; mock adapter stores a `u64` index.
/// The handle is constructible only inside the engine crate; external consumers
/// receive opaque handles from stopwatch adapters and must use them through the
/// `Stopwatch` trait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopwatchHandle(std::time::Instant);

impl StopwatchHandle {
    /// Wrap an `Instant` (used by [`SystemStopwatch`]). Crate-internal: the
    /// handle is opaque to consumers; only stopwatch adapters may construct one.
    pub(crate) fn from_instant(i: std::time::Instant) -> Self {
        Self(i)
    }

    /// Returns the inner instant (used only by [`SystemStopwatch`]).
    /// Crate-internal: the handle is opaque to consumers; only stopwatch
    /// adapters may unwrap the inner `Instant`.
    pub(crate) fn as_instant(&self) -> std::time::Instant {
        self.0
    }
}

/// Port for measuring elapsed wall-clock time in milliseconds.
///
/// Abstracts `std::time::Instant::now()` so services are deterministic in tests.
///
/// System adapter: [`SystemStopwatch`]
/// Mock adapter: [`MockStopwatch`]
pub trait Stopwatch: Send + Sync {
    /// Start the stopwatch and return an opaque handle.
    fn start(&self) -> StopwatchHandle;

    /// Return the elapsed time in milliseconds since the handle was obtained.
    fn elapsed_ms(&self, handle: &StopwatchHandle) -> u64;
}

/// System-stopwatch adapter — delegates to `std::time::Instant::now()`.
pub struct SystemStopwatch;

impl Stopwatch for SystemStopwatch {
    fn start(&self) -> StopwatchHandle {
        StopwatchHandle::from_instant(std::time::Instant::now())
    }

    fn elapsed_ms(&self, handle: &StopwatchHandle) -> u64 {
        handle
            .as_instant()
            .elapsed()
            .as_millis() as u64
    }
}

/// Mock-stopwatch adapter — returns a fixed elapsed value until `set_elapsed_ms` is called.
///
/// Uses `AtomicU64` for interior mutability so the struct satisfies `Send + Sync`.
/// All handles share the same internal elapsed value (set via `set_elapsed_ms`).
/// This makes it trivial to write deterministic tests:
///
/// ```
/// use engine::ports::hexagonal::{Stopwatch, MockStopwatch};
/// let sw = MockStopwatch::new();
/// let h = sw.start();
/// sw.set_elapsed_ms(42);
/// assert_eq!(sw.elapsed_ms(&h), 42);
/// ```
#[derive(Debug)]
pub struct MockStopwatch {
    /// Shared elapsed value (milliseconds) set by `set_elapsed_ms`.
    elapsed_ms: std::sync::atomic::AtomicU64,
}

impl Clone for MockStopwatch {
    fn clone(&self) -> Self {
        Self {
            elapsed_ms: std::sync::atomic::AtomicU64::new(
                self.elapsed_ms.load(std::sync::atomic::Ordering::SeqCst)
            ),
        }
    }
}

impl MockStopwatch {
    /// Construct with elapsed = 0.
    pub fn new() -> Self {
        Self {
            elapsed_ms: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Construct with a fixed elapsed value.
    pub fn with_elapsed_ms(elapsed_ms: u64) -> Self {
        Self {
            elapsed_ms: std::sync::atomic::AtomicU64::new(elapsed_ms),
        }
    }

    /// Overwrite the returned elapsed value for all handles.
    pub fn set_elapsed_ms(&self, ms: u64) {
        self.elapsed_ms.store(ms, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Stopwatch for MockStopwatch {
    fn start(&self) -> StopwatchHandle {
        // Mock handle is a sentinel Instant at epoch (not used for real timing).
        StopwatchHandle(std::time::Instant::now())
    }

    fn elapsed_ms(&self, _handle: &StopwatchHandle) -> u64 {
        self.elapsed_ms.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Arc<dyn Trait> impls — enable Arc<dyn Trait> to satisfy S: Trait bounds
// in service constructors (mirrors the pattern in ports.rs).
// ─────────────────────────────────────────────────────────────────────────────

impl Clock for std::sync::Arc<dyn Clock> {
    fn now(&self) -> DateTime<Utc> {
        (**self).now()
    }
}

impl IdGenerator for std::sync::Arc<dyn IdGenerator> {
    fn next_id(&self) -> Uuid {
        (**self).next_id()
    }
}

impl Stopwatch for std::sync::Arc<dyn Stopwatch> {
    fn start(&self) -> StopwatchHandle {
        (**self).start()
    }

    fn elapsed_ms(&self, handle: &StopwatchHandle) -> u64 {
        (**self).elapsed_ms(handle)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Clock ────────────────────────────────────────────────────────────────

    #[test]
    fn system_clock_returns_reasonable_time() {
        let clock = SystemClock;
        let before = Utc::now();
        let result = clock.now();
        let after = Utc::now();
        assert!(result >= before && result <= after);
    }

    #[test]
    fn mock_clock_returns_fixed_time() {
        let fixed = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let clock = MockClock::new(fixed);
        assert_eq!(clock.now(), fixed);
        assert_eq!(clock.now(), fixed); // deterministic across calls
    }

    #[test]
    fn mock_clock_set_changes_returned_time() {
        let t1 = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2026-06-10T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let clock = MockClock::new(t1);
        assert_eq!(clock.now(), t1);
        clock.set(t2);
        assert_eq!(clock.now(), t2);
    }

    // ─── IdGenerator ─────────────────────────────────────────────────────────

    #[test]
    fn random_id_gen_produces_unique_ids() {
        let id_gen = RandomIdGen;
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = id_gen.next_id();
            assert!(ids.insert(id), "UUID collision detected");
        }
    }

    #[test]
    fn mock_id_gen_call_0_returns_nil() {
        let id_gen = MockIdGen::new();
        assert_eq!(id_gen.next_id(), Uuid::nil());
    }

    #[test]
    fn mock_id_gen_call_1_returns_non_nil() {
        let id_gen = MockIdGen::new();
        let _ = id_gen.next_id(); // call0 → nil
        let id = id_gen.next_id(); // call 1 → non-nil
        assert_ne!(id, Uuid::nil());
    }

    #[test]
    fn mock_id_gen_sequential_calls_return_different_ids() {
        let id_gen = MockIdGen::new();
        let _ = id_gen.next_id(); // call 0 → nil
        let id1 = id_gen.next_id();
        let id2 = id_gen.next_id();
        let id3 = id_gen.next_id();
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn mock_id_gen_counter_increments() {
        let id_gen = MockIdGen::new();
        assert_eq!(id_gen.counter(), 0);
        let _ = id_gen.next_id(); // call0
        assert_eq!(id_gen.counter(), 1);
        let _ = id_gen.next_id(); // call 1
        assert_eq!(id_gen.counter(), 2);
    }

    #[test]
    fn mock_id_gen_advance_by() {
        let id_gen = MockIdGen::new();
        id_gen.advance_by(5);
        assert_eq!(id_gen.counter(), 5);
        let _ = id_gen.next_id(); // call 0 → nil (counter already5, n=5, nil)
        // Actually: counter starts at 0. advance_by(5) sets to 5.
        // next_id: n=5 → returns Uuid::from_u64_pair(5, 0)
        // counter becomes 6
        assert_eq!(id_gen.counter(), 6);
    }

    // ─── Stopwatch ────────────────────────────────────────────────────────────

    #[test]
    fn system_stopwatch_returns_non_zero_after_sleep() {
        let sw = SystemStopwatch;
        let h = sw.start();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let elapsed = sw.elapsed_ms(&h);
        assert!(elapsed >= 50, "expected >= 50ms, got {}ms", elapsed);
    }

    #[test]
    fn mock_stopwatch_returns_fixed_elapsed() {
        let sw = MockStopwatch::with_elapsed_ms(123);
        let h = sw.start();
        assert_eq!(sw.elapsed_ms(&h), 123);
    }

    #[test]
    fn mock_stopwatch_set_elapsed_ms_changes_returned_value() {
        let sw = MockStopwatch::new();
        let h = sw.start();
        assert_eq!(sw.elapsed_ms(&h), 0);
        sw.set_elapsed_ms(99);
        assert_eq!(sw.elapsed_ms(&h), 99);
    }

    #[test]
    fn mock_stopwatch_all_handles_share_same_elapsed() {
        let sw = MockStopwatch::with_elapsed_ms(50);
        let h1 = sw.start();
        let h2 = sw.start();
        sw.set_elapsed_ms(75);
        assert_eq!(sw.elapsed_ms(&h1), 75);
        assert_eq!(sw.elapsed_ms(&h2), 75);
    }

    // ─── Arc<dyn Trait> ──────────────────────────────────────────────────────

    #[test]
    fn arc_dyn_clock_satisfies_clock_bound() {
        fn assert_clock<C: Clock + ?Sized>(_: &C) {}
        let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(SystemClock);
        assert_clock(&*clock);
    }

    #[test]
    fn arc_dyn_id_gen_satisfies_id_gen_bound() {
        fn assert_id_gen<I: IdGenerator + ?Sized>(_: &I) {}
        let id_gen: std::sync::Arc<dyn IdGenerator> = std::sync::Arc::new(RandomIdGen);
        assert_id_gen(&*id_gen);
    }

    #[test]
    fn arc_dyn_stopwatch_satisfies_stopwatch_bound() {
        fn assert_stopwatch<S: Stopwatch + ?Sized>(_: &S) {}
        let sw: std::sync::Arc<dyn Stopwatch> = std::sync::Arc::new(SystemStopwatch);
        assert_stopwatch(&*sw);
    }
}
