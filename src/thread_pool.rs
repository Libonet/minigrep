//! Simple implementation of a ThreadPool to make searches with multithreading
//!
//! ## Example usage
//!
//! ```rust
//! use minigrep::thread_pool::ThreadPool;
//!
//! let thread_count = 4;    
//! let pool = ThreadPool::new(thread_count);
//!
//! let expensive_search_function = || { /* expensive stuff!!! */ };
//!
//! pool.execute(expensive_search_function);
//! ```

use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

/// Structure that handles creation of workers
/// and communication
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for _id in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Send a function to be handled when available.
    pub fn execute<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(function);

        self.sender.as_ref().unwrap().send(job).unwrap_or_default();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap_or(());
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver
                .lock()
                .expect("Poisoned mutex. Killing worker! :D")
                .recv();

            match message {
                Ok(job) => {
                    job();
                }
                Err(_) => {
                    break;
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}
