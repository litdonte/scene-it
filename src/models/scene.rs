use crate::models::{
    Id,
    metadata::Metadata,
    scene_elements::{SceneElement, heading::SceneHeading},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneVariant {
    id: Id<Self>,
    heading: Option<SceneHeading>,
    elements: Vec<SceneElement>,
    metadata: Metadata,
}

impl SceneVariant {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            heading: None,
            elements: Vec::new(),
            metadata: Metadata::new(),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Scene {
    id: Id<Self>,
    active_variant: Id<SceneVariant>,
    variants: Vec<SceneVariant>,
    metadata: Metadata,
}

impl Scene {
    pub fn new() -> Self {
        let variant = SceneVariant::new();

        Self {
            id: Id::new(),
            active_variant: variant.id.clone(),
            variants: vec![variant],
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {}
