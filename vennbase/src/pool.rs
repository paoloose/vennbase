use std::thread::{self, JoinHandle};
use std::sync::{mpsc, Arc, Mutex};

type Job = Box<dyn Send + 'static + FnOnce() -> ()>;

struct Worker {
    // receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
    thread: JoinHandle<()>
}

pub struct ThreadPool {
    sender: mpsc::Sender<Job>,
    workers: Vec<Worker>
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            println!("worker {id}: received job");
            job();
        });
        Worker { thread }
    }
}

impl ThreadPool {
    pub fn new(n: usize) -> Self {
        assert!(n > 0);
        let (sender, receiver) = mpsc::channel::<Job>();
        let mut workers = Vec::with_capacity(n);

        let receiver = Arc::new(Mutex::new(receiver));
        for id in  0..n {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn spawn<F>(&self, f: F)
    where F: FnOnce() -> () + Send + 'static
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}
