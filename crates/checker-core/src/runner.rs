use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::checker::{CheckContext, CheckResult, Checker};

/// Progress of a batch run, sent after every finished target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

/// Runs a checker over many targets with bounded concurrency and cancellation.
#[derive(Debug, Clone)]
pub struct JobRunner {
    concurrency: usize,
}

impl JobRunner {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency: concurrency.max(1),
        }
    }

    /// Checks every target and returns `(index, result)` pairs sorted by index.
    ///
    /// `on_result` is called as each target finishes (in completion order).
    /// When `ctx.cancel` fires, no new targets are started and the results
    /// gathered so far are returned.
    pub async fn run<C, F>(
        &self,
        checker: Arc<C>,
        targets: Vec<C::Target>,
        ctx: &CheckContext,
        mut on_result: F,
    ) -> Vec<(usize, CheckResult)>
    where
        C: Checker + 'static,
        F: FnMut(Progress, usize, &CheckResult),
    {
        let total = targets.len();
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut set = JoinSet::new();
        let mut results = Vec::with_capacity(total);
        let mut pending = targets.into_iter().enumerate();

        loop {
            // Fill free slots with new work unless cancelled.
            while !ctx.cancel.is_cancelled() && semaphore.available_permits() > 0 {
                let Some((index, target)) = pending.next() else {
                    break;
                };
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");
                let checker = checker.clone();
                let ctx = ctx.clone();
                set.spawn(async move {
                    let _permit = permit;
                    let result = tokio::select! {
                        r = checker.check(&target, &ctx) => r,
                        _ = ctx.cancel.cancelled() => CheckResult::error(checker.kind(), "cancelled"),
                    };
                    (index, result)
                });
            }

            match set.join_next().await {
                Some(Ok((index, result))) => {
                    results.push((index, result));
                    let progress = Progress {
                        done: results.len(),
                        total,
                    };
                    let (index, result) = results.last().expect("just pushed");
                    on_result(progress, *index, result);
                }
                Some(Err(err)) => tracing::error!("check task panicked: {err}"),
                None => break,
            }
        }

        results.sort_by_key(|(index, _)| *index);
        results
    }
}

impl Default for JobRunner {
    fn default() -> Self {
        Self::new(50)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;

    use super::*;
    use crate::checker::{CheckKind, Status};

    struct SleepChecker {
        running: AtomicUsize,
        max_running: AtomicUsize,
    }

    #[async_trait]
    impl Checker for SleepChecker {
        type Target = u64;

        fn kind(&self) -> CheckKind {
            CheckKind::Tcp
        }

        async fn check(&self, ms: &u64, _ctx: &CheckContext) -> CheckResult {
            let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_running.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(*ms)).await;
            self.running.fetch_sub(1, Ordering::SeqCst);
            let status = if ms.is_multiple_of(2) {
                Status::Up
            } else {
                Status::Down
            };
            CheckResult::new(CheckKind::Tcp, status)
        }
    }

    fn checker() -> Arc<SleepChecker> {
        Arc::new(SleepChecker {
            running: AtomicUsize::new(0),
            max_running: AtomicUsize::new(0),
        })
    }

    #[tokio::test]
    async fn respects_concurrency_and_keeps_order() {
        let checker = checker();
        let targets = vec![30, 11, 20, 5, 10, 1, 8, 4];
        let mut calls = 0;
        let results = JobRunner::new(3)
            .run(
                checker.clone(),
                targets.clone(),
                &CheckContext::default(),
                |p, _, _| {
                    calls += 1;
                    assert_eq!(p.total, 8);
                    assert_eq!(p.done, calls);
                },
            )
            .await;

        assert_eq!(calls, 8);
        assert!(checker.max_running.load(Ordering::SeqCst) <= 3);
        let indices: Vec<_> = results.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, (0..8).collect::<Vec<_>>());
        assert_eq!(results[1].1.status, Status::Down);
        assert_eq!(results[0].1.status, Status::Up);
    }

    #[tokio::test]
    async fn cancellation_stops_new_work() {
        let checker = checker();
        let ctx = CheckContext::default();
        let cancel = ctx.cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            cancel.cancel();
        });
        let results = JobRunner::new(1)
            .run(checker, vec![20; 20], &ctx, |_, _, _| {})
            .await;
        assert!(results.len() < 20);
        assert_eq!(results.last().unwrap().1.status, Status::Error);
    }
}
