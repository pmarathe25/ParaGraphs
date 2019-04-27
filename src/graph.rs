use std::borrow::Borrow;
use crate::threadpool::{ThreadExecute, ThreadPool, WorkerStatus};
use std::collections::{HashSet, HashMap};
use std::iter::{FromIterator};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;
    use std::collections::{HashSet, HashMap};
    use super::Graph;
    use crate::threadpool::ThreadExecute;
    use crate::threadpool::tests::{Adder};
    use std::sync::Arc;

    const NUM_THREADS: usize = 8;

    #[test]
    fn can_construct_graph() {
        let _graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
    }

    #[test]
    fn can_add_node() {
        let mut graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
        let node_id = graph.add(Adder::new(), &[]);
        assert_eq!(node_id, 0);
    }

    #[test]
    #[should_panic(expected="does not yet exist in the graph")]
    fn cannot_set_node_input_to_itself() {
        let mut graph: Graph<Adder, i32> = Graph::new(NUM_THREADS);
        graph.add(Adder::new(), vec![0]);
    }

    fn build_diamond_graph() -> (Graph<Adder, i32> ,usize, usize, usize, usize, usize) {
        let mut graph = Graph::new(NUM_THREADS);
        let input = graph.add(Adder::new(), &[]);
        // Diamond graph.
        let hidden1 = graph.add(Adder::new(), &[input, input]);
        let hidden2 = graph.add(Adder::new(), &[input, input]);
        let output1 = graph.add(Adder::new(), &[hidden1, hidden2]);
        let output2 = graph.add(Adder::new(), &[hidden1, hidden2]);
        let _deadend = graph.add(Adder::new(), &[hidden1]);
        return (graph, input, hidden1, hidden2, output1, output2);
    }

    #[test]
    fn can_compile_graph() {
        let (graph, input, hidden1, hidden2, output1, output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        // Get recipe for the first output.
        let out1_recipe = graph.compile(&[output1]);
        println!("Output 1 Recipe: {:?}", out1_recipe);
        // Check
        assert_eq!(out1_recipe.runs, HashSet::from_iter(vec![input, hidden1, hidden2, output1]));
        assert_eq!(out1_recipe.inputs, HashSet::from_iter([input].iter().cloned()));
        // Get recipe for the second output.
        let out2_recipe = graph.compile(vec![output2]);
        println!("Output 2 Recipe: {:?}", out2_recipe);
        // Check correctness.
        assert_eq!(out2_recipe.runs, HashSet::from_iter(vec![input, hidden1, hidden2, output2]));
        assert_eq!(out2_recipe.inputs, HashSet::from_iter([input].iter().cloned()));
    }

    #[test]
    fn can_run_graph() {
        let (mut graph, input, _hidden1, _hidden2, output1, output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        let recipe = graph.compile(vec![output1, output2, output2]);
        println!("Recipe: {:?}", recipe);
        let inputs_map = HashMap::from_iter(vec!(
            (input, vec![1, 2, 3])
        ));
        let outputs = graph.run(&recipe, inputs_map);
        println!("Outputs: {:?}", outputs);
        assert_eq!(outputs.get(&output1), Some(&24));
        assert_eq!(outputs.get(&output2), Some(&24));
    }

    #[test]
    fn can_run_graph_all_nodes_outputs() {
        let (mut graph, input, _hidden1, _hidden2, output1, output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        let recipe = graph.compile(vec![input, _hidden1, _hidden2, output1, output2, output2]);
        println!("Recipe: {:?}", recipe);
        let inputs_map = HashMap::from_iter(vec!(
            (input, vec![1, 2, 3])
        ));
        println!("Input map: {:?}", inputs_map);
        let outputs = graph.run(&recipe, inputs_map);
        println!("Outputs: {:?}", outputs);
        assert_eq!(outputs.get(&output1), Some(&24));
        assert_eq!(outputs.get(&output2), Some(&24));
    }

    #[test]
    fn can_run_graph_input_is_output() {
        let (mut graph, input, _hidden1, _hidden2, _output1, _output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        let recipe = graph.compile(vec![input]);
        println!("Recipe: {:?}", recipe);
        let inputs_map = HashMap::from_iter(vec!(
            (input, vec![1, 2, 3])
        ));
        println!("Input map: {:?}", inputs_map);
        let outputs = graph.run(&recipe, inputs_map);
        println!("Outputs: {:?}", outputs);
        assert_eq!(outputs.get(&input), Some(&6));
    }

    #[test]
    fn can_run_graph_input_and_node() {
        let (mut graph, input, hidden1, _hidden2, _output1, _output2) = build_diamond_graph();
        println!("Graph: {:?}", graph);
        let recipe = graph.compile(vec![input, hidden1]);
        println!("Recipe: {:?}", recipe);
        let inputs_map = HashMap::from_iter(vec!(
            (input, vec![1, 2, 3])
        ));
        println!("Input map: {:?}", inputs_map);
        let outputs = graph.run(&recipe, inputs_map);
        println!("Outputs: {:?}", outputs);
        assert_eq!(outputs.get(&hidden1), Some(&12));
    }

    struct FailNode;

    impl ThreadExecute<i32> for FailNode {
        fn execute(&mut self, _inputs: Vec<Arc<i32>>) -> Option<i32> {
            return None;
        }
    }

    #[test]
    #[should_panic(expected="Graph failed to execute because node")]
    fn node_failure_causes_panic() {
        let mut graph = Graph::new(NUM_THREADS);
        let input = graph.add(FailNode{}, &[]);
        let recipe = graph.compile(&[input]);
        let mut inputs_map = HashMap::new();
        inputs_map.insert(input, vec!());
        let _ = graph.run(&recipe, inputs_map);
    }

    #[test]
    fn can_iterate_graph_nodes() {
        let graph = build_diamond_graph().0;
        let mut num_nodes = 0;
        let expected_num_nodes = graph.len();
        for node in graph {
            assert!(node.valid);
            num_nodes += 1;
        }
        assert_eq!(num_nodes, expected_num_nodes);
    }

    #[test]
    fn can_iterate_ref_graph_nodes() {
        let graph = build_diamond_graph().0;
        let mut num_nodes = 0;
        let expected_num_nodes = graph.len();
        for node in &graph {
            assert!(node.valid);
            num_nodes += 1;
        }
        assert_eq!(num_nodes, expected_num_nodes);
    }

    #[test]
    fn can_iterate_ref_mut_graph_nodes() {
        let mut graph = build_diamond_graph().0;
        let mut num_nodes = 0;
        let expected_num_nodes = graph.len();
        for node in &mut graph {
            assert!(node.valid);
            num_nodes += 1;
        }
        assert_eq!(num_nodes, expected_num_nodes);
    }
}

/// Describes a recipe for retrieving a particular set of outputs.
#[derive(Debug)]
pub struct Recipe {
    runs: HashSet<usize>,
    /// The graph indices of the inputs of this recipe.
    /// Values for each of these must be provided at graph execution time.
    pub inputs: HashSet<usize>,
    /// The graph indices of the outputs of this recipe.
    /// Values for each of these will be returned after graph execution.
    pub outputs: HashSet<usize>,
    // Maps every node in runs to any outputs in the Recipe.
    node_outputs: HashMap<usize, HashSet<usize>>,
    // Maps every node in runs to its inputs, which must be in the Recipe.
    node_inputs: HashMap<usize, HashSet<usize>>,
}

impl Recipe {
    fn new(runs: HashSet<usize>, inputs: HashSet<usize>, outputs: HashSet<usize>, node_outputs: HashMap<usize, HashSet<usize>>, node_inputs: HashMap<usize, HashSet<usize>>) -> Recipe {
        if inputs.len() == 0 {
            panic!("Invalid Recipe: Found 0 inputs. Recipes must have at least one input node.");
        }
        return Recipe{runs: runs, inputs: inputs, outputs: outputs, node_outputs: node_outputs, node_inputs: node_inputs};
    }
}

/// A computation graph.
/// Nodes are executed concurrently wherever possible.
/// Each graph manages its own threadpool, so although it may be possible to creat
/// higher-order graphs, it is generally not advisable.
#[derive(Debug)]
pub struct Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    // This needs to be an option so that we can take() from it.
    nodes: Vec<Option<Node>>,
    node_inputs: Vec<Vec<usize>>,
    pool: ThreadPool<Node, Data>,
}

impl<Node, Data> IntoIterator for Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    type Item = Node;
    type IntoIter = std::iter::Map<std::vec::IntoIter<std::option::Option<Node>>, fn(std::option::Option<Node>) -> Node>;


    fn into_iter(self) -> Self::IntoIter {
        fn expect_node<Node>(node: Option<Node>) -> Node {
            return node.expect("Node has been moved out of the graph. Is the graph being executed?");
        }
        return self.nodes.into_iter().map(expect_node);
    }
}

impl<'a, Node, Data> IntoIterator for &'a Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    type Item = &'a Node;
    // type IntoIter = GraphIterator<Node>;
    type IntoIter = std::iter::Map<std::slice::Iter<'a, std::option::Option<Node>>, fn(&std::option::Option<Node>) -> &Node>;

    fn into_iter(self) -> Self::IntoIter {
        fn expect_node<Node>(node: &Option<Node>) -> &Node {
            return match node {
                Some(n) => n,
                None => panic!("Node has been moved out of the graph. Is the graph being executed?"),
            };
        }
        return self.nodes.iter().map(expect_node);
    }
}

