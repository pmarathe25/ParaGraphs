# ParaGraph - Parallel Graph Execution Library

## v0.1.0 (10/03/2019)
- Adds Executable type, which can be dispatched to the ThreadPool.
- Makes ThreadPool generic over an Edge type. This can be used for inter-node communication. For now, the whole graph must use the same Edge type.