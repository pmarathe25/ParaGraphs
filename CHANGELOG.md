# ParaGraphs - Parallel Graph Execution Library
**NOTE**: Dates are in YYYY-MM-DD format.

## v0.3.0 (2019-04-02)
- Changes `execute` to use `Vec<Arc<Data>>` because intermediate outputs were not being reference counted correctly.
- Recipe inputs/outputs are now `HashSet`s to prevent superfluous duplicates.
- `compile` now generates `node_inputs`, instead of `run`.
- Adds several docstrings.

## v0.2.2 (2019-03-25)
- Adds `get` and `get_mut` functions in `Graph` that will do one level of Option unwrapping.

## v0.2.1 (2019-03-20)
- Makes `Recipe` outputs public.
- Adds `IntoIterator` for `Graph`, so nodes are now easily iterable.

## v0.2.0 (2019-03-17)
- The execute function now returns an `Option`, so that the graph is aware when a node has failed.

## v0.1.1 (2019-03-16)
- lib.rs now exports `Graph` and `ThreadExecute`.

## v0.1.0 (2019-03-16)
- Nodes now also receive a container of `Edge`s as an input when they execute. This means that `Node`s can depend on the outputs of previous nodes.
- Renames `Edge` to `Data` for clarity.
- Renames `Execute` to `ThreadExecute`. Additionally, `ThreadExecute` implies `Send`.
- Inputs to nodes are now `Vec<Arc<Data>>` so that `Data` does not need to be copied for multiple outputs.
- Adds the concept of Recipes, which specify all the nodes that need to be run to get the desired output.
- Adds compile, which builds a `Recipe`, given a Vec of node indices to fetch from the graph.
- Adds run which executes the `Graph` using the `ThreadPool`.
- Adds handling for cases where an input is specified more than once for a node.
- Run function now returns the results of the output nodes in a `HashMap`.

## v0.0.1 (2019-03-11)
- Makes `ThreadPool` generic over an `Edge` type. This can be used for inter-node communication. For now, the whole graph must use the same `Edge` type.
- Makes `ThreadPool` generic over `Node` types as well. Nodes do not need to be stateful (though they can be), as the execute function now returns an `Edge`.
