use time::UtcDateTime;

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[cfg(test)]
mod tests {}
