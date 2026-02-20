use crate::animgraph_definition::*;
use crate::edges::*;
use crate::node_definitions::GenericNodeDefinition;
use crate::nodes::*;
use anyhow::anyhow;
use mapgraph::aliases::SlotMapGraph;
use mapgraph::map::slotmap::NodeIndex;
use ozz_animation_rs::*;
use std::collections::HashMap;
use std::rc::Rc;


pub struct AnimGraph {
    skeleton: Rc<Skeleton>,
    graph: SlotMapGraph<GenericNode, TransitionIndex>,
    samplers: SamplerNodesContainer<SamplerNode>,
    transitions: TransitionsContainer<Transition>,
    root: NodeIndex,
    node_names: HashMap::<String, NodeIndex>,
}

impl AnimGraph {
    pub fn new(
        skeleton: Rc<Skeleton>,
        animgraph_definition: &AnimGraphDefinition,
        animations_by_name: &HashMap<String, Rc<Animation>>,
    ) -> Result<Self, anyhow::Error> {
        let mut graph = SlotMapGraph::<GenericNode, TransitionIndex>::with_capacities(animgraph_definition.graph.nodes_count(), animgraph_definition.graph.edges_count());
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
                    let sampler_node = SamplerNode::new(skeleton.clone(), animation.clone(), val.looping);
                    let sampler_idx = samplers.push(sampler_node);
                    let node_idx = graph.add_node(GenericNode::Sampler(SamplerNodeIndex::from(sampler_idx)));
                    node_names.insert(val.name.clone(), node_idx);
                    node_mappings.insert(node_definition_idx, node_idx);

                }
                GenericNodeDefinition::BlendTreeOneDim(val) => {} // DO LATER
            }
        }
        
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
                    let from = node_mappings[&val.from()];
                    let to = node_mappings[&val.to()];
                    let transition = Transition {duration: val.weight().duration};
                    let transition_idx = transitions.push(transition);
                    let _ = graph.add_edge(transition_idx, from, to);

                },
                None => {
                    return Err(anyhow!("Invalid edge in graph definition"));
                }
            }
        }
        let root = node_mappings[&animgraph_definition.root];
        
        Ok(AnimGraph { skeleton: skeleton.clone(), graph, samplers, transitions, root, node_names  })
    }
}
