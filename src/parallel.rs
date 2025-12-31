use crossbeam_channel::{Sender, bounded};
use std::thread;

pub struct WorkPool<T: Send + 'static> {
    tx: Sender<T>,
}

pub fn run_pool<'scope, T, W, B>(thread_count: usize, chan_size: usize, work_fn: W, body_fn: B)
where
    T: Send + 'static,
    W: Fn(T) + Send + Clone + 'scope,
    B: Fn(WorkPool<T>) + 'scope,
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

        body_fn(WorkPool { tx });
    });
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

        run_pool(
            8,
            32,
            |i| {
                let p = is_prime(i);
                let mut r = res_lock.lock().unwrap();
                r.insert(i, p);
            },
            |pool| {
                pool.send(5);
                pool.send(10);
                pool.send(15);
                pool.send(19);
            },
        );

        let results = res_lock.lock().unwrap();
        let mut res_pairs: Vec<_> = results.iter().collect();
        res_pairs.sort();
        assert_eq!(
            res_pairs,
            &[(&5, &true), (&10, &false), (&15, &false), (&19, &true)]
        );
    }
}
