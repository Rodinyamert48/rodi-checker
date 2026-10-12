use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

pub type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Job = Arc<dyn Fn() -> BoxFuture + Send + Sync>;

/// Runs registered jobs periodically. Re-registering an id replaces the job.
#[derive(Clone, Default)]
pub struct Scheduler {
    jobs: Arc<Mutex<HashMap<i64, CancellationToken>>>,
    shutdown: CancellationToken,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedules `job` every `interval`, first run immediately.
    pub fn schedule(&self, id: i64, interval: Duration, job: Job) {
        let token = self.shutdown.child_token();
        if let Some(old) = self.jobs.lock().unwrap().insert(id, token.clone()) {
            old.cancel();
        }
        let interval = interval.max(Duration::from_secs(1));
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = ticker.tick() => {
                        tokio::select! {
                            _ = token.cancelled() => break,
                            _ = job() => {}
                        }
                    }
                }
            }
        });
    }

    pub fn unschedule(&self, id: i64) {
        if let Some(token) = self.jobs.lock().unwrap().remove(&id) {
            token.cancel();
        }
    }

    pub fn is_scheduled(&self, id: i64) -> bool {
        self.jobs.lock().unwrap().contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Stops every job.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
        self.jobs.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn counting_job(counter: Arc<AtomicUsize>) -> Job {
        Arc::new(move || {
            let counter = counter.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        })
    }

    #[tokio::test(start_paused = true)]
    async fn runs_periodically_until_unscheduled() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        scheduler.schedule(1, Duration::from_secs(10), counting_job(counter.clone()));

        tokio::time::sleep(Duration::from_secs(25)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 3); // t = 0, 10, 20

        scheduler.unschedule(1);
        assert!(!scheduler.is_scheduled(1));
        tokio::time::sleep(Duration::from_secs(30)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn rescheduling_replaces_job() {
        let scheduler = Scheduler::new();
        let first = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        scheduler.schedule(7, Duration::from_secs(5), counting_job(first.clone()));
        tokio::time::sleep(Duration::from_secs(1)).await;
        scheduler.schedule(7, Duration::from_secs(5), counting_job(second.clone()));
        tokio::time::sleep(Duration::from_secs(12)).await;

        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 3);
        assert_eq!(scheduler.len(), 1);
        scheduler.shutdown();
        assert!(scheduler.is_empty());
    }
}
