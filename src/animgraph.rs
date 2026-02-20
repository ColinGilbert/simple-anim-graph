use crate::animgraph_definition::*;
use crate::edges::*;
use crate::nodes::*;
use mapgraph::aliases::SlotMapGraph;
use ozz_animation_rs::*;
use std::collections::HashMap;
use std::rc::Rc;

safe_index::new! {
SamplerNodeIndex,
map: SamplerNodesContainer
}

safe_index::new! {
TransitionIndex,
map: TransitionsContainer
}

pub struct AnimGraph {
    skeleton: Rc<Skeleton>,
    graph: SlotMapGraph<GenericNode, TransitionIndex>,
    samplers: SamplerNodesContainer<SamplerNode>,
    transitions: TransitionsContainer<Transition>,
}

impl AnimGraph {
    pub fn new(
        skeleton: Rc<Skeleton>,
        definition: &AnimGraphDefinition,
        animations_by_name: &HashMap<String, Rc<Animation>>,
    ) -> Option<Self> {

        //AnimGraph {skeleton: skeleton.clone()}
        None
    }
}
