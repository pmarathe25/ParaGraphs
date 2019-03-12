# ParaGraphs - Parallel Graph Execution Library


# Understanding ParaGraphs

## The ThreadPool
The `ThreadPool` manages one or more `Worker`s. Each `Worker` is responsible for managing a single thread.
`Node`s, the unit of execution, are *moved* into Worker threads over an `mpsc::Channel`,
and returned when execution completes. It is the responsibility of the `Graph` to move the `Node`s back. 

The `ThreadPool` receives `Job`s, which are comprised of a `Node`, inputs (`Vec<Data>`), and a
unique identifier (`usize`). The `ThreadPool` then dispatches this job to one of its workers by
sending it over an `mpsc::Sender`. The contents of the `Job` are moved, so it is important that
`Node` and `Data` are both `Send`.

Each `Worker` listens for `Job`s on the corresponding `mpsc::Receiver`. Upon receiving a new `Job`,
it immediately begins execution. Upon completion, the `Worker` sends a message over its own `mpsc::Sender`
with the `Node`, its output (`Data`) as well the unique identifier (`usize`) for the `Job`.
