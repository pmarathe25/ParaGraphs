use crate::threadpool::{ThreadExecute, ThreadPool, WorkerStatus};
use std::collections::{HashSet, HashMap};
use std::iter::{FromIterator, Iterator};
use std::sync::Arc;
// DEBUG: TODO: Remove
use std::fmt::Debug;

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;
    use std::collections::{HashSet, HashMap};
    use super::Graph;
    use crate::threadpool::tests::{Adder};

    const NUM_THREADS: usize = 8;

    #[test]
    fn can_construct_graph() {
        let _graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
    }

    #[test]
    fn can_add_node() {
        let mut graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
        let node_id = graph.add(Adder::new(), vec![]);
        assert_eq!(node_id, 0);
    }

    #[test]
    #[should_panic(expected="does not exist in the graph")]
    fn cannot_set_node_input_to_itself() {
        let mut graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
        graph.add(Adder::new(), vec![0]);
    }

    fn build_diamond_graph() -> (Graph<Adder, i32> ,usize, usize, usize, usize, usize) {
        let mut graph = Graph::new(NUM_THREADS);
        let input = graph.add(Adder::new(), vec![]);
        // Diamond graph.
        let hidden1 = graph.add(Adder::new(), vec![input, input]);
        let hidden2 = graph.add(Adder::new(), vec![input, input]);
        let output1 = graph.add(Adder::new(), vec![hidden1, hidden2]);
        let output2 = graph.add(Adder::new(), vec![hidden1, hidden2]);
        let _deadend = graph.add(Adder::new(), vec![hidden1]);
        return (graph, input, hidden1, hidden2, output1, output2);
    }

    #[test]
    fn can_compile_graph() {
        let (graph, input, hidden1, hidden2, output1, output2) = build_diamond_graph();
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

    #[test]
    fn can_run_graph() {
        let (mut graph, input, _hidden1, _hidden2, output1, output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        let recipe = graph.compile(vec![output1, output2]);
        println!("Recipe: {:?}", recipe);
        let inputs_map = HashMap::from_iter(vec!(
            (input, vec![1, 2, 3])
        ));
        graph.run(&recipe, inputs_map);
    }
}

// A simple struct for specifying what nodes need to be run, and which of them are
// graph inputs/outputs. This struct also allocates space for storing intermediate outputs.
#[derive(Debug)]
pub struct Recipe {
    runs: HashSet<usize>,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
    // Maps every node in runs to any outputs in the Recipe.
    node_outputs: HashMap<usize, HashSet<usize>>,
}

impl Recipe {
    fn new(runs: HashSet<usize>, inputs: Vec<usize>, outputs: Vec<usize>, node_outputs: HashMap<usize, HashSet<usize>>) -> Recipe {
        return Recipe{runs: runs, inputs: inputs, outputs: outputs, node_outputs: node_outputs};
    }
}

#[derive(Debug)]
pub struct Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    // This needs to be an option so that we can take() from it.
    nodes: Vec<Option<Node>>,
    node_inputs: Vec<Vec<usize>>,
    pool: ThreadPool<Node, Data>,
}


// DEBUG: TODO: Remove + Debug bound.
impl<Node: 'static, Data: 'static> Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync + Debug {
    pub fn new(num_threads: usize) -> Graph<Node, Data> {
        return Graph{nodes: Vec::new(), node_inputs: Vec::new(), pool: ThreadPool::new(num_threads)};
    }

    // Ensure that the graph is acyclic at an API level by not allowing inputs to
    // be set after the node is first added. Additionally, all inputs must already be in
    // the graph, or this function panics.
    // Returns the index of the newly added node.
    pub fn add(&mut self, node: Node, inputs: impl IntoIterator<Item=usize>) -> usize {
        let node_id = self.nodes.len();
        // Push at the end, so that the above will fail if this node is an input to itself.
        self.nodes.push(Some(node));
        let inputs = inputs.into_iter().collect();
        for &input in &inputs {
            if input >= node_id {
                panic!("Cannot add {} as an input to {} as it does not exist in the graph.", input, node_id);
            }
        }
        self.node_inputs.push(inputs);
        return node_id;
    }

