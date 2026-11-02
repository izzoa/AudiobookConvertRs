//! Undo/Redo system using the Command pattern
//!
//! Provides functionality for:
//! - Tracking editable operations as commands
//! - Undoing and redoing metadata/chapter changes
//! - Managing undo history with configurable depth

use crate::error::{AppError, AppResult};
use crate::models::{AudioBook, Chapter, Metadata};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, info};

/// Maximum number of undo steps to keep
const DEFAULT_MAX_UNDO_STEPS: usize = 50;

/// A command that can be executed, undone, and redone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditCommand {
    /// Update audiobook metadata
    UpdateMetadata {
        audiobook_id: String,
        old_metadata: Metadata,
        new_metadata: Metadata,
    },
    /// Update a single chapter
    UpdateChapter {
        audiobook_id: String,
        index: usize,
        old_chapter: Chapter,
        new_chapter: Chapter,
    },
    /// Add a new chapter
    AddChapter {
        audiobook_id: String,
        chapter: Chapter,
    },
    /// Delete a chapter
    DeleteChapter {
        audiobook_id: String,
        index: usize,
        chapter: Chapter,
    },
    /// Reorder chapters
    ReorderChapters {
        audiobook_id: String,
        old_order: Vec<usize>,
        new_order: Vec<usize>,
    },
    /// Replace all chapters (for imports/generation)
    ReplaceChapters {
        audiobook_id: String,
        old_chapters: Vec<Chapter>,
        new_chapters: Vec<Chapter>,
    },
    /// Batch update (for grouping multiple changes)
    Batch {
        description: String,
        commands: Vec<EditCommand>,
    },
}

impl EditCommand {
    /// Get a human-readable description of this command
    pub fn description(&self) -> String {
        match self {
            EditCommand::UpdateMetadata { .. } => "Update metadata".to_string(),
            EditCommand::UpdateChapter { index, .. } => format!("Update chapter {}", index + 1),
            EditCommand::AddChapter { chapter, .. } => format!("Add chapter \"{}\"", chapter.title),
            EditCommand::DeleteChapter { chapter, .. } => {
                format!("Delete chapter \"{}\"", chapter.title)
            }
            EditCommand::ReorderChapters { .. } => "Reorder chapters".to_string(),
            EditCommand::ReplaceChapters { new_chapters, .. } => {
                format!("Replace chapters ({} chapters)", new_chapters.len())
            }
            EditCommand::Batch { description, .. } => description.clone(),
        }
    }

    /// Get the audiobook ID this command operates on
    pub fn audiobook_id(&self) -> &str {
        match self {
            EditCommand::UpdateMetadata { audiobook_id, .. } => audiobook_id,
            EditCommand::UpdateChapter { audiobook_id, .. } => audiobook_id,
            EditCommand::AddChapter { audiobook_id, .. } => audiobook_id,
            EditCommand::DeleteChapter { audiobook_id, .. } => audiobook_id,
            EditCommand::ReorderChapters { audiobook_id, .. } => audiobook_id,
            EditCommand::ReplaceChapters { audiobook_id, .. } => audiobook_id,
            EditCommand::Batch { commands, .. } => {
                commands.first().map(|c| c.audiobook_id()).unwrap_or("")
            }
        }
    }
}

