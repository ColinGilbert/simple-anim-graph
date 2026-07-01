use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;

use crate::{edge_definitions::TransitionDefinition, node_definitions::GenericNodeDefinition};

pub struct AnimGraphDefinition {
    pub graph: Graph<GenericNodeDefinition, TransitionDefinition>,
    pub root: Option<NodeIndex>,
}

impl AnimGraphDefinition {
    pub fn new() -> Self {
        let graph: Graph<GenericNodeDefinition, TransitionDefinition> = Graph::<GenericNodeDefinition, TransitionDefinition>::default();
        Self {
            graph,
            root: None
        }
    }
}