use crate::threadpool::ThreadExecute;
use std::sync::Arc;
use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;
    use std::collections::HashSet;
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

    #[test]
    fn can_compile_graph() {
        let mut graph = Graph::new();
        let input = graph.add(Adder::new(), vec![]);
        // Diamond graph.
        let hidden1 = graph.add(Adder::new(), vec![input]);
        let hidden2 = graph.add(Adder::new(), vec![input]);
        let _deadend = graph.add(Adder::new(), vec![hidden1]);
        let output1 = graph.add(Adder::new(), vec![hidden1, hidden2]);
        let output2 = graph.add(Adder::new(), vec![hidden1, hidden2]);
        println!("Graph: {:?}", graph);
        // Get recipe for the first output.
        let out1_recipe = graph.compile(vec![output1]);
        println!("Output 1 Recipe: {:?}", out1_recipe);
        // Check
        assert_eq!(out1_recipe.runs, HashSet::from_iter(vec![input, hidden1, hidden2, output1]));
        assert_eq!(out1_recipe.inputs, vec![input]);
        // Get recipe for the second output.
        let out2_recipe = graph.compile(vec![output2]);
        println!("Output 2 Recipe: {:?}", out2_recipe);
        // Check correctness.
        assert_eq!(out2_recipe.runs, HashSet::from_iter(vec![input, hidden1, hidden2, output2]));
        assert_eq!(out2_recipe.inputs, vec![input]);
    }
}

// A simple struct for specifying what nodes need to be run, and which of them are
// graph inputs/outputs. This struct also allocates space for storing intermediate outputs.
#[derive(Debug)]
pub struct Recipe<Data> {
    runs: HashSet<usize>,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
    intermediates: HashMap<usize, Arc<Data>>
}

impl<Data> Recipe<Data> {
    fn new(runs: HashSet<usize>, inputs: Vec<usize>, outputs: Vec<usize>) -> Recipe<Data> {
        let num_nodes = runs.len();
        return Recipe{runs: runs, inputs: inputs, outputs: outputs, intermediates: HashMap::with_capacity(num_nodes)};
    }
}

#[derive(Debug)]
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

    pub fn compile(&self, mut fetches: Vec<usize>) -> Recipe<Data> {
        let mut index = 0;
        let mut recipe_inputs = Vec::new();
        // Remove unecessary duplicates, and then store as recipe_outputs.
        fetches.dedup();
        let recipe_outputs = fetches.clone();
        // Walk over fetches, and append the inputs of each node in it to the end of the vector.
        // This is a BFS for finding all nodes that need to be executed.
        while index < fetches.len() {
            if let Some(current_node) = fetches.get(index) {
                if let Some(inputs) = self.node_inputs.get(*current_node) {
                    // Nodes with no inputs ARE inputs.
                    if inputs.len() == 0 {
                        recipe_inputs.push(*current_node);
                    }
                    fetches.extend(inputs);
                };
            };
            index += 1;
        }
        recipe_inputs.dedup();
        return Recipe::new(HashSet::from_iter(fetches), recipe_inputs, recipe_outputs);
    }
}