    pub fn compile(&self, mut fetches: Vec<usize>) -> Recipe {
        let mut index = 0;
        let mut recipe_inputs = Vec::new();
        let mut node_outputs: HashMap<usize, HashSet<usize>> = HashMap::new();
        // Remove unecessary duplicates, and then store as recipe_outputs.
        fetches.dedup();
        let recipe_outputs = fetches.clone();
        // Walk over fetches, and append the inputs of each node in it to the end of the vector.
        // This is a BFS for finding all nodes that need to be executed.
        while index < fetches.len() {
            if let Some(node_id) = fetches.get(index) {
                if let Some(inputs) = self.node_inputs.get(*node_id) {
                    // Nodes with no inputs ARE inputs.
                    if inputs.len() == 0 {
                        recipe_inputs.push(*node_id);
                    }
                    for input in inputs {
                        match node_outputs.get_mut(input) {
                            // If this node is already in the map, then append the output to it.
                            Some(outputs) => { outputs.insert(*node_id); },
                            // Otherwise insert it into the map.
                            None => {
                                node_outputs.insert(*input, HashSet::from_iter(vec![*node_id]));
                            },
                        };
                    }
                    fetches.extend(inputs);
                };
            };
            index += 1;
        }
        recipe_inputs.dedup();
        return Recipe::new(HashSet::from_iter(fetches), recipe_inputs, recipe_outputs, node_outputs);
    }

    // Runs the provided Recipe with the provded inputs (map of {node: inputs}).
    // If inputs are missing, panics.
    // DEBUG: TODO: Remove + Debug here
    pub fn run(&mut self, recipe: &Recipe, mut inputs_map: HashMap<usize, Vec<Data>>) where Data: Debug {
        // DEBUG: TODO: Remove + Debug here
        fn execute_node<Node: 'static, Data: 'static>(graph: &mut Graph<Node, Data>, node_id: usize, inputs: Vec<Arc<Data>>) where Node: ThreadExecute<Data>, Data: Send + Sync + Debug {
            if let Some(node_option) = graph.nodes.get_mut(node_id) {
                if let Some(node) = node_option.take() {
                    // DEBUG:
                    println!("Executing node {} with inputs {:?}", node_id, inputs);
                    graph.pool.execute(node, inputs, node_id);
                }
            };
        }

        // Each time we execute, we decrement the nodes remaining count.
        let mut num_nodes_remaining = recipe.runs.len();
        // We also store intermediate outputs of nodes.
        let mut intermediates: HashMap<usize, Arc<Data>> = HashMap::with_capacity(num_nodes_remaining);
        // Maps each node in recipe.runs to its inputs. When there are no inputs remaining,
        // it means the node can be executed.
        let mut node_inputs_map: HashMap<usize, HashSet<usize>> = recipe.runs.iter().map(
            |index| {
                match self.node_inputs.get(*index) {
                    Some(inputs) => (index.clone(), inputs.iter().cloned().collect()),
                    None => panic!("Node {} is specified in recipe, but does not exist in the graph", index),
                }
            }
        ).collect();

        // First, launch all input nodes.
        for input_node in &recipe.inputs {
            if let Some(inputs) = inputs_map.remove(input_node) {
                let arc_inputs: Vec<Arc<Data>> = inputs.into_iter().map(|input| Arc::new(input)).collect();
                // Queue up every input node.
                execute_node(self, *input_node, arc_inputs);
            };
        }

        // Keep going until everything has finished executing.
        while num_nodes_remaining > 0 {
            // Currently there is only one valid WorkerStatus.
            if let Ok(WorkerStatus::Complete(node, result, node_id)) = self.pool.wstatus_receiver.recv() {
                num_nodes_remaining -= 1;
                // Place the node back into the graph.
                match self.nodes.get_mut(node_id) {
                    Some(node_option) => node_option.replace(node),
                    None => panic!("Received WorkerStatus for node {}, but this node is not in the graph", node_id),
                };

                // Store the intermediate output.
                // DEBUG:
                println!("Node {} returned output {:?}", node_id, result);
                match intermediates.insert(node_id, Arc::new(result)) {
                    Some(_) => panic!("Node {} was executed more than once, possibly due to a cycle", node_id),
                    None => (),
                };

                // See if it is possible to queue other nodes.
                // If a node is not found in the output map, then it means it IS an output.
                if let Some(output_ids) = recipe.node_outputs.get(&node_id) {
                    // Next, walk over all the output_ids of this node, decrementing
                    // their input counts.
                    // It is ok for nodes to be missing in the recipe.
                    for output_id in output_ids {
                        if let Some(remaining_inputs) = node_inputs_map.get_mut(output_id) {
                            remaining_inputs.remove(&node_id);
                            // If any hit 0, execute them.
                            if remaining_inputs.len() == 0 {
                                // Assemble the required inputs.
                                let mut inputs: Vec<Arc<Data>> = Vec::new();
                                match self.node_inputs.get(*output_id) {
                                    Some(input_ids) => {
                                        for input_id in input_ids {
                                            match intermediates.get(input_id) {
                                                Some(intermediate) => inputs.push(Arc::clone(intermediate)),
                                                None => panic!("Node {} attempted to execute, but input {} is missing", output_id, input_id)
                                            }
                                        }
                                    },
                                    None => panic!("Could not find node {}'s output {} in the graph", node_id, output_id)
                                };
                                execute_node(self, *output_id, inputs);
                            }
                        }
                    } // for output_id in output_ids
                }
            }
        }
    } // fn run
} // impl
