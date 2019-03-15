use crate::threadpool::ThreadExecute;
use std::sync::Arc;
use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::Graph;
    use crate::threadpool::tests::{Adder};

    #[test]
    fn can_construct_graph() {
        let _graph: Graph<Adder, i32> = Graph::new();
    }

    #[test]
    fn can_add_node() {
        let mut graph: Graph<Adder, i32> = Graph::new();
        let node_id = graph.add(Adder::new(), vec![]);
        assert_eq!(node_id, 0);
    }

    #[test]
    #[should_panic(expected="does not exist in the graph")]
    fn cannot_set_node_input_to_itself() {
        let mut graph: Graph<Adder, i32> = Graph::new();
        graph.add(Adder::new(), vec![0]);
    }
}

// A simple struct for specifying what nodes need to be run, and which of them are inputs.
pub struct Recipe {
    runs: Vec<usize>,
    inputs: Vec<usize>
}

impl Recipe {
    fn new() -> Recipe {
        return Recipe{runs: Vec::new(), inputs: Vec::new()};
    }
}

pub struct Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    // This needs to be an option so that we can take() from it.
    nodes: Vec<Option<Node>>,
    // TODO: If input order matters, this needs to be a Vec<Vec<usize>>.
    node_inputs: Vec<HashSet<usize>>,
    node_outputs: Vec<HashSet<usize>>,
    phantom: std::marker::PhantomData<Data>,
}

impl<Node, Data> Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    pub fn new() -> Graph<Node, Data> {
        return Graph{nodes: Vec::new(), node_inputs: Vec::new(), node_outputs: Vec::new(), phantom: std::marker::PhantomData};
    }

    // Ensure that the graph is acyclic at an API level by not allowing inputs to
    // be set after the node is first added. Additionally, all inputs must already be in
    // the graph, or this function panics.
    // Returns the index of the newly added node.
    pub fn add(&mut self, node: Node, inputs: impl IntoIterator<Item=usize>) -> usize {
        let node_id = self.nodes.len();
        let mut input_set = HashSet::new();
        for input in inputs {
            // Add this node as an output of each of its inputs.
            match self.node_outputs.get_mut(input) {
                Some(outputs) => {
                    outputs.insert(input);
                },
                None => panic!("Node {}, specified as an input to {} does not exist in the graph", input, node_id)
            }
            // Add input to this node.
            input_set.insert(input);
        }
        // Push at the end, so that the above will fail if this node is an input to itself.
        self.nodes.push(Some(node));
        self.node_inputs.push(input_set);
        self.node_outputs.push(HashSet::new());
        return node_id;
    }

    // TODO: Need to have a separate function that takes fetches and builds some kind of
    // Recipe structure that the user can query for inputs.
    pub fn compile(&self, fetches: Vec<usize>) -> Recipe {
        return Recipe::new();
    }
}
