use mapgraph::{aliases::SlotMapGraph, map::slotmap::NodeIndex};

use crate::{edge_definitions::TransitionDefinition, node_definitions::GenericNodeDefinition};

pub struct AnimGraphDefinition {
    pub graph: SlotMapGraph<GenericNodeDefinition, TransitionDefinition>,
    pub root: NodeIndex
}