use crate::animgraph_definition::*;
use crate::edges::*;
use crate::node_definitions::GenericNodeDefinition;
use crate::nodes::*;
use anyhow::anyhow;
use ozz_animation_rs::*;
use petgraph::algo::dijkstra;
use petgraph::graph::{EdgeIndex, Graph, NodeIndex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

pub struct AnimGraph {
    skeleton: Rc<Skeleton>,
    graph: Graph<GenericNode, TransitionIndex>,
    samplers: SamplerNodesContainer<SamplerNode>,
    blend_trees_one_dim: BlendTreeOneDimNodesContainer<BlendTreeOneDimNode>,
    transitions: TransitionsContainer<Transition>,
    root_node_idx: NodeIndex,
    current_node_idx: Option<NodeIndex>,
    current_edge_idx: Option<EdgeIndex>,
    target: NodeIndex,
    on_a_transition: bool,
    path: VecDeque<EdgeIndex>,
    node_names: HashMap<String, NodeIndex>,
    local_to_model_job: LocalToModelJobRc,
    output: Rc<RefCell<Vec<SoaTransform>>>,
    model_matrices: Rc<RefCell<Vec<glam::Mat4>>>,
}

impl AnimGraph {
    pub fn new(
        skeleton: Rc<Skeleton>,
        animgraph_definition: &AnimGraphDefinition,
        animations_by_name: &HashMap<String, Rc<Animation>>,
    ) -> Result<Self, anyhow::Error> {
        match animgraph_definition.root {
            Some(val) => {
                let node_opt = animgraph_definition.graph.node_weight(val);
                match node_opt {
                    Some(_) => {}
                    None => {
                        return Err(anyhow!("Invalid root node in animgraph definition"));
                    }
                }
            }
            None => {
                return Err(anyhow!("No root node found in animgraph definition"));
            }
        }
        let mut graph = Graph::<GenericNode, TransitionIndex>::with_capacity(
            animgraph_definition.graph.node_count(),
            animgraph_definition.graph.edge_count(),
        );
        let mut samplers = SamplerNodesContainer::<SamplerNode>::new();
        // Go over each node in the animgraph's definition and add it to the final graph, saving its definition node/final node pair in a map
        let mut node_definitions_to_node_mappings = HashMap::<NodeIndex, NodeIndex>::new();
        let mut node_names = HashMap::<String, NodeIndex>::new();
        for node_definition_idx in animgraph_definition.graph.node_indices() {
            let node_definition = animgraph_definition.graph.node_weight(node_definition_idx);
            match node_definition {
                Some(def) => {
                    match def {
                        GenericNodeDefinition::Sampler(val) => {
                            if !animations_by_name.contains_key(&val.animation_name) {
                                return Err(anyhow!(
                                    "Could not find animation name {}",
                                    &val.animation_name
                                ));
                            }
                            if node_names.contains_key(&val.name) {
                                return Err(anyhow!("Duplicate node name: {}", &val.name));
                            }
                            let animation = &animations_by_name[&val.animation_name];
                            let sampler_node =
                                SamplerNode::new(skeleton.clone(), animation.clone(), val.looping);
                            let sampler_idx = samplers.push(sampler_node);
                            let node_idx = graph.add_node(GenericNode::Sampler(
                                SamplerNodeIndex::from(sampler_idx),
                            ));
                            node_names.insert(val.name.clone(), node_idx);
                            node_definitions_to_node_mappings.insert(node_definition_idx, node_idx);
                        }
                        GenericNodeDefinition::BlendTreeOneDim(val) => {} // TODO: DO LATER
                    }
                }
                None => {
                    return Err(anyhow!("Invalid node definition"));
                }
            }
        }

        let blend_trees_one_dim = BlendTreeOneDimNodesContainer::<BlendTreeOneDimNode>::new();

        let mut transitions = TransitionsContainer::<Transition>::new();

        // Go over each edge in the animgraph's definition and add it to the final graph, using the node mapping to find the appropriate final node.
        for edge_index in animgraph_definition.graph.edge_indices() {
            let endpoints = animgraph_definition.graph.edge_endpoints(edge_index);
            match endpoints {
                Some(val) => {
                    if !node_definitions_to_node_mappings.contains_key(&val.0) {
                        return Err(anyhow!("Couldn't find \"from\" node from edge"));
                    }
                    if !node_definitions_to_node_mappings.contains_key(&val.1) {
                        return Err(anyhow!("Couldn't find \"to\" node from edge"));
                    }
                    let from_idx = node_definitions_to_node_mappings[&val.0];
                    let from_node = graph.node_weight(NodeIndex::from(from_idx));
                    let from_output: Rc<RefCell<Vec<SoaTransform>>>;
                    match from_node {
                        Some(node_def) => match node_def {
                            GenericNode::Sampler(val) => {
                                from_output = samplers[*val].output.clone();
                            }
                            GenericNode::BlendTreeOneDim(val) => {
                                from_output = blend_trees_one_dim[*val].output.clone();
                            }
                        },
                        None => {
                            return Err(anyhow!("Invalid \"from\" node"));
                        }
                    }
                    let to_idx = node_definitions_to_node_mappings[&val.1];
                    let to_node = graph.node_weight(to_idx);
                    let to_output: Rc<RefCell<Vec<SoaTransform>>>;
                    match to_node {
                        Some(node_def) => match node_def {
                            GenericNode::Sampler(val) => {
                                to_output = samplers[*val].output.clone();
                            }
                            GenericNode::BlendTreeOneDim(val) => {
                                to_output = blend_trees_one_dim[*val].output.clone();
                            }
                        },
                        None => {
                            return Err(anyhow!("Invalid \"from\" node"));
                        }
                    }
                    let edge_def = animgraph_definition.graph.edge_weight(edge_index);
                    let duration: web_time::Duration;
                    match edge_def {
                        Some(val) => {
                            duration = val.duration;
                        }
                        None => {
                            return Err(anyhow!(
                                "Invalid edge weight found while adding transition"
                            ));
                        }
                    }
                    let transition =
                        Transition::new(skeleton.clone(), duration, from_output, to_output);
                    let transition_idx = transitions.push(transition);
                    let _ = graph.add_edge(from_idx, to_idx, transition_idx);
                }
                None => return Err(anyhow!("Invalid edge endpoints")),
            }
        }

        let root_node_idx = node_definitions_to_node_mappings[&animgraph_definition.root.unwrap()];
        let current_node_idx = Some(root_node_idx);
        let target = root_node_idx;
        let path = VecDeque::<EdgeIndex>::new();

        let mut local_to_model_job = LocalToModelJobRc::default();
        local_to_model_job.set_skeleton(skeleton.clone());
        let current_node = graph.node_weight(root_node_idx).unwrap();
        let output: Rc<RefCell<Vec<SoaTransform>>>;
        match current_node {
            GenericNode::Sampler(val) => {
                local_to_model_job.set_input(samplers[*val].output.clone());
                output = samplers[*val].output.clone();
            }
            GenericNode::BlendTreeOneDim(val) => {
                local_to_model_job.set_input(blend_trees_one_dim[*val].output.clone());
                output = blend_trees_one_dim[*val].output.clone();
            }
        }

        let model_matrices = Rc::new(RefCell::new(vec![
            glam::Mat4::IDENTITY;
            skeleton.num_joints()
        ]));

        local_to_model_job.set_output(models.clone());

        Ok(AnimGraph {
            skeleton: skeleton.clone(),
            graph,
            samplers,
            blend_trees_one_dim,
            transitions,
            root_node_idx,
            current_node_idx,
            current_edge_idx: None,
            target,
            path,
            on_a_transition: false,
            node_names,
            local_to_model_job,
            output,
            model_matrices,
        })
    }

    pub fn evaluate(&mut self, dt: web_time::Duration) -> Result<(), anyhow::Error> {
        // Handle the transition case
        let mut ratio = 0.0;
        if self.on_a_transition {
            match self.current_edge_idx {
                Some(edge_idx) => {
                    let transition_idx = self.graph.edge_weight(edge_idx).unwrap();
                    // Calculate the time taken and check whether or not the transition is finished.
                    self.transitions[*transition_idx].seek += dt;
                    let finished: bool;
                    if self.transitions[*transition_idx].seek
                        >= self.transitions[*transition_idx].duration
                    {
                        finished = true;
                    } else {
                        finished = false;
                    }
                    // If finished, move onto the next node.
                    if finished {
                        self.transitions[*transition_idx].reset();
                        let front = self.path.front();
                        match front {
                            Some(val) => {
                                self.current_edge_idx = Some(*val);
                                self.path.pop_front();
                                self.on_a_transition = false;
                            }
                            None => {}
                        }
                    } else {
                        let edge = self
                            .graph
                            .edge_endpoints(self.current_edge_idx.unwrap())
                            .unwrap();
                        let from = self.graph.node_weight(edge.0).unwrap();
                        // Otherwise, evaluate the transition
                        match from {
                            GenericNode::Sampler(val) => {
                                self.samplers[*val].update(dt);
                            }
                            GenericNode::BlendTreeOneDim(val) => {
                                self.blend_trees_one_dim[*val].update(dt);
                            }
                        }
                        let to = self.graph.node_weight(edge.1).unwrap();
                        match to {
                            GenericNode::Sampler(val) => {
                                self.samplers[*val].update(dt);
                            }
                            GenericNode::BlendTreeOneDim(val) => {
                                self.blend_trees_one_dim[*val].update(dt);
                            }
                        }
                        // Set the blend layer weights based on time elapsed.
                        self.transitions[*transition_idx].seek =
                            self.transitions[*transition_idx].seek + dt;
                        ratio = (self.transitions[*transition_idx].duration.as_millis() as f32
                            / self.transitions[*transition_idx].seek.as_millis() as f32)
                            .clamp(0.0, 1.0);

                        self.transitions[*transition_idx].blend_job.layers_mut()[0].weight = ratio;
                        self.transitions[*transition_idx].blend_job.layers_mut()[0].weight =
                            1.0 - ratio;

                        let results = self.transitions[*transition_idx].blend_job.run();
                        match results {
                            Ok(_) => {} // Do nothing
                            Err(e) => {
                                return Err(anyhow! {"Ozz error during transition blend: {}", e});
                            }
                        }
                    }
                }
                None => return Err(anyhow! {"Invalid current transition during evaluation."}),
            }
        } else {
            // If we are on a node. Far simpler to evaluate
            match self.current_node_idx {
                Some(val) => {
                    let node = self.graph.node_weight(val).unwrap();
                    match node {
                        GenericNode::Sampler(val) => {
                            self.samplers[*val].update(dt);
                        }
                        GenericNode::BlendTreeOneDim(val) => {
                            self.blend_trees_one_dim[*val].update(dt);
                        }
                    }
                }
                None => return Err(anyhow! {"Invalid current node during evaluation."}),
            }
        }
        // Now we check whether or not we are in need of transitioning through the path list
        let mut first_time_on_transition = false;
        let mut first_time_on_node = false;
        if self.on_a_transition && ratio >= 1.0 {
            let last_transition_idx = self
                .graph
                .edge_weight(self.current_edge_idx.unwrap())
                .unwrap();
            self.transitions[*last_transition_idx].reset();
            match self.path.front() {
                Some(val) => {
                    self.current_edge_idx = Some(*val);
                    self.current_node_idx = None;
                    self.path.pop_front();
                    first_time_on_transition = true;
                }
                None => {
                    // The path is empty. We now set the current node/edge as the "to" node of the current edge
                    let t = self
                        .graph
                        .edge_endpoints(self.current_edge_idx.unwrap())
                        .unwrap()
                        .1;
                    if t != self.target {
                        return Err(anyhow! {"Path ended on a non-target node"});
                    }
                    self.current_node_idx = Some(t);
                    self.current_edge_idx = None;
                    first_time_on_node = true;
                }
            }
        }
        let l2m_results = self.local_to_model_job.run();
        match l2m_results {
            Ok(_) => {}
            Err(e) => return Err(anyhow! {"Error running local-to-model job: {}", e}),
        }
        // If this is the first time we're entering a transition or we find our final target, clone() the outputs to our local2model job's inputs
        if first_time_on_transition {
            let current_transition_idx = self
                .graph
                .edge_weight(self.current_edge_idx.unwrap())
                .unwrap();
            self.local_to_model_job.clear_input();
            self.local_to_model_job
                .set_input(self.transitions[*current_transition_idx].output.clone());
            self.output = self.transitions[*current_transition_idx].output.clone();
        } else if first_time_on_node {
            self.local_to_model_job.clear_input();
            let current_node = self
                .graph
                .node_weight(self.current_node_idx.unwrap())
                .unwrap();
            match current_node {
                GenericNode::Sampler(val) => {
                    self.local_to_model_job
                        .set_input(self.samplers[*val].output.clone());
                    self.output = self.samplers[*val].output.clone();
                }
                GenericNode::BlendTreeOneDim(val) => {
                    self.local_to_model_job
                        .set_input(self.blend_trees_one_dim[*val].output.clone());
                    self.output = self.blend_trees_one_dim[*val].output.clone();
                }
            }
        }

        Ok(())
    }

    pub fn get_skeletal_matrices(&mut self) -> Rc<RefCell<Vec<glam::Mat4>>> {
        let results = self.local_to_model_job.output().unwrap();
        results.clone()
    }

    pub fn get_soa_transforms(&mut self) -> Rc<RefCell<Vec<SoaTransform>>> {
        self.output.clone()
    }

    pub fn get_node_by_name(&mut self, node_name: String) -> Option<NodeIndex> {
        if self.node_names.contains_key(&node_name) {
            let idx = self.node_names[&node_name];
            return Some(idx);
        }
        None
    }

    pub fn set_target_node_by_idx(&mut self, node_idx: NodeIndex) {
        //self.dfs(node_idx);
    }

    pub fn set_target_node_by_name(&mut self, node_name: String) {
        if self.node_names.contains_key(&node_name) {
            let node_idx = self.node_names[&node_name];
            //self.dfs(node_idx);
        }
    }

}