impl<'a, Node, Data> IntoIterator for &'a mut Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {
    type Item = &'a mut Node;
    // type IntoIter = GraphIterator<Node>;
    type IntoIter = std::iter::Map<std::slice::IterMut<'a, std::option::Option<Node>>, fn(&mut std::option::Option<Node>) -> &mut Node>;

    fn into_iter(self) -> Self::IntoIter {
        fn expect_node<Node>(node: &mut Option<Node>) -> &mut Node {
            return match node {
                Some(n) => n,
                None => panic!("Node has been moved out of the graph. Is the graph being executed?"),
            };
        }
        return self.nodes.iter_mut().map(expect_node);
    }
}

impl<Node: 'static, Data: 'static> Graph<Node, Data> where Node: ThreadExecute<Data>, Data: Send + Sync {

    /// Creates a new graph.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - The number of threads available to the graph.
    ///
    /// # Example
    ///
    /// ```
    /// use paragraphs::{Graph, ThreadExecute};
    /// use std::sync::Arc;
    ///
    /// struct MyNode;
    /// struct MyData;
    ///
    /// impl ThreadExecute<MyData> for MyNode {
    ///     fn execute(&mut self, inputs: Vec<Arc<MyData>>) -> Option<MyData> {
    ///         return Some(MyData{});
    ///     }
    /// }
    ///
    /// let graph: Graph<MyNode, MyData> = Graph::new(8);
    /// ```
    pub fn new(num_threads: usize) -> Graph<Node, Data> {
        return Graph{nodes: Vec::new(), node_inputs: Vec::new(), pool: ThreadPool::new(num_threads)};
    }

