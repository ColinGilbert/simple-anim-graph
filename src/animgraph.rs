use crate::animgraph_definition::*;
use crate::edges::*;
use crate::node_definitions::GenericNodeDefinition;
use crate::nodes::*;
use anyhow::anyhow;
use mapgraph::aliases::SlotMapGraph;
use mapgraph::map::slotmap::EdgeIndex;
use mapgraph::map::slotmap::NodeIndex;
use ozz_animation_rs::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::rc::Rc;

pub struct AnimGraph {
    skeleton: Rc<Skeleton>,
    graph: SlotMapGraph<GenericNode, TransitionIndex>,
    samplers: SamplerNodesContainer<SamplerNode>,
    blend_trees_one_dim: BlendTreeOneDimNodesContainer<BlendTreeOneDimNode>,
    transitions: TransitionsContainer<Transition>,
    root_node_idx: NodeIndex,
    current_node_idx: Option<NodeIndex>,
    current_edge_idx: Option<EdgeIndex>,
    target: NodeIndex,
    on_a_transition: bool,
    path: VecDeque<EdgeIndex>,
    dfs_node_under_evaluation: Option<NodeIndex>,
    dfs_temp_edges_stack: Vec<EdgeIndex>,
    dfs_visited: HashSet<NodeIndex>,
    node_names: HashMap<String, NodeIndex>,
    local_to_model_job: LocalToModelJobRc,
    ozz_outputs: Rc<RefCell<Vec<SoaTransform>>>
}

