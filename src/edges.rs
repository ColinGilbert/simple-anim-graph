safe_index::new! {
TransitionIndex,
map: TransitionsContainer
}

pub struct Transition {
    pub duration: web_time::Duration,
}