/// Apply a command to an audiobook (execute)
pub fn apply_command(audiobook: &mut AudioBook, command: &EditCommand) -> AppResult<()> {
    match command {
        EditCommand::UpdateMetadata { new_metadata, .. } => {
            audiobook.output_metadata = new_metadata.clone();
            audiobook.mark_modified();
        }
        EditCommand::UpdateChapter {
            index, new_chapter, ..
        } => {
            if *index >= audiobook.chapters.len() {
                return Err(AppError::InvalidChapter {
                    reason: format!("Chapter index {} out of range", index),
                });
            }
            audiobook.chapters[*index] = new_chapter.clone();
            audiobook.mark_modified();
        }
        EditCommand::AddChapter { chapter, .. } => {
            audiobook.chapters.push(chapter.clone());
            // Re-index chapters
            for (i, ch) in audiobook.chapters.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.mark_modified();
        }
        EditCommand::DeleteChapter { index, .. } => {
            if *index >= audiobook.chapters.len() {
                return Err(AppError::InvalidChapter {
                    reason: format!("Chapter index {} out of range", index),
                });
            }
            audiobook.chapters.remove(*index);
            // Re-index chapters
            for (i, ch) in audiobook.chapters.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.mark_modified();
        }
        EditCommand::ReorderChapters { new_order, .. } => {
            let mut reordered = Vec::with_capacity(audiobook.chapters.len());
            for &idx in new_order {
                if idx < audiobook.chapters.len() {
                    reordered.push(audiobook.chapters[idx].clone());
                }
            }
            // Re-index
            for (i, ch) in reordered.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.chapters = reordered;
            audiobook.mark_modified();
        }
        EditCommand::ReplaceChapters { new_chapters, .. } => {
            audiobook.chapters = new_chapters.clone();
            // Re-index
            for (i, ch) in audiobook.chapters.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.mark_modified();
        }
        EditCommand::Batch { commands, .. } => {
            for cmd in commands {
                apply_command(audiobook, cmd)?;
            }
        }
    }
    Ok(())
}

/// Unapply a command (undo)
pub fn unapply_command(audiobook: &mut AudioBook, command: &EditCommand) -> AppResult<()> {
    match command {
        EditCommand::UpdateMetadata { old_metadata, .. } => {
            audiobook.output_metadata = old_metadata.clone();
            audiobook.mark_modified();
        }
        EditCommand::UpdateChapter {
            index, old_chapter, ..
        } => {
            if *index >= audiobook.chapters.len() {
                return Err(AppError::InvalidChapter {
                    reason: format!("Chapter index {} out of range", index),
                });
            }
            audiobook.chapters[*index] = old_chapter.clone();
            audiobook.mark_modified();
        }
        EditCommand::AddChapter { .. } => {
            // Undo add = remove last chapter
            audiobook.chapters.pop();
            audiobook.mark_modified();
        }
        EditCommand::DeleteChapter { index, chapter, .. } => {
            // Undo delete = re-insert
            audiobook.chapters.insert(*index, chapter.clone());
            // Re-index
            for (i, ch) in audiobook.chapters.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.mark_modified();
        }
        EditCommand::ReorderChapters { old_order, .. } => {
            let current = audiobook.chapters.clone();
            let mut reordered = Vec::with_capacity(current.len());
            for &idx in old_order {
                if idx < current.len() {
                    reordered.push(current[idx].clone());
                }
            }
            for (i, ch) in reordered.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.chapters = reordered;
            audiobook.mark_modified();
        }
        EditCommand::ReplaceChapters { old_chapters, .. } => {
            audiobook.chapters = old_chapters.clone();
            // Re-index
            for (i, ch) in audiobook.chapters.iter_mut().enumerate() {
                ch.index = i;
            }
            audiobook.mark_modified();
        }
        EditCommand::Batch { commands, .. } => {
            // Undo in reverse order
            for cmd in commands.iter().rev() {
                unapply_command(audiobook, cmd)?;
            }
        }
    }
    Ok(())
}

