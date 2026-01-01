use crossbeam_channel::{Sender, bounded};
use std::num::NonZero;
use std::{marker::PhantomData, thread};

/// Create a thread pool and provide it to a scope that can spawn work.
///
/// The thread pool has a number of threads equal to the number of available
/// hardware threads. The work-item buffer has a size equal to double that.
pub fn scope<'scope, F, B, R>(body_fn: B) -> R
where
    F: FnOnce() + Send + 'scope,
    B: (Fn(ThreadPool<F>) -> R) + 'scope,
{
    scope_with_threads(None, body_fn)
}

/// Create a thread pool with an optionally specified number of threads.
///
/// Like `scope` but with a specific number of threads.
pub fn scope_with_threads<'scope, F, B, R>(thread_count: Option<NonZero<usize>>, body_fn: B) -> R
where
    F: FnOnce() + Send + 'scope,
    B: (Fn(ThreadPool<F>) -> R) + 'scope,
{
    let threads = thread_count
        .or_else(|| thread::available_parallelism().ok())
        .map(|n| n.get())
        .unwrap_or(1);
    scope_with_sizes(threads, threads * 2, body_fn)
}

/// Create a thread pool with a specified number of threads and buffer slots.
///
/// Like `scope` but with fewer defaults.
pub fn scope_with_sizes<'scope, F, B, R>(thread_count: usize, chan_size: usize, body_fn: B) -> R
where
    F: FnOnce() + Send + 'scope,
    B: (Fn(ThreadPool<F>) -> R) + 'scope,
{
    assert!(thread_count > 0);
    assert!(chan_size > 0);

    thread::scope(|s| {
        let (tx, rx) = bounded::<F>(chan_size);

        for _ in 0..thread_count {
            let thread_rx = rx.clone();
            s.spawn(move || {
                while let Ok(func) = thread_rx.recv() {
                    func();
                }
            });
        }

        body_fn(ThreadPool {
            tx,
            _phantom: PhantomData,
        })
    })
}

/// A running hread pool that can accept work.
pub struct ThreadPool<'scope, F>
where
    F: FnOnce() + Send + 'scope,
{
    tx: Sender<F>,
    _phantom: PhantomData<&'scope F>,
}

impl<'scope, F> ThreadPool<'scope, F>
where
    F: FnOnce() + Send + 'scope,
{
    pub fn spawn(&self, func: F) {
        self.tx.send(func).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn is_prime(i: u64) -> bool {
        for j in 2..i {
            if i % j == 0 {
                return false;
            }
        }
        true
    }

    #[test]
    fn simple() {
        let results = HashMap::<u64, bool>::new();
        let res_lock = Arc::new(Mutex::new(results));

        scope(|pool| {
            for i in [5, 10, 15, 19] {
                let lock = res_lock.clone();
                pool.spawn(move || {
                    let p = is_prime(i);
                    lock.lock().unwrap().insert(i, p);
                });
            }
        });

        let results = Arc::try_unwrap(res_lock).unwrap().into_inner().unwrap();
        let mut res_pairs: Vec<_> = results.into_iter().collect();
        res_pairs.sort();
        assert_eq!(res_pairs, [(5, true), (10, false), (15, false), (19, true)]);
    }
}
