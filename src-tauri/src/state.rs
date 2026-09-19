// Shared app state: concurrency limiter (TaskManager) + cancellation registry.
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex};

struct TmInner {
    limit: usize,
    active: usize,
}

pub struct TaskManager {
    inner: Mutex<TmInner>,
    cond: Condvar,
}

impl TaskManager {
    pub fn new(limit: usize) -> Self {
        Self { inner: Mutex::new(TmInner { limit: limit.max(1), active: 0 }), cond: Condvar::new() }
    }
    pub fn set_limit(&self, n: usize) {
        let mut g = self.inner.lock().unwrap();
        g.limit = n.max(1);
        self.cond.notify_all();
    }
    pub fn limit(&self) -> usize {
        self.inner.lock().unwrap().limit
    }
    // block while active >= limit, then take a slot
    pub fn acquire(&self) {
        let mut g = self.inner.lock().unwrap();
        while g.active >= g.limit {
            g = self.cond.wait(g).unwrap();
        }
        g.active += 1;
    }
    pub fn release(&self) {
        let mut g = self.inner.lock().unwrap();
        if g.active > 0 {
            g.active -= 1;
        }
        self.cond.notify_all();
    }
}

pub type CancelRegistry = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

pub struct AppState {
    pub tasks: Arc<TaskManager>,
    cancels: CancelRegistry,
}

impl AppState {
    pub fn new(concurrency: usize) -> Self {
        Self {
            tasks: Arc::new(TaskManager::new(concurrency)),
            cancels: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn registry(&self) -> CancelRegistry {
        self.cancels.clone()
    }
    pub fn register_cancel(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels.lock().unwrap().insert(job_id.to_string(), flag.clone());
        flag
    }
    pub fn cancel(&self, job_id: &str) {
        if let Some(f) = self.cancels.lock().unwrap().get(job_id) {
            f.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    pub fn unregister_cancel(&self, job_id: &str) {
        self.cancels.lock().unwrap().remove(job_id);
    }
}