    /// Returns the number of nodes in the graph.
    pub fn len(&self) -> usize {
        return self.nodes.len();
    }

    /// Gets an Option containing a reference to the node at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the node.
    pub fn get(&self, index: usize) -> Option<&Node> {
        if let Some(node) = self.nodes.get(index) {
            return node.as_ref();
        }
        return None;
    }

    /// Gets an Option containing a mutable referenece to the node at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the node.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Node> {
        if let Some(node) = self.nodes.get_mut(index) {
            return node.as_mut();
        }
        return None;
    }

    /// Adds a new node to the graph.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to add to the graph.
    /// * `inputs` - The indices of the inputs to the node. All inputs specified must
    ///             already be present in the graph, or the function panics.
    ///             Inputs may be specified more than once.
    ///
    /// # Example
    ///
    /// ```
    /// use paragraphs::Graph;
    /// # use paragraphs::ThreadExecute;
    /// # use std::sync::Arc;
    /// # struct MyNode;
    /// # struct MyData;
    /// # impl ThreadExecute<MyData> for MyNode {
    /// #     fn execute(&mut self, inputs: Vec<Arc<MyData>>) -> Option<MyData> {
    /// #         return Some(MyData{});
    /// #     }
    /// # }
    /// let mut graph = Graph::new(8);
    /// let node0 = graph.add(MyNode{}, &[]);
    /// let node1 = graph.add(MyNode{}, &[node0, node0]);
    /// ```
    pub fn add<Container, Elem>(&mut self, node: Node, inputs: Container) -> usize where Container: IntoIterator<Item=Elem>, Elem: Borrow<usize> {
        let node_id = self.nodes.len();
        self.nodes.push(Some(node));
        // Push inputs at the end, so that the above will fail if this node is an input to itself.
        let inputs = inputs.into_iter().map(|x| x.borrow().clone()).collect();
        for &input in &inputs {
            if input >= node_id {
                panic!("Cannot add node {} as an input to node {} as it does not yet exist in the graph.", input, node_id);
            }
        }
        self.node_inputs.push(inputs);
        return node_id;
    }

    /// Generates a Recipe that can be used to compute the specified outputs.
    ///
    /// # Arguments
    ///
    /// * `fetches` - The indices of the nodes to fetch. May contain duplicates.
    ///             Panics if no nodes are specified.
    ///
    /// # Example
    ///
    /// ```
    /// # use paragraphs::{Graph, ThreadExecute};
    /// # use std::sync::Arc;
    /// # struct MyNode;
    /// # struct MyData;
    /// # impl ThreadExecute<MyData> for MyNode {
    /// #     fn execute(&mut self, inputs: Vec<Arc<MyData>>) -> Option<MyData> {
    /// #         return Some(MyData{});
    /// #     }
    /// # }
    /// # let mut graph = Graph::new(8);
    /// # let node0 = graph.add(MyNode{}, &[]);
    /// # let node1 = graph.add(MyNode{}, &[node0, node0]);
    /// // This recipe can be used to fetch the result of node1.
    /// let node1_recipe = graph.compile(&[node1]);
    /// ```
    pub fn compile<Container, Elem>(&self, fetches: Container) -> Recipe
        where Container: IntoIterator<Item=Elem>, Elem: Borrow<usize> {
        let mut index = 0;
        let mut recipe_inputs = HashSet::new();
        let mut node_outputs: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut node_inputs: HashMap<usize, HashSet<usize>> = HashMap::new();
        // Remove unecessary duplicates, and then store as recipe_outputs.
        let mut fetches: Vec<usize> = fetches.into_iter().map(|x| x.borrow().clone()).collect();
        let recipe_outputs = HashSet::from_iter(fetches.iter().cloned());
        // Walk over fetches, and append the inputs of each node in it to the end of the vector.
        // This is a BFS for finding all nodes that need to be executed.
        while index < fetches.len() {
            let node_id = match fetches.get(index) {
                Some(id) => id,
                None => panic!("Could not get index {} index in fetches ({:?}) during BFS", index, fetches),
            };
            let inputs = match self.node_inputs.get(*node_id) {
                Some(id) => id,
                None => panic!("Could not get node inputs for node {}", node_id),
            };
            // Nodes with no inputs ARE inputs.
            if inputs.len() == 0 {
                recipe_inputs.insert(*node_id);
            }
            // Add node inputs.
            node_inputs.insert(*node_id, inputs.iter().cloned().collect());
            // Add node outputs.
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
            index += 1;
        }
        return Recipe::new(HashSet::from_iter(fetches), recipe_inputs, recipe_outputs, node_outputs, node_inputs);
    }

    // Runs the provided Recipe with the provded inputs (map of {node: inputs}).
    // If inputs are missing, panics.
    // TODO: Document all panic conditions.
    /// Executes the nodes specified by the recipe using the provided inputs.
    ///
    /// # Arguments
    ///
    /// * `recipe` - The recipe to execute. All outputs of the recipe are fetched.
    /// * `inputs_map` - Inputs for each input node. Panics if inputs are missing.
    /// # Example
    ///
    /// ```
    /// # use paragraphs::{Graph, ThreadExecute};
    /// # use std::sync::Arc;
    /// use std::collections::HashMap;
    /// use std::iter::FromIterator;
    /// # struct MyNode;
    /// # struct MyData;
    /// # impl ThreadExecute<MyData> for MyNode {
    /// #     fn execute(&mut self, inputs: Vec<Arc<MyData>>) -> Option<MyData> {
    /// #         return Some(MyData{});
    /// #     }
    /// # }
    /// # let mut graph = Graph::new(8);
    /// # let node0 = graph.add(MyNode{}, &[]);
    /// # let node1 = graph.add(MyNode{}, &[node0, node0]);
    /// # let node1_recipe = graph.compile(&[node1]);
    /// let inputs_map = HashMap::from_iter(vec!(
    ///     (node0, vec![MyData{}]),
    /// ));
    /// let outputs = graph.run(&node1_recipe, inputs_map);
    /// ```
    pub fn run(&mut self, recipe: &Recipe, mut inputs_map: HashMap<usize, Vec<Data>>) -> HashMap<usize, Data> {
        fn execute_node<Node: 'static, Data: 'static>(graph: &mut Graph<Node, Data>, node_id: usize, inputs: Vec<Arc<Data>>) where Node: ThreadExecute<Data>, Data: Send + Sync {
            let node_opt = match graph.nodes.get_mut(node_id) {
                Some(id) => id,
                None => panic!("While attempting to execute, could not retrieve node {}", node_id)
            };
            let node = match node_opt.take() {
                Some(node) => node,
                None => panic!("Could not retrieve node {} - is it currently being executed?", node_id)
            };
            graph.pool.execute(node, inputs, node_id);
        }

        fn assemble_inputs<Node, Data>(graph: &Graph<Node, Data>, intermediates: &HashMap<usize, Arc<Data>>, node_id: usize) -> Vec<Arc<Data>> where Node: ThreadExecute<Data>, Data: Send + Sync {
            let mut inputs: Vec<Arc<Data>> = Vec::new();
            let input_ids = match graph.node_inputs.get(node_id) {
                Some(id) => id,
                None => panic!("Could not find node {} in the graph", node_id)
            };
            for input_id in input_ids {
                let intermediate = match intermediates.get(input_id) {
                    Some(intermediate) => intermediate,
                    None => panic!("Node {} attempted to execute, but input {} is missing", node_id, input_id)
                };
                inputs.push(Arc::clone(intermediate));
            }
            return inputs;
        }

        // Each time we receive a node back, we will decrement the nodes remaining count.
        let mut num_nodes_remaining = recipe.runs.len();
        // We also store intermediate outputs of nodes.
        let mut intermediates: HashMap<usize, Arc<Data>> = HashMap::with_capacity(num_nodes_remaining);
        // Maps each node in recipe.runs to its inputs. When there are no inputs remaining,
        // it means the node can be executed.
        let mut remaining_inputs_map = recipe.node_inputs.clone();

        // First, launch all input nodes.
        for input_node in &recipe.inputs {
            match inputs_map.remove(input_node) {
                Some(inputs) => {
                    let arc_inputs: Vec<Arc<Data>> = inputs.into_iter().map(|input| Arc::new(input)).collect();
                    // Queue up every input node.
                    execute_node(self, *input_node, arc_inputs);
                },
                None => panic!("Input for {} is missing", input_node),
            };
        }

        // Keep going until everything has finished executing.
        while num_nodes_remaining > 0 {
            if let Ok(wstatus) = self.pool.wstatus_receiver.recv() {
                match wstatus {
                    WorkerStatus::Complete(node, result, node_id) => {
                        // Each time we receive a node back, we decrement the count of nodes
                        // that haven't yet finnished executing.
                        num_nodes_remaining -= 1;
                        // Place the node back into the graph.
                        match self.nodes.get_mut(node_id) {
                            Some(node_option) => node_option.replace(node),
                            None => panic!("Received WorkerStatus for node {}, but this node is not in the graph", node_id),
                        };

                        // Store the intermediate output. If it is already present,
                        // it means the node was executed more than once.
                        if intermediates.insert(node_id, Arc::new(result)).is_some() {
                            panic!("Node {} was executed more than once, possibly due to a cycle", node_id);
                        }

                        // See if it is possible to queue other nodes by checking on this node's outputs.
                        // If a node is not found in the output map, then it means it IS an output.
                        if let Some(output_ids) = recipe.node_outputs.get(&node_id) {
                            // Next, walk over all the output_ids of this node, decrementing
                            // their input counts.
                            for output_id in output_ids {
                                let remaining_inputs = match remaining_inputs_map.get_mut(output_id) {
                                    Some(id) => id,
                                    None => panic!("Node {} is not registered in the remaining_inputs_map", output_id)
                                };
                                remaining_inputs.remove(&node_id);
                                // If any hit 0, execute them.
                                if remaining_inputs.len() == 0 {
                                    // Assemble the required inputs and dispatch.
                                    let inputs = assemble_inputs(&self, &intermediates, output_id.clone());
                                    execute_node(self, *output_id, inputs);
                                }
                            } // for output_id in output_ids
                        } // if let Some(output_ids)
                    },
                    WorkerStatus::Fail(node_id) => panic!("Graph failed to execute because node {} failed", node_id),
                } // match wstatus
            }
        } // while num_nodes_remaining > 0

        // Return the outputs.
        let mut outputs_map = HashMap::new();
        for output in &recipe.outputs {
            // Unwrap the Arc to get the underlying Data.
            match Arc::try_unwrap(intermediates.remove(output).unwrap()) {
                Ok(data) => { outputs_map.insert(output.clone(), data); },
                Err(_) => panic!("Could not retrieve output for node {}", output),
            }
        }
        return outputs_map;
    } // fn run
} // impl
