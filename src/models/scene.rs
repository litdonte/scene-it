use std::collections::HashMap;

use crate::models::{
    Id,
    metadata::{HasMetadata, Metadata},
    scene_elements::{SceneElement, heading::SceneHeading},
};

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Scene {
    id: Id<Self>,
    active_variant: Id<SceneVariant>,
    variants: HashMap<Id<SceneVariant>, SceneVariant>,
    metadata: Metadata,
}

impl Scene {
    pub fn new() -> Self {
        let variant = SceneVariant::new();
        let variant_id = variant.id.clone();
        let mut variants = HashMap::new();
        variants.insert(variant_id.clone(), variant);

        Self {
            id: Id::new(),
            active_variant: variant_id,
            variants,
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }
}

impl HasMetadata for Scene {
    fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

#[cfg(test)]
mod tests {}
