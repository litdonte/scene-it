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
    next: Option<Id<SceneVariant>>,
    metadata: Metadata,
}

impl SceneVariant {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            heading: None,
            elements: Vec::new(),
            summary: Summary::default(),
            next: None,
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn set_next(&mut self, next: Id<SceneVariant>) {
        self.next = Some(next)
    }

    pub fn clear_next(&mut self) {
        self.next = None
    }

    pub fn next(&self) -> Option<&Id<SceneVariant>> {
        self.next.as_ref()
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

    pub fn variants(&self) -> &HashMap<Id<SceneVariant>, SceneVariant> {
        &self.variants
    }

    pub fn variants_mut(&mut self) -> &mut HashMap<Id<SceneVariant>, SceneVariant> {
        &mut self.variants
    }

    pub fn variant_ids(&self) -> impl Iterator<Item = &Id<SceneVariant>> {
        self.variants.keys().clone()
    }

    pub fn has_variant(&self, variant_id: &Id<SceneVariant>) -> bool {
        self.variants.contains_key(variant_id)
    }

    pub fn active_variant(&self) -> &Id<SceneVariant> {
        &self.active_variant
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
