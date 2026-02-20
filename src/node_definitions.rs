pub enum GenericNodeDefinition {
    Sampler(SamplerNodeDefinition),
    BlendTreeOneDim(BlendTreeOneDimDefinition)
}

pub struct SamplerNodeDefinition {
    animation_name: String,
    looping: bool
}

pub struct BlendTreeOneDimDefinition {
    animation_names: Vec<String>    
}