use std::thread::{self, JoinHandle};
use std::sync::{mpsc, Arc, Mutex};

type Job = Box<dyn Send + 'static + FnOnce()>;

struct Worker {
    // receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
    id: usize,
    // If there is some thread, then the worker is running
    thread: Option<JoinHandle<()>>
}

pub struct ThreadPool {
    sender: Option<mpsc::Sender<Job>>,
    workers: Vec<Worker>
}

impl Worker {
    #[allow(clippy::while_let_loop)]
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            match receiver.lock().unwrap().recv() {
                Ok(job) => job(),
                Err(_) => break,
            }
        });
        Worker { id, thread: Some(thread) }
    }
}

/// Simple thread pool that implement graceful shutdown
impl ThreadPool {
    /// Creates a new thread pool with a given amount of workers.
    ///
    /// # Panics
    /// Panics if `n` is zero
    pub fn new(n: usize) -> Self {
        assert!(n > 0);
        let (sender, receiver) = mpsc::channel::<Job>();
        let mut workers = Vec::with_capacity(n);

        let receiver = Arc::new(Mutex::new(receiver));
        for id in  0..n {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    /// Runs a given job.
    ///
    /// There is no guarantee that the job will be executed immediately.
    /// Any free worker is able to take the job and execute it.
    pub fn run<F>(&self, job: F)
    where F: FnOnce() + Send + 'static
    {
        let job = Box::new(job);
        self.sender.as_ref().unwrap().send(job).expect("Receiver channel is still opened");
    }

    pub fn with_same_workers_as_cpus() -> std::io::Result<Self> {
        let cpus = std::thread::available_parallelism()?;
        Ok(ThreadPool::new(cpus.get()))
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drops the sender so all workers stop waiting for jobs
        drop(self.sender.take().unwrap());

        // Waits for all workers to finish
        for worker in &mut self.workers {
            println!("joining thread {id}", id=worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("Couldn't join thread");
            }
        }
    }
}
