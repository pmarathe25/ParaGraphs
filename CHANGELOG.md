# ParaGraphs - Parallel Graph Execution Library
**NOTE**: Dates are in dd-mm-yyy format.

## vNext ()
- Nodes now also receive a container of Edges as an input when they execute. This means that Nodes can depend on the outputs of previous nodes.
- Renames Edge to Data for clarity.
- Renames Execute to ThreadExecute. Additionally, ThreadExecute implies Send.
- Inputs to nodes are now Vec<Arc<Data>> so that Data does not need to be copied for multiple outputs.
- Adds the concept of Recipes, which specify all the nodes that need to be run to get the desired output.
- Adds compile, which builds a Recipe, given a Vec of node indices to fetch from the graph.
- Adds run which executes the Graph using the ThreadPool.
- Adds handling for cases where an input is specified more than once for a node.

## v0.0.1 (11-03-2019)
- Makes ThreadPool generic over an Edge type. This can be used for inter-node communication. For now, the whole graph must use the same Edge type.
- Makes ThreadPool generic over Node types as well. Nodes do not need to be stateful (though they can be), as the execute function now returns an Edge.
