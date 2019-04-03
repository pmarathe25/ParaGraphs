use std::thread;
use std::sync::{mpsc, Arc, Mutex};

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;
    use super::{ThreadPool, ThreadExecute, WorkerStatus};

    // Adds two numbers together.
    #[derive(Debug, Clone)]
    pub(crate) struct Adder {
        pub(crate) valid: bool,
    }

    impl Adder {
        pub(crate) fn new() -> Adder {
            return Adder{valid: true};
        }
    }

    impl ThreadExecute<i32> for Adder {
        fn execute(&mut self, inputs: Vec<Arc<i32>>) -> Option<i32> {
            let mut sum = 0;
            for inp in inputs {
                sum += *inp;
            }
            return Some(sum);
        }
    }

    #[test]
    fn can_construct_threadpool() {
        let _pool = ThreadPool::<Adder, i32>::new(8);
    }

    #[test]
    fn can_launch_jobs() {
        // Launch some arbitrary job, like adding.
        let pool = ThreadPool::new(8);
        for i in 0..8 {
            let adder = Adder::new();
            pool.execute(adder, vec!(Arc::new(1), Arc::new(i)), i as usize);
        }
    }

    #[test]
    fn can_move_jobs_to_pool() {
        // Try to create a vector of nodes, and then Send them to the pool.
        // We need to use Option here so we can safely move them back and forth.
        const NUM_ADDERS: usize = 8;
        let mut adders: Vec<Option<Adder>> = vec![Some(Adder::new()); NUM_ADDERS];

        let pool = ThreadPool::new(8);
        for (index, adder) in adders.iter_mut().enumerate() {
            pool.execute(adder.take().unwrap(), vec!(Arc::new(index as i32), Arc::new(1)), index);
        }
        assert_eq!(adders.len(), NUM_ADDERS);
    }

    #[test]
    fn can_retrieve_jobs_from_pool() {
        // Try to create a vector of nodes, and then Send them to the pool.
        // We need to use Option here so we can safely move them back and forth.
        const NUM_ADDERS: usize = 8;
        let mut adders: Vec<Option<Adder>> = vec![Some(Adder::new()); NUM_ADDERS];
        // Create a pool and puh all the adders.
        let pool = ThreadPool::new(8);
        for (index, adder) in adders.iter_mut().enumerate() {
            pool.execute(adder.take().unwrap(), vec!(Arc::new(1), Arc::new(index as i32)), index);
        }
        assert_eq!(adders.len(), NUM_ADDERS);
        // Next, try to retrieve the jobs. We know that there are exactly NUM_ADDERS jobs.
        let mut num_running_jobs = NUM_ADDERS.clone();
        let mut results = vec![0; NUM_ADDERS];
        while num_running_jobs > 0 {
            if let Ok(wstatus) = pool.wstatus_receiver.recv() {
                match wstatus {
                    WorkerStatus::Complete(adder, result, id) => {
                        // Populate the Option in the adders vector.
                        if let Some(opt) = adders.get_mut(id) {
                            opt.replace(adder);
                        }
                        if let Some(elem) = results.get_mut(id) {
                            *elem = result;
                        }
                        println!("Replacing {}", id);
                        num_running_jobs -= 1;
                    },
                    WorkerStatus::Fail(id) => panic!("Worker {} failed to execute", id),
                };
            };
        }
        println!("{:?}", results);
        for (index, result) in results.into_iter().enumerate() {
            println!("Checking result {}", index);
            assert_eq!(result, 1 + index as i32);
            println!("Adder {} passed", index);
        }
    }
}

// Any job to be executed by the ThreadPool must be ThreadExecute.
/// A trait for the ability to be dispatched to and exectued on a separate thread.
pub trait ThreadExecute<Data> : Send where Data: Send + Sync {