/// Undo/Redo history manager
#[derive(Debug, Clone)]
pub struct UndoManager {
    /// Stack of commands that can be undone
    undo_stack: VecDeque<EditCommand>,
    /// Stack of commands that can be redone
    redo_stack: VecDeque<EditCommand>,
    /// Maximum number of undo steps
    max_steps: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoManager {
    /// Create a new undo manager
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_steps: DEFAULT_MAX_UNDO_STEPS,
        }
    }

    /// Create with custom max steps
    pub fn with_max_steps(max_steps: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_steps,
        }
    }

    /// Execute a command and add it to the undo stack
    pub fn execute(&mut self, audiobook: &mut AudioBook, command: EditCommand) -> AppResult<()> {
        apply_command(audiobook, &command)?;

        // A new edit makes this book's redo history meaningless; other books'
        // redo history is untouched.
        let book = command.audiobook_id().to_string();
        self.undo_stack.push_back(command);
        self.redo_stack.retain(|c| c.audiobook_id() != book);

        // Trim undo stack if too long
        while self.undo_stack.len() > self.max_steps {
            self.undo_stack.pop_front();
        }

        debug!(
            "Executed command, undo stack size: {}",
            self.undo_stack.len()
        );
        Ok(())
    }

    /// Undo this book's most recent edit.
    ///
    /// History is shared across every loaded book (a batch import holds many),
    /// so it is searched for the book's own last edit; another book's edit must
    /// never be replayed onto this one.
    pub fn undo(&mut self, audiobook: &mut AudioBook) -> AppResult<Option<String>> {
        let command = match take_last_for(&mut self.undo_stack, &audiobook.id) {
            Some(cmd) => cmd,
            None => return Ok(None),
        };

        let description = command.description();
        unapply_command(audiobook, &command)?;

        self.redo_stack.push_back(command);

        info!("Undid: {}", description);
        Ok(Some(description))
    }

    /// Redo this book's most recently undone edit.
    pub fn redo(&mut self, audiobook: &mut AudioBook) -> AppResult<Option<String>> {
        let command = match take_last_for(&mut self.redo_stack, &audiobook.id) {
            Some(cmd) => cmd,
            None => return Ok(None),
        };

        let description = command.description();
        apply_command(audiobook, &command)?;

        self.undo_stack.push_back(command);

        info!("Redid: {}", description);
        Ok(Some(description))
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the description of the next undo action
    pub fn peek_undo(&self) -> Option<String> {
        self.undo_stack.back().map(|c| c.description())
    }

    /// Get the description of the next redo action
    pub fn peek_redo(&self) -> Option<String> {
        self.redo_stack.back().map(|c| c.description())
    }

    /// Whether this book has an edit to undo.
    pub fn can_undo_for(&self, audiobook_id: &str) -> bool {
        self.undo_stack
            .iter()
            .any(|c| c.audiobook_id() == audiobook_id)
    }

    /// Whether this book has an undone edit to redo.
    pub fn can_redo_for(&self, audiobook_id: &str) -> bool {
        self.redo_stack
            .iter()
            .any(|c| c.audiobook_id() == audiobook_id)
    }

    /// Description of this book's next undo.
    pub fn peek_undo_for(&self, audiobook_id: &str) -> Option<String> {
        self.undo_stack
            .iter()
            .rev()
            .find(|c| c.audiobook_id() == audiobook_id)
            .map(|c| c.description())
    }

    /// Description of this book's next redo.
    pub fn peek_redo_for(&self, audiobook_id: &str) -> Option<String> {
        self.redo_stack
            .iter()
            .rev()
            .find(|c| c.audiobook_id() == audiobook_id)
            .map(|c| c.description())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get number of available undo steps
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get number of available redo steps
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

/// Removes and returns the newest command in `stack` that edits this book.
fn take_last_for(stack: &mut VecDeque<EditCommand>, audiobook_id: &str) -> Option<EditCommand> {
    let index = stack
        .iter()
        .rposition(|c| c.audiobook_id() == audiobook_id)?;
    stack.remove(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_audiobook() -> AudioBook {
        let mut ab = AudioBook::new("Test".to_string());
        ab.chapters.push(Chapter {
            index: 0,
            title: "Chapter 1".to_string(),
            start_time_ms: 0,
            end_time_ms: 60000,
            auto_detected: false,
        });
        ab
    }

    fn retitle(manager: &mut UndoManager, book: &mut AudioBook, title: &str) {
        let old_metadata = book.output_metadata.clone();
        let mut new_metadata = old_metadata.clone();
        new_metadata.title = Some(title.to_string());
        let command = EditCommand::UpdateMetadata {
            audiobook_id: book.id.clone(),
            old_metadata,
            new_metadata,
        };
        manager.execute(book, command).unwrap();
    }

    #[test]
    fn undo_only_touches_the_book_it_is_run_on() {
        let mut manager = UndoManager::new();
        let mut dune = AudioBook::new("Dune".to_string());
        let mut emma = AudioBook::new("Emma".to_string());

        retitle(&mut manager, &mut dune, "Dune (edited)");
        retitle(&mut manager, &mut emma, "Emma (edited)");

        // Dune's undo must find Dune's edit even though Emma's is newer.
        manager.undo(&mut dune).unwrap();
        assert_eq!(dune.output_metadata.title, None);
        assert_eq!(emma.output_metadata.title.as_deref(), Some("Emma (edited)"));

        assert!(manager.can_undo_for(&emma.id));
        assert!(!manager.can_undo_for(&dune.id));
        assert!(manager.can_redo_for(&dune.id));

        // An edit to Emma leaves Dune's redo in place.
        retitle(&mut manager, &mut emma, "Emma (again)");
        assert!(manager.can_redo_for(&dune.id));
        manager.redo(&mut dune).unwrap();
        assert_eq!(dune.output_metadata.title.as_deref(), Some("Dune (edited)"));
    }

    #[test]
    fn test_undo_redo_metadata() {
        let mut ab = create_test_audiobook();
        let mut manager = UndoManager::new();

        let old_metadata = ab.output_metadata.clone();
        let mut new_metadata = old_metadata.clone();
        new_metadata.title = Some("New Title".to_string());

        let cmd = EditCommand::UpdateMetadata {
            audiobook_id: ab.id.clone(),
            old_metadata,
            new_metadata: new_metadata.clone(),
        };

        manager.execute(&mut ab, cmd).unwrap();
        assert_eq!(ab.output_metadata.title, Some("New Title".to_string()));

        manager.undo(&mut ab).unwrap();
        assert_eq!(ab.output_metadata.title, None);

        manager.redo(&mut ab).unwrap();
        assert_eq!(ab.output_metadata.title, Some("New Title".to_string()));
    }

    #[test]
    fn test_undo_redo_chapter() {
        let mut ab = create_test_audiobook();
        let mut manager = UndoManager::new();

        let old_chapter = ab.chapters[0].clone();
        let mut new_chapter = old_chapter.clone();
        new_chapter.title = "Updated Chapter".to_string();

        let cmd = EditCommand::UpdateChapter {
            audiobook_id: ab.id.clone(),
            index: 0,
            old_chapter,
            new_chapter,
        };

        manager.execute(&mut ab, cmd).unwrap();
        assert_eq!(ab.chapters[0].title, "Updated Chapter");

        manager.undo(&mut ab).unwrap();
        assert_eq!(ab.chapters[0].title, "Chapter 1");
    }

    #[test]
    fn test_add_delete_chapter() {
        let mut ab = create_test_audiobook();
        let mut manager = UndoManager::new();

        let new_chapter = Chapter {
            index: 1,
            title: "Chapter 2".to_string(),
            start_time_ms: 60000,
            end_time_ms: 120000,
            auto_detected: false,
        };

        let cmd = EditCommand::AddChapter {
            audiobook_id: ab.id.clone(),
            chapter: new_chapter,
        };

        manager.execute(&mut ab, cmd).unwrap();
        assert_eq!(ab.chapters.len(), 2);

        manager.undo(&mut ab).unwrap();
        assert_eq!(ab.chapters.len(), 1);

        manager.redo(&mut ab).unwrap();
        assert_eq!(ab.chapters.len(), 2);
    }

    #[test]
    fn test_undo_clears_redo() {
        let mut ab = create_test_audiobook();
        let mut manager = UndoManager::new();

        // Execute first command
        let cmd1 = EditCommand::UpdateMetadata {
            audiobook_id: ab.id.clone(),
            old_metadata: ab.output_metadata.clone(),
            new_metadata: {
                let mut m = ab.output_metadata.clone();
                m.title = Some("Title 1".to_string());
                m
            },
        };
        manager.execute(&mut ab, cmd1).unwrap();

        // Undo
        manager.undo(&mut ab).unwrap();
        assert!(manager.can_redo());

        // Execute new command - should clear redo
        let cmd2 = EditCommand::UpdateMetadata {
            audiobook_id: ab.id.clone(),
            old_metadata: ab.output_metadata.clone(),
            new_metadata: {
                let mut m = ab.output_metadata.clone();
                m.title = Some("Title 2".to_string());
                m
            },
        };
        manager.execute(&mut ab, cmd2).unwrap();

        assert!(!manager.can_redo());
    }
}
