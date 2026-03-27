use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{
    Id,
    metadata::{HasMetadata, Metadata},
    scene_elements::{SceneElement, heading::SceneHeading},
    summary::Summary,
};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SceneVariant {
    id: Id<Self>,
    heading: Option<SceneHeading>,
    elements: Vec<SceneElement>,
    summary: Summary,
    metadata: Metadata,
}

impl SceneVariant {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            heading: None,
            elements: Vec::new(),
            summary: Summary::default(),
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Scene {
    id: Id<Self>,
    active_variant: Id<SceneVariant>,
    variants: HashMap<Id<SceneVariant>, SceneVariant>,
    metadata: Metadata,
}

impl Scene {
    pub fn new() -> Self {
        let variant = SceneVariant::new();

        Self {
            id: Id::new(),
            active_variant: variant.id(),
            variants: HashMap::from([(variant.id(), variant)]),
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }

    pub fn summary(&self) -> Summary {
        self.variants
            .get(&self.active_variant)
            .map(|var| var.summary().clone())
            .unwrap_or_else(|| {
                Summary::new("Summary not available.").expect("Fallback summary is valid")
            })
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
