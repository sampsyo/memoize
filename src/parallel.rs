use crossbeam_channel::{Sender, bounded};
use std::thread;

pub fn work_pool<'scope, T, W, B, R>(work_fn: W, body_fn: B) -> R
where
    T: Send + 'static,
    W: Fn(T) + Send + Clone + 'scope,
    B: (Fn(WorkPool<T>) -> R) + 'scope,
{
    let threads = thread::available_parallelism()
        .map(|n| n.into())
        .unwrap_or(1);
    work_pool_with_sizes(threads, threads * 2, work_fn, body_fn)
}

pub fn work_pool_with_sizes<'scope, T, W, B, R>(
    thread_count: usize,
    chan_size: usize,
    work_fn: W,
    body_fn: B,
) -> R
where
    T: Send + 'static,
    W: Fn(T) + Send + Clone + 'scope,
    B: (Fn(WorkPool<T>) -> R) + 'scope,
{
    thread::scope(|s| {
        let (tx, rx) = bounded(chan_size);

        for _ in 0..thread_count {
            let thread_rx = rx.clone();
            let thread_work = work_fn.clone();
            s.spawn(move || {
                while let Ok(val) = thread_rx.recv() {
                    thread_work(val);
                }
            });
        }

        body_fn(WorkPool { tx })
    })
}

pub struct WorkPool<T: Send + 'static> {
    tx: Sender<T>,
}

impl<T: Send + 'static> WorkPool<T> {
    pub fn send(&self, value: T) {
        self.tx.send(value).unwrap();
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

        work_pool(
            |i| {
                let p = is_prime(i);
                res_lock.lock().unwrap().insert(i, p);
            },
            |pool| {
                pool.send(5);
                pool.send(10);
                pool.send(15);
                pool.send(19);
            },
        );

        let results = Arc::try_unwrap(res_lock).unwrap().into_inner().unwrap();
        let mut res_pairs: Vec<_> = results.into_iter().collect();
        res_pairs.sort();
        assert_eq!(res_pairs, [(5, true), (10, false), (15, false), (19, true)]);
    }
}
