//! Worker registry for tracking connected workers.
//!
//! Manages worker registration, heartbeats, and cleanup of stale workers.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Default heartbeat timeout (90 seconds - 3x heartbeat interval).
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(90);

/// Information about a registered worker.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Worker ID.
    pub id: String,
    /// Queues this worker processes.
    pub queues: Vec<String>,
    /// Maximum concurrent jobs.
    pub concurrency: u32,
    /// Registration timestamp.
    pub registered_at: Instant,
    /// Last heartbeat timestamp.
    pub last_heartbeat: Instant,
    /// Number of active jobs.
    pub active_jobs: u32,
    /// Total jobs processed by this worker.
    pub jobs_processed: u64,
    /// Total jobs failed by this worker.
    pub jobs_failed: u64,
}

impl WorkerInfo {
    /// Create a new worker info.
    fn new(id: String, queues: Vec<String>, concurrency: u32) -> Self {
        let now = Instant::now();
        Self {
            id,
            queues,
            concurrency,
            registered_at: now,
            last_heartbeat: now,
            active_jobs: 0,
            jobs_processed: 0,
            jobs_failed: 0,
        }
    }

    /// Check if the worker is considered alive based on heartbeat timeout.
    pub fn is_alive(&self, timeout: Duration) -> bool {
        self.last_heartbeat.elapsed() < timeout
    }

    /// Update the heartbeat timestamp and active job count.
    fn heartbeat(&mut self, active_jobs: u32) {
        self.last_heartbeat = Instant::now();
        self.active_jobs = active_jobs;
    }
}

/// Worker registry for managing connected workers.
pub struct WorkerRegistry {
    /// Registered workers by ID.
    workers: RwLock<HashMap<String, WorkerInfo>>,
    /// Heartbeat timeout duration.
    heartbeat_timeout: Duration,
    /// Counter for total registrations.
    registration_count: AtomicU64,
}

impl WorkerRegistry {
    /// Create a new worker registry.
    pub fn new() -> Self {
        Self {
            workers: RwLock::new(HashMap::new()),
            heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
            registration_count: AtomicU64::new(0),
        }
    }

    /// Create a new worker registry with custom heartbeat timeout.
    pub fn with_timeout(heartbeat_timeout: Duration) -> Self {
        Self {
            workers: RwLock::new(HashMap::new()),
            heartbeat_timeout,
            registration_count: AtomicU64::new(0),
        }
    }

    /// Register a worker.
    ///
    /// Returns the registration sequence number.
    pub fn register(&self, worker_id: &str, queues: Vec<String>, concurrency: u32) -> u64 {
        let info = WorkerInfo::new(worker_id.to_string(), queues.clone(), concurrency);

        let seq = self.registration_count.fetch_add(1, Ordering::Relaxed) + 1;

        self.workers.write().insert(worker_id.to_string(), info);

        info!(
            worker_id = %worker_id,
            queues = ?queues,
            concurrency = concurrency,
            registration_seq = seq,
            "Worker registered"
        );

        seq
    }

