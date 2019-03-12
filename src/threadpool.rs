use std::thread;
use std::sync::{mpsc, Arc, Mutex};

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::{ThreadPool, ThreadExecute, WorkerStatus};

    // Adds two numbers together.
    #[derive(Debug, Clone)]
    struct Adder {}

    impl Adder {
        fn new() -> Adder {
            return Adder{};
        }
    }

    impl ThreadExecute<i32> for Adder {
        fn execute(&mut self, inputs: Vec<&i32>) -> i32 {
            let mut sum = 0;
            for inp in inputs {
                sum += *inp;
            }
            return sum;
        }
    }

    #[test]
    fn can_construct_threadpool() {
        let _pool = ThreadPool::<Adder, i32>::new(8);
    }

    #[test]
    fn can_launch_jobs() {
        // Launch some arbitrary job, like waiting.
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

// Any job to be executed by this ThreadPool must be ThreadExecute.
pub trait ThreadExecute<Data> : Send where Data: Send + Sync {
    fn execute(&mut self, inputs: Vec<&Data>) -> Data;
}

// Any type which is ThreadExecute and Send can be dispatched via the ThreadPool.
// Messages are sent from the ThreadPool to each worker.
// Each message can either be a new job, which includes an node and unique identifier,
// or the Terminate signal, which signals the Worker to stop listening for new jobs.
// TODO: For some types, cloning might be cheaper than using an Arc. Need to do some
// kind of compile-time size check on Data.
enum Message<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    Job(Node, Vec<Arc<Data>>, usize),
    Terminate,
}

// WorkerStatus is used by each thread to report when it is finished.
// As part of this message, the worker also sends back the Node and the job ID,
// as well as the result of the Node (Data).
// The node may or may not be stateful.
pub enum WorkerStatus<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    Complete(Node, Data, usize),
}

// Worker manages a single thread. It can receive jobs via the associated mpsc::Receiver.
// Each job should be accompanied by an id so it can be identified.
struct Worker {
    #[allow(dead_code)]
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    // Creates a new worker with the given ID. Also sets up a receiver to listen for jobs.
    fn new<Node: 'static, Data: 'static>(id: usize,
        node_receiver: Arc<Mutex<mpsc::Receiver<Message<Node, Data>>>>,
        sender: mpsc::Sender<WorkerStatus<Node, Data>>) -> Worker
        where Node: ThreadExecute<Data>, Data: Send + Sync {
        let thread = thread::spawn(move || {
            loop {
                // Listen for jobs. This blocks and is NOT a busy wait.
                let message = node_receiver.lock().unwrap().recv().unwrap();
                match message {
                    // New jobs are executed, and a status message is returned which packages the
                    // included Executable and its id.
                    Message::Job(mut node, inputs, job_id) => {
                        // Consume the vector of Arcs, and create a vector of references,
                        // for ease-of-use in the public API. This should be fairly
                        // light-weight, as it just dereferences the Arcs.
                        // TODO: If this turns out not to be light-weight, need to use
                        // Arcs in the public API.
                        let deref_inputs;
                        unsafe {
                            deref_inputs = inputs.into_iter().map(
                                |inp| &*Arc::into_raw(inp)).collect();
                        }
                        let result = node.execute(deref_inputs);
                        match sender.send(WorkerStatus::Complete(node, result, job_id)) {
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
pub struct ThreadPool<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    workers: Vec<Worker>,
    node_sender: mpsc::Sender<Message<Node, Data>>,
    pub wstatus_receiver: mpsc::Receiver<WorkerStatus<Node, Data>>,
}

impl<Node: 'static, Data: 'static> ThreadPool<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    pub fn new(num_workers: usize) -> ThreadPool<Node, Data> {
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

    /// Executes the provided node with the provided inputs.
    pub fn execute(&self, node: Node, inputs: Vec<Arc<Data>>, id: usize) {
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
