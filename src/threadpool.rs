use std::thread;
use std::sync::{mpsc, Arc, Mutex};

#[cfg(test)]
mod tests {
    use super::ThreadPool;
    #[test]
    fn can_construct_threadpool() {
        let pool = ThreadPool::new(8);
    }
}

// Any job to be executed by this ThreadPool must be Execute.
pub trait Execute {
    fn execute(&mut self);
}

// Any type which is Execute and Send can be dispatched via the ThreadPool.
pub type Executable = Box<Execute + Send>;

// Messages are sent from the ThreadPool to each worker.
// Each message can either be a new job, which includes an executable and unique identifier,
// or the Terminate signal, which signals the Worker to stop listening for new jobs.
enum Message {
    Job(Executable, usize),
    Terminate,
}

// WorkerStatus is used by each thread to report when it is finished.
// As part of this message, the worker also sends back the Executable and the job ID.
// The executable may or may not be stateful.
pub enum WorkerStatus {
    Complete(Executable, usize),
}

// Worker manages a single thread. It can receive jobs via the associated mpsc::Receiver.
// Each job should be accompanied by an id so it can be identified.
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    // Creates a new worker with the given ID. Also sets up a receiver to listen for jobs.
    fn new(id: usize, exec_receiver: Arc<Mutex<mpsc::Receiver<Message>>>, sender: mpsc::Sender<WorkerStatus>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // Listen for jobs. This blocks and is NOT a busy wait.
                let message = exec_receiver.lock().unwrap().recv().unwrap();
                match message {
                    // New jobs are executed, and a status message is returned which packages the
                    // included Executable and its id.
                    Message::Job(mut exec, exec_id) => {
                        exec.execute();
                        match sender.send(WorkerStatus::Complete(exec, exec_id)) {
                            Ok(_) => (),
                            Err(what) => panic!("Worker {} could not send exec {} status.\n{}", id, exec_id, what),
                        };
                    },
                    // Terminate will break out of the loop, so that this Worker
                    // is no longer listening for jobs.
                    Message::Terminate => {
                        break;
                    }
                }
            }
        });
        return Worker{id: id, thread: Some(thread)};
    }
}

// The ThreadPool tracks a group of workers. When a new Executable is received, it is moved into
// a worker where it is executed. Afterwards, it can be moved back by listening on
// the wstatus_receiver.
pub struct ThreadPool {
    workers: Vec<Worker>,
    exec_sender: mpsc::Sender<Message>,
    pub wstatus_receiver: mpsc::Receiver<WorkerStatus>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let (exec_sender, exec_receiver) = mpsc::channel();
        let exec_receiver = Arc::new(Mutex::new(exec_receiver));

        let (wstatus_sender, wstatus_receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&exec_receiver), wstatus_sender.clone()));
        }

        return ThreadPool{workers: workers, exec_sender: exec_sender, wstatus_receiver: wstatus_receiver};
    }

    pub fn execute(&self, exec: Executable, id: usize) {
        self.exec_sender.send(Message::Job(exec, id)).unwrap();
    }
}

// Implements graceful shutdown and clean up for the ThreadPool.
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.exec_sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
