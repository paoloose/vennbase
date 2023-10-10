use std::sync::atomic::{Ordering, AtomicUsize};
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};

pub struct ThreadPool {
    limit: usize,
    current: Arc<AtomicUsize>,
    pool: Arc<Mutex<Vec<JoinHandle<()>>>>
}

impl ThreadPool {
    pub fn new(n: usize) -> Self {
        assert!(n > 0);
        ThreadPool {
            limit: n,
            current: Arc::new(AtomicUsize::new(0)),
            pool: Arc::new(Mutex::new(Vec::with_capacity(n)))
        }
    }

    pub fn spawn<F, T>(&mut self, f: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static
    {
        let current = self.current.load(Ordering::Relaxed);
        dbg!(current);
        if current < self.limit {
            let current = self.current.clone();
            current.fetch_add(1, Ordering::Relaxed);
            let pool = self.pool.clone();
            let handle = thread::spawn(move || {
                f();
                let mut pool = pool.lock().unwrap();
                pool.retain(|handle| handle.thread().id() != thread::current().id());
                current.fetch_sub(1, Ordering::Relaxed);
                println!("pool len: {len}", len=pool.len());
            });
            self.pool.lock().unwrap().push(handle);
        }
        else {
            let mut pool = self.pool.lock().unwrap();
            let handle = pool.pop().unwrap();
            handle.join().unwrap();

            let current = self.current.clone();
            current.fetch_add(1, Ordering::Relaxed);
            let pool = self.pool.clone();
            let handle = thread::spawn(move || {
                f();
                let mut pool = pool.lock().unwrap();
                pool.retain(|handle| handle.thread().id() != thread::current().id());
                current.fetch_sub(1, Ordering::Relaxed);
                println!("pool len: {len}", len=pool.len());
            });
            self.pool.lock().unwrap().push(handle);
        }
    }
}
