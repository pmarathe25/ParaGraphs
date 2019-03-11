pub mod threadpool;
// use crate::threadpool::ThreadPool;

// First create a Node trait.
// Create a graph structure that tracks a Vec<Option<impl Node>>, along with typical graph things like I/O.
// Next, move nodes out of the graph when dispatching to the threadpool. Threads in the threadpool can return the (potentially modified) nodes when done. These can then be moved back into the grpah.
// Lastly, need a way to transfer values through node edges. Or perhaps nodes should be stateful and hold their computation result. Then the next node can recieve immutable references to its inputs before executing.
