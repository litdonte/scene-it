use time::UtcDateTime;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Metadata {
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
    pub version: u32,
    pub revision_notes: Vec<String>,
    pub tags: Vec<String>, // Optional
    pub locked: bool,
}

impl Metadata {
    pub fn new() -> Self {
        let now = UtcDateTime::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            revision_notes: Vec::new(),
            tags: Vec::new(),
            locked: false,
        }
    }
}

pub trait HasMetadata {
    fn metadata(&self) -> &Metadata;
    fn metadata_mut(&mut self) -> &mut Metadata;

    fn touch(&mut self) {
        let now = UtcDateTime::now();
        let meta = self.metadata_mut();
        meta.updated_at = now;
        meta.version += 1;
    }
}

#[cfg(test)]
mod tests {}
