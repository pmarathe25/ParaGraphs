// TODO: For now, since casting is not possible, Nodes should be strongly typed as well.
// This means that the graph would be homogenous (behavior would come from
// the state of the node though), but that may be better than the way it is now.
use std::thread;
use std::sync::{mpsc, Arc, Mutex};

#[cfg(test)]
mod tests {
    use super::{ThreadPool, Execute, Executable, WorkerStatus};
    use std::thread;
    use std::time;

    // Adds two numbers together and stores in sum.
    struct Adder {
        lhs: i32,
        rhs: i32,
        sum: i32,
    }

    impl Adder {
        fn new(lhs: i32, rhs: i32) -> Adder {
            return Adder{lhs: lhs, rhs: rhs, sum: 0};
        }
    }

    impl Execute<i32> for Adder {
        fn execute(&mut self) {
            self.sum = self.lhs + self.rhs;
        }

        fn result(&self) -> i32 {
            return self.sum;
        }
    }

    #[test]
    fn can_construct_threadpool() {
        let pool = ThreadPool::<u32>::new(8);
    }

    #[test]
    fn can_launch_jobs() {
        // Launch some arbitrary job, like waiting.
        let pool = ThreadPool::new(8);
        for i in 0..8 {
            let adder = Box::new(Adder::new(i, 1));
            pool.execute(adder, i as usize);
        }
    }

    #[test]
    fn can_move_jobs_to_pool() {
        // Try to create a vector of executables, and then Send them to the pool.
        // We need to use Option here so we can safely move them back and forth.
        let num_adders = 8;
        let mut adders: Vec<Option<Box<Adder>>> = std::iter::repeat_with(|| return Some(Box::new(Adder::new(1, 5)))).take(num_adders).collect();

        let pool = ThreadPool::new(8);
        for (index, adder) in adders.iter_mut().enumerate() {
            pool.execute(adder.take().unwrap(), index);
        }
        assert_eq!(adders.len(), num_adders);
    }

    #[test]
    fn can_retrieve_jobs_from_pool() {
        // Try to create a vector of executables, and then Send them to the pool.
        // We need to use Option here so we can safely move them back and forth.
        let num_adders = 8;
        let mut adders: Vec<Option<Executable<_>>> = Vec::with_capacity(num_adders);
        for i in 0..num_adders {
            let incrementer = Box::new(Adder::new(1, i as i32));
            adders.push(Some(incrementer));
        }
        // Create a pool and puh all the adders.
        let pool = ThreadPool::new(8);
        for (index, adder) in adders.iter_mut().enumerate() {
            pool.execute(adder.take().unwrap(), index);
        }
        assert_eq!(adders.len(), num_adders);
        // Next, try to retrieve the jobs. We know that there are exactly num_adders jobs.
        let mut num_running_jobs = num_adders.clone();
        while num_running_jobs > 0 {
            if let Ok(wstatus) = pool.wstatus_receiver.recv() {
                match wstatus {
                    WorkerStatus::Complete(adder, id) => {
                        // Populate the Option in the adders vector.
                        if let Some(opt) = adders.get_mut(id) {
                            opt.replace(adder);
                        }
                        println!("Replacing {}", id);
                        num_running_jobs -= 1;
                    },
                };
            };
        }
        for (index, adder) in adders.into_iter().enumerate() {
            println!("Checking adder {}", index);
            assert_eq!(adder.unwrap().result(), 1 + index as i32);
            println!("Adder {} passed", index);
        }
    }
}

// Any job to be executed by this ThreadPool must be Execute.
pub trait Execute<Edge> {
    fn execute(&mut self);

    fn result(&self) -> Edge;
}

// Any type which is Execute and Send can be dispatched via the ThreadPool.
pub type Executable<Edge> = Box<dyn Execute<Edge> + Send>;

// Messages are sent from the ThreadPool to each worker.
// Each message can either be a new job, which includes an executable and unique identifier,
// or the Terminate signal, which signals the Worker to stop listening for new jobs.
enum Message<Edge> {
    Job(Executable<Edge>, usize),
    Terminate,
}

// WorkerStatus is used by each thread to report when it is finished.
// As part of this message, the worker also sends back the Executable and the job ID.
// The executable may or may not be stateful.
pub enum WorkerStatus<Edge> {
    Complete(Executable<Edge>, usize),
}

// Worker manages a single thread. It can receive jobs via the associated mpsc::Receiver.
// Each job should be accompanied by an id so it can be identified.
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    // Creates a new worker with the given ID. Also sets up a receiver to listen for jobs.
    fn new<Edge: 'static>(id: usize, exec_receiver: Arc<Mutex<mpsc::Receiver<Message<Edge>>>>, sender: mpsc::Sender<WorkerStatus<Edge>>) -> Worker {
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
pub struct ThreadPool<Edge> {
    workers: Vec<Worker>,
    exec_sender: mpsc::Sender<Message<Edge>>,
    pub wstatus_receiver: mpsc::Receiver<WorkerStatus<Edge>>,
}

impl<Edge: 'static> ThreadPool<Edge> {
    pub fn new(num_workers: usize) -> ThreadPool<Edge> {
        assert!(num_workers > 0);
        let (exec_sender, exec_receiver) = mpsc::channel();
        let exec_receiver = Arc::new(Mutex::new(exec_receiver));

        let (wstatus_sender, wstatus_receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(num_workers);
        for id in 0..num_workers {
            workers.push(Worker::new(id, Arc::clone(&exec_receiver), wstatus_sender.clone()));
        }

        return ThreadPool{workers: workers, exec_sender: exec_sender, wstatus_receiver: wstatus_receiver};
    }

    pub fn execute(&self, exec: Executable<Edge>, id: usize) {
        self.exec_sender.send(Message::Job(exec, id)).unwrap();
    }
}

// Implements graceful shutdown and clean up for the ThreadPool.
impl<Edge> Drop for ThreadPool<Edge> {
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