    /// Executes with the provided inputs. This function must not panic, or chaos ensues.
    /// Instead, upon error, the function should return None.
    ///
    /// # Arguments
    ///
    /// * `inputs` - The inputs to use, in order.
    ///
    /// # Example
    ///
    /// ```
    /// use paragraphs::ThreadExecute;
    /// use std::sync::Arc;
    /// struct Adder;
    ///
    /// impl ThreadExecute<i32> for Adder {
    ///     fn execute(&mut self, inputs: Vec<Arc<i32>>) -> Option<i32> {
    ///         return Some(inputs.into_iter().map(|x| *x).sum());
    ///     }
    /// }
    /// ```
    fn execute(&mut self, inputs: Vec<Arc<Data>>) -> Option<Data>;
}

// Messages are sent from the ThreadPool to each worker.
// Each message can either be a new job, which includes an node and unique identifier,
// or the Terminate signal, which signals the Worker to stop listening for new jobs.
// TODO: For some types, cloning might be cheaper than using an Arc. Should do some
// kind of compile-time size check on Data.
enum Message<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    Job(Node, Vec<Arc<Data>>, usize),
    Terminate,
}

// WorkerStatus is used by each thread to report when it is finished.
// As part of this message, the worker also sends back the Node and the job ID,
// as well as the result of the Node (Data).
pub(crate) enum WorkerStatus<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    Complete(Node, Data, usize),
    // TODO: Return node on failure as well if we want to continue in the event of a failure.
    Fail(usize),
}

// Worker manages a single thread. It can receive Jobs via the associated mpsc::Receiver.
#[derive(Debug)]
struct Worker {
    #[allow(dead_code)]
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

// A receiver that listens for messages from the ThreadPool.
type JobReceiver<Node, Data> = Arc<Mutex<mpsc::Receiver<Message<Node, Data>>>>;
// A sender that sends WorkerStatus messages to the ThreadPool.
type WorkerSender<Node, Data> = mpsc::Sender<WorkerStatus<Node, Data>>;

impl Worker {
    // Creates a new worker with the given ID. Also sets up a receiver to listen for jobs.
    fn new<Node: 'static, Data: 'static>(id: usize, node_receiver: JobReceiver<Node, Data>, sender: WorkerSender<Node, Data>) -> Worker where Node: ThreadExecute<Data>, Data: Send + Sync {
        let thread = thread::spawn(move || {
            loop {
                // Listen for jobs. This blocks and is NOT a busy wait.
                let message = node_receiver.lock().unwrap().recv().unwrap();
                match message {
                    Message::Job(mut node, inputs, job_id) => {
                        let result = node.execute(inputs);
                        let wstatus;
                        match result {
                            Some(output) => wstatus = WorkerStatus::Complete(node, output, job_id),
                            None => wstatus = WorkerStatus::Fail(id),
                        }
                        match sender.send(wstatus) {
                            Ok(_) => (),
                            Err(what) => panic!("Worker {} could not send node {} status.\n{}", id, job_id, what),
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
#[derive(Debug)]
pub(crate) struct ThreadPool<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    workers: Vec<Worker>,
    node_sender: mpsc::Sender<Message<Node, Data>>,
    pub(crate) wstatus_receiver: mpsc::Receiver<WorkerStatus<Node, Data>>,
}

impl<Node: 'static, Data: 'static> ThreadPool<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    pub(crate) fn new(num_workers: usize) -> ThreadPool<Node, Data> {
        assert!(num_workers > 0);
        let (node_sender, node_receiver) = mpsc::channel();
        let node_receiver = Arc::new(Mutex::new(node_receiver));

        let (wstatus_sender, wstatus_receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(num_workers);
        for id in 0..num_workers {
            workers.push(Worker::new(id, Arc::clone(&node_receiver), wstatus_sender.clone()));
        }

        return ThreadPool{workers: workers, node_sender: node_sender, wstatus_receiver: wstatus_receiver};
    }

    // Executes the provided node with the provided inputs.
    pub(crate) fn execute(&self, node: Node, inputs: Vec<Arc<Data>>, id: usize) {
        self.node_sender.send(Message::Job(node, inputs, id)).unwrap();
    }
}

// Implements graceful shutdown and clean up for the ThreadPool.
impl<Node, Data> Drop for ThreadPool<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.node_sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