    /// Update worker heartbeat.
    ///
    /// Returns true if the worker is registered and the heartbeat was updated.
    pub fn heartbeat(&self, worker_id: &str, active_jobs: u32) -> bool {
        let mut workers = self.workers.write();

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.heartbeat(active_jobs);
            debug!(
                worker_id = %worker_id,
                active_jobs = active_jobs,
                "Worker heartbeat received"
            );
            true
        } else {
            warn!(worker_id = %worker_id, "Heartbeat from unknown worker");
            false
        }
    }

    /// Check if a worker is alive (registered and heartbeat not expired).
    pub fn is_worker_alive(&self, worker_id: &str) -> bool {
        self.workers
            .read()
            .get(worker_id)
            .map(|w| w.is_alive(self.heartbeat_timeout))
            .unwrap_or(false)
    }

    /// Unregister a worker.
    pub fn unregister(&self, worker_id: &str) -> bool {
        let removed = self.workers.write().remove(worker_id).is_some();
        if removed {
            info!(worker_id = %worker_id, "Worker unregistered");
        }
        removed
    }

    /// Cleanup stale workers that have missed heartbeats.
    ///
    /// Returns the list of removed worker IDs.
    pub fn cleanup_stale_workers(&self) -> Vec<String> {
        let mut workers = self.workers.write();
        let timeout = self.heartbeat_timeout;

        let stale_ids: Vec<String> = workers
            .iter()
            .filter(|(_, w)| !w.is_alive(timeout))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &stale_ids {
            workers.remove(id);
            warn!(worker_id = %id, "Removed stale worker");
        }

        stale_ids
    }

    /// Get information about a specific worker.
    pub fn get_worker(&self, worker_id: &str) -> Option<WorkerInfo> {
        self.workers.read().get(worker_id).cloned()
    }

    /// Get all registered workers.
    pub fn get_all_workers(&self) -> Vec<WorkerInfo> {
        self.workers.read().values().cloned().collect()
    }

    /// Get workers that process a specific queue.
    pub fn get_workers_for_queue(&self, queue: &str) -> Vec<WorkerInfo> {
        self.workers
            .read()
            .values()
            .filter(|w| w.queues.contains(&queue.to_string()))
            .cloned()
            .collect()
    }

    /// Get the count of active workers.
    pub fn active_worker_count(&self) -> usize {
        let timeout = self.heartbeat_timeout;
        self.workers
            .read()
            .values()
            .filter(|w| w.is_alive(timeout))
            .count()
    }

    /// Get total registration count.
    pub fn total_registrations(&self) -> u64 {
        self.registration_count.load(Ordering::Relaxed)
    }

    /// Increment job processed count for a worker.
    pub fn record_job_processed(&self, worker_id: &str) {
        if let Some(worker) = self.workers.write().get_mut(worker_id) {
            worker.jobs_processed += 1;
        }
    }

    /// Increment job failed count for a worker.
    pub fn record_job_failed(&self, worker_id: &str) {
        if let Some(worker) = self.workers.write().get_mut(worker_id) {
            worker.jobs_failed += 1;
        }
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_registration() {
        let registry = WorkerRegistry::new();

        let seq = registry.register("worker-1", vec!["default".to_string()], 4);
        assert_eq!(seq, 1);

        let seq = registry.register("worker-2", vec!["high".to_string()], 2);
        assert_eq!(seq, 2);

        assert!(registry.is_worker_alive("worker-1"));
        assert!(registry.is_worker_alive("worker-2"));
        assert!(!registry.is_worker_alive("worker-3"));
    }

    #[test]
    fn test_worker_heartbeat() {
        let registry = WorkerRegistry::new();

        registry.register("worker-1", vec!["default".to_string()], 4);

        assert!(registry.heartbeat("worker-1", 2));
        assert!(!registry.heartbeat("unknown-worker", 0));

        let worker = registry.get_worker("worker-1").unwrap();
        assert_eq!(worker.active_jobs, 2);
    }

    #[test]
    fn test_stale_worker_cleanup() {
        let registry = WorkerRegistry::with_timeout(Duration::from_millis(10));

        registry.register("worker-1", vec!["default".to_string()], 4);

        // Worker should be alive initially
        assert!(registry.is_worker_alive("worker-1"));

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(20));

        // Worker should now be stale
        assert!(!registry.is_worker_alive("worker-1"));

        // Cleanup should remove the worker
        let stale = registry.cleanup_stale_workers();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0], "worker-1");

        // Worker should be gone
        assert!(registry.get_worker("worker-1").is_none());
    }

    #[test]
    fn test_get_workers_for_queue() {
        let registry = WorkerRegistry::new();

        registry.register("worker-1", vec!["default".to_string(), "high".to_string()], 4);
        registry.register("worker-2", vec!["high".to_string()], 2);
        registry.register("worker-3", vec!["low".to_string()], 1);

        let high_workers = registry.get_workers_for_queue("high");
        assert_eq!(high_workers.len(), 2);

        let default_workers = registry.get_workers_for_queue("default");
        assert_eq!(default_workers.len(), 1);

        let low_workers = registry.get_workers_for_queue("low");
        assert_eq!(low_workers.len(), 1);
    }

    #[test]
    fn test_unregister() {
        let registry = WorkerRegistry::new();

        registry.register("worker-1", vec!["default".to_string()], 4);
        assert!(registry.is_worker_alive("worker-1"));

        assert!(registry.unregister("worker-1"));
        assert!(!registry.is_worker_alive("worker-1"));

        assert!(!registry.unregister("worker-1")); // Already removed
    }

    #[test]
    fn test_job_recording() {
        let registry = WorkerRegistry::new();

        registry.register("worker-1", vec!["default".to_string()], 4);

        registry.record_job_processed("worker-1");
        registry.record_job_processed("worker-1");
        registry.record_job_failed("worker-1");

        let worker = registry.get_worker("worker-1").unwrap();
        assert_eq!(worker.jobs_processed, 2);
        assert_eq!(worker.jobs_failed, 1);
    }
}
