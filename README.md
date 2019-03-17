# ParaGraphs - Parallel Graph Execution Library


# Understanding ParaGraphs

## The ThreadPool
The `ThreadPool` manages one or more `Worker`s. Each `Worker` is responsible for managing a
single thread. `Node`s, the units of execution, are *moved* into Worker threads over an
`mpsc::Channel`, and returned over a second channel when execution completes. It is the
responsibility of the `Graph` to move the `Node`s back out of the channel.

The `ThreadPool` receives `Job`s, which are comprised of a `Node`, inputs (`Vec<Arc<Data>>`), and a
unique identifier (`usize`). The `ThreadPool` then dispatches each job to one of its workers by
sending it via an `mpsc::Sender`. The contents of the `Job` are moved, so it is important that
`Node` is `Send` and `Data` is `Send + Sync`. The inputs to the `Node` are captured in a vector of
atomically reference counted pointers (`Arc`s), so that multiple output nodes can access the same
values without the need for `Clone`-ing and `Send`ing.

Each `Worker` listens for `Job`s on the corresponding `mpsc::Receiver`. Upon receiving a new `Job`,
it immediately begins execution. Upon completion, the `Worker` sends a message over its own `mpsc::Sender`
with the `Node`, its output (`Data`) as well the unique identifier (`usize`) for the `Job`.

## Graph
