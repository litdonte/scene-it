use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::utils;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct RevisionNote(String);

impl RevisionNote {
    pub fn new(input: &str) -> Result<Self, MetadataError> {
        let note = utils::trim_input(input);

        if note.is_empty() {
            return Err(MetadataError::EmptyRevisionNote);
        }

        if note.chars().any(|c| c.is_control()) {
            return Err(MetadataError::RevisionNoteHasControlChars);
        }

        Ok(Self(note))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum MetadataError {
    EmptyRevisionNote,
    RevisionNoteHasControlChars,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Metadata {
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    pub version: u32,
    pub revision_notes: Vec<RevisionNote>,
    pub tags: Vec<String>, // Optional
    pub locked: bool,
}

impl Metadata {
    pub fn new() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            revision_notes: Vec::new(),
            tags: Vec::new(),
            locked: false,
        }
    }

    pub fn add_revision_note(&mut self, note: RevisionNote) {
        let now = OffsetDateTime::now_utc();
        self.revision_notes.push(note);
        self.updated_at = now;
        self.version += 1;
    }
}

pub trait HasMetadata {
    fn metadata(&self) -> &Metadata;
    fn metadata_mut(&mut self) -> &mut Metadata;

    fn touch(&mut self) {
        let meta = self.metadata_mut();
        meta.updated_at = OffsetDateTime::now_utc();
        meta.version += 1;
    }
}

#[cfg(test)]
mod tests {}
