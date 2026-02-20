

pub enum GenericNodeDefinition {
    Sampler(SamplerNodeDefinition),
    BlendTreeOneDim(BlendTreeOneDimDefinition)
}

pub struct SamplerNodeDefinition {
    pub speed: f32,
    pub animation_name: String,
    pub looping: bool,
}

pub struct BlendTreeOneDimDefinition {
    animation_names: Vec<String>    
}