impl AnimGraph {
    pub fn new(
        skeleton: Rc<Skeleton>,
        animgraph_definition: &AnimGraphDefinition,
        animations_by_name: &HashMap<String, Rc<Animation>>,
    ) -> Result<Self, anyhow::Error> {
        match animgraph_definition.root {
            Some(val) => {
                let node_opt = animgraph_definition.graph.node(val);
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
        let mut graph = SlotMapGraph::<GenericNode, TransitionIndex>::with_capacities(
            animgraph_definition.graph.nodes_count(),
            animgraph_definition.graph.edges_count(),
        );
        let mut samplers = SamplerNodesContainer::<SamplerNode>::new();
        // Go over each node in the animgraph's definition and add it to the final graph, saving its definition node/final node pair in a map
        let mut node_mappings = HashMap::<NodeIndex, NodeIndex>::new();
        let mut node_names = HashMap::<String, NodeIndex>::new();
        for (node_definition_idx, node_definition) in animgraph_definition.graph.node_weights() {
            match node_definition {
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
                    let node_idx =
                        graph.add_node(GenericNode::Sampler(SamplerNodeIndex::from(sampler_idx)));
                    node_names.insert(val.name.clone(), node_idx);
                    node_mappings.insert(node_definition_idx, node_idx);
                }
                GenericNodeDefinition::BlendTreeOneDim(val) => {} // DO LATER
            }
        }
        // TODO
        let blend_trees_one_dim = BlendTreeOneDimNodesContainer::<BlendTreeOneDimNode>::new();

        let mut transitions = TransitionsContainer::<Transition>::new();

        // Go over each edge in the animgraph's definition and add it to the final graph, using the node mapping to find the appropriate final node.
        for (edge_definition_idx, _) in animgraph_definition.graph.edge_weights() {
            let edge_definition = animgraph_definition.graph.edge(edge_definition_idx);
            match edge_definition {
                Some(val) => {
                    if !node_mappings.contains_key(&val.from()) {
                        return Err(anyhow!("Invalid \"from\" node in edge"));
                    }
                    if !node_mappings.contains_key(&val.to()) {
                        return Err(anyhow!("Invalid \"to\" node in edge"));
                    }
                    let from_idx = node_mappings[&val.from()];
                    let from_node = graph.node(from_idx).unwrap().weight();
                    let from_output: Rc<RefCell<Vec<SoaTransform>>>;
                    match from_node {
                        GenericNode::Sampler(val) => {
                            from_output = samplers[*val].output.clone();
                        }
                        GenericNode::BlendTreeOneDim(val) => {
                            from_output = blend_trees_one_dim[*val].output.clone();
                        }
                    }
                    let to_idx = node_mappings[&val.to()];
                    let to_node = graph.node(to_idx).unwrap().weight();
                    let to_output: Rc<RefCell<Vec<SoaTransform>>>;
                    match to_node {
                        GenericNode::Sampler(val) => {
                            to_output = samplers[*val].output.clone();
                        }
                        GenericNode::BlendTreeOneDim(val) => {
                            to_output = blend_trees_one_dim[*val].output.clone();
                        }
                    }
                    let transition = Transition::new(
                        skeleton.clone(),
                        val.weight().duration,
                        from_output,
                        to_output,
                    );
                    let transition_idx = transitions.push(transition);
                    let _ = graph.add_edge(transition_idx, from_idx, to_idx);
                }
                None => {
                    return Err(anyhow!("Invalid edge found in graph definition"));
                }
            }
        }
        let root_node_idx = node_mappings[&animgraph_definition.root.unwrap()];
        let current_node_idx = Some(root_node_idx);
        let target = root_node_idx;
        let dfs_node_under_evaluation = None;
        let path = VecDeque::<EdgeIndex>::new();

        let mut local_to_model_job = LocalToModelJobRc::default();
        local_to_model_job.set_skeleton(skeleton.clone());
        let current_node = graph.node(root_node_idx).unwrap().weight();
        let ozz_outputs: Rc<RefCell<Vec<SoaTransform>>>;
        match current_node {
            GenericNode::Sampler(val) => {
                local_to_model_job.set_input(samplers[*val].output.clone());
                ozz_outputs = samplers[*val].output.clone();
            }
            GenericNode::BlendTreeOneDim(val) => {
                local_to_model_job.set_input(blend_trees_one_dim[*val].output.clone());
                ozz_outputs = blend_trees_one_dim[*val].output.clone();
            }
        }

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
            dfs_temp_edges_stack: Vec::<EdgeIndex>::new(),
            dfs_visited: HashSet::<NodeIndex>::new(),
            dfs_node_under_evaluation,
            node_names,
            local_to_model_job,
            ozz_outputs
        })
    }

    pub fn evaluate(&mut self, dt: web_time::Duration) -> Result<(), anyhow::Error> {
        // Handle the transition case
        let mut ratio = 0.0;
        if self.on_a_transition {
            match self.current_edge_idx {
                Some(val) => {
                    let edge = self.graph.edge(val).unwrap();
                    let transition_idx = edge.weight();
                    let from = self.graph.node(edge.from()).unwrap().weight();
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
                        // Otherwise, evaluate the transition
                        match from {
                            GenericNode::Sampler(val) => {
                                self.samplers[*val].update(dt);
                            }
                            GenericNode::BlendTreeOneDim(val) => {
                                self.blend_trees_one_dim[*val].update(dt);
                            }
                        }
                        let to = self.graph.node(edge.to()).unwrap().weight();
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
                            Ok(_) => {}
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
                    // TODO: Evaluate node
                    let node = self.graph.node(val).unwrap().weight();
                    match node {
                        GenericNode::Sampler(val) => {
                            self.samplers[*val].update(dt);
                            // : Get results of sampler to output
                        }
                        GenericNode::BlendTreeOneDim(val) => {
                            self.blend_trees_one_dim[*val].update(dt);
                        }
                    }
                }
                None => return Err(anyhow! {"Invalid current node during evaluation."}),
            }
        }
        // Now we check whether we are in need of transitioning through the path list, or if we're on target
        let mut first_time_on_transition = false;
        let mut first_time_on_node = false;
        if self.on_a_transition && ratio >= 1.0 {
            let last_transition_idx = self
                .graph
                .edge(self.current_edge_idx.unwrap())
                .unwrap()
                .weight();
            self.transitions[*last_transition_idx].reset();
            match self.path.front() {
                Some(val) => {
                    self.current_edge_idx = Some(*val);
                    self.current_node_idx = None;
                    self.path.pop_front();
                    first_time_on_transition = true;
                }
                None => {
                    let t = self
                        .graph
                        .edge(self.current_edge_idx.unwrap())
                        .unwrap()
                        .to();
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
            Err(e) => {return Err(anyhow!{"Error running local-to-model job: {}", e})}
        }
        // If this is the first time we're entering a transition or we find our final target, clone() the outputs to our local2model job's inputs
        if first_time_on_transition {
            let current_transition_idx = self
                .graph
                .edge(self.current_edge_idx.unwrap())
                .unwrap()
                .weight();
            self.local_to_model_job.clear_input();
            self.local_to_model_job
                .set_input(self.transitions[*current_transition_idx].output.clone());
            self.ozz_outputs = self.transitions[*current_transition_idx].output.clone();
        }
        if first_time_on_node {
            self.local_to_model_job.clear_input();

            let current_node = self
                .graph
                .node(self.current_node_idx.unwrap())
                .unwrap()
                .weight();
            match current_node {
                GenericNode::Sampler(val) => {
                    self.local_to_model_job
                        .set_input(self.samplers[*val].output.clone());
                    self.ozz_outputs = self.samplers[*val].output.clone();
                }
                GenericNode::BlendTreeOneDim(val) => {
                    self.local_to_model_job
                        .set_input(self.blend_trees_one_dim[*val].output.clone());
                    self.ozz_outputs = self.blend_trees_one_dim[*val].output.clone();
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
        self.ozz_outputs.clone()
    }

    pub fn set_target_node_by_idx(&mut self, node_idx: NodeIndex) {
        self.dfs(node_idx);
    }

    pub fn set_target_node_by_name(&mut self, node_name: String) {
        if self.node_names.contains_key(&node_name) {
            let node_idx = self.node_names[&node_name];
            self.dfs(node_idx);
        }
    }

    pub fn get_node_by_name(&mut self, node_name: String) -> Option<NodeIndex> {
        if self.node_names.contains_key(&node_name) {
            let idx = self.node_names[&node_name];
            return Some(idx);
        }
        None
    }

    fn dfs(&mut self, target: NodeIndex) {
        self.target = target;
        self.path.clear();
        self.dfs_temp_edges_stack.clear();
        self.dfs_visited.clear();
        self.dfs_node_under_evaluation = self.current_node_idx;
        self.dfs_helper();
    }

    fn dfs_helper(&mut self) {
        // Get the last item in the path, check to see if its been visited, add it to path stack, and add to the visited set
        if self
            .dfs_visited
            .contains(&self.dfs_node_under_evaluation.unwrap())
        {
            self.dfs_temp_edges_stack.pop();
        } else {
            self.dfs_visited
                .insert(self.dfs_node_under_evaluation.unwrap());
        }
        let mut backtracking = true;
        for (edge_index, edge_ref) in self.graph.outputs(self.dfs_node_under_evaluation.unwrap()) {
            if !self.dfs_visited.contains(&edge_ref.to()) {
                backtracking = false;
                self.dfs_temp_edges_stack.push(edge_index);
                self.dfs_node_under_evaluation = Some(edge_ref.to());
                break;
            }
        }
        if backtracking {
            let last_node = self
                .graph
                .edge(*self.dfs_temp_edges_stack.last().unwrap())
                .unwrap()
                .from();
            self.dfs_node_under_evaluation = Some(last_node);
            self.dfs_temp_edges_stack.pop();
        }
        // Check to see if the work is finished and return if so.
        let mut finished = false;
        match self.dfs_temp_edges_stack.last() {
            Some(val) => {
                let n = self.graph.edge(*val).unwrap().to();
                if n == self.target {
                    finished = true;
                }
            }
            None => {}
        }
        if finished {
            for e in &self.dfs_temp_edges_stack {
                self.path.push_back(*e);
            }
            return;
        }
        self.dfs_helper();
    }
}
