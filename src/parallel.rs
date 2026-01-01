use crossbeam_channel::{Sender, bounded};
use std::{marker::PhantomData, thread};

pub fn work_pool<'scope, F, B, R>(body_fn: B) -> R
where
    F: FnOnce() + Send + 'scope,
    B: (Fn(WorkPool<F>) -> R) + 'scope,
{
    let threads = thread::available_parallelism()
        .map(|n| n.into())
        .unwrap_or(1);
    work_pool_with_sizes(threads, threads * 2, body_fn)
}

pub fn work_pool_with_sizes<'scope, F, B, R>(thread_count: usize, chan_size: usize, body_fn: B) -> R
where
    F: FnOnce() + Send + 'scope,
    B: (Fn(WorkPool<F>) -> R) + 'scope,
{
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

        body_fn(WorkPool {
            tx,
            _phantom: PhantomData,
        })
    })
}

pub struct WorkPool<'scope, F>
where
    F: FnOnce() + Send + 'scope,
{
    tx: Sender<F>,
    _phantom: PhantomData<&'scope F>,
}

impl<'scope, F> WorkPool<'scope, F>
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

        work_pool(|pool| {
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
