safe_index::new! {
    TransitionDefinitionIndex,
    map: TransitionDefinitionContainer,
}

pub struct TransitionDefinition {
    pub duration: web_time::Duration,
}

