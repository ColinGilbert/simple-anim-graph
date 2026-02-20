use mapgraph::aliases::SlotMapGraph;

use crate::{edge_definitions::TransitionDefinition, node_definitions::GenericNodeDefinition};

pub struct AnimGraphDefinition {
    pub graph: SlotMapGraph<GenericNodeDefinition, TransitionDefinition>
}