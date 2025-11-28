// Undo/Redo Command System
//
// Implements the Command pattern for event operations,
// enabling undo and redo functionality.

use crate::models::event::Event;
use crate::services::event::EventService;
use anyhow::Result;
use std::sync::Mutex;

/// Trait for undoable commands
pub trait Command: std::fmt::Debug {
    /// Execute the command (do/redo)
    fn execute(&self, event_service: &EventService) -> Result<()>;

    /// Undo the command
    fn undo(&self, event_service: &EventService) -> Result<()>;

    /// Get a human-readable description of the command
    fn description(&self) -> String;
}

/// Command for creating an event
/// Uses Mutex to allow updating the event ID after redo (recreation)
#[derive(Debug)]
pub struct CreateEventCommand {
    /// The event that was created (with ID set after execution)
    /// Wrapped in Mutex so we can update the ID after recreation
    event: Mutex<Event>,
}

impl CreateEventCommand {
    pub fn new(event: Event) -> Self {
        Self { event: Mutex::new(event) }
    }
}

impl Command for CreateEventCommand {
    fn execute(&self, event_service: &EventService) -> Result<()> {
        // Re-create the event (for redo) and update stored ID
        let mut event = self.event.lock().unwrap();
        let mut new_event = event.clone();
        new_event.id = None; // Clear ID so it creates a new record
        let created = event_service.create(new_event)?;
        // Update our stored event with the new ID so undo works
        event.id = created.id;
        Ok(())
    }

    fn undo(&self, event_service: &EventService) -> Result<()> {
        // Delete the event
        let event = self.event.lock().unwrap();
        if let Some(id) = event.id {
            event_service.delete(id)?;
        }
        Ok(())
    }

    fn description(&self) -> String {
        let event = self.event.lock().unwrap();
        format!("Create event \"{}\"", event.title)
    }
}

/// Command for updating an event
#[derive(Debug, Clone)]
pub struct UpdateEventCommand {
    /// The event state before the update
    pub old_event: Event,
    /// The event state after the update
    pub new_event: Event,
}

impl UpdateEventCommand {
    pub fn new(old_event: Event, new_event: Event) -> Self {
        Self {
            old_event,
            new_event,
        }
    }
}

impl Command for UpdateEventCommand {
    fn execute(&self, event_service: &EventService) -> Result<()> {
        event_service.update(&self.new_event)?;
        Ok(())
    }

    fn undo(&self, event_service: &EventService) -> Result<()> {
        event_service.update(&self.old_event)?;
        Ok(())
    }

    fn description(&self) -> String {
        format!("Update event \"{}\"", self.new_event.title)
    }
}

/// Command for deleting an event
/// Uses Mutex to allow updating the event ID after undo (recreation)
#[derive(Debug)]
pub struct DeleteEventCommand {
    /// The event that was deleted (stored for undo)
    /// Wrapped in Mutex so we can update the ID after recreation
    event: Mutex<Event>,
}

impl DeleteEventCommand {
    pub fn new(event: Event) -> Self {
        Self { event: Mutex::new(event) }
    }
}

impl Command for DeleteEventCommand {
    fn execute(&self, event_service: &EventService) -> Result<()> {
        let event = self.event.lock().unwrap();
        if let Some(id) = event.id {
            event_service.delete(id)?;
        }
        Ok(())
    }

    fn undo(&self, event_service: &EventService) -> Result<()> {
        // Re-create the event and update stored ID
        let mut event = self.event.lock().unwrap();
        let mut new_event = event.clone();
        new_event.id = None; // Clear ID so it creates a new record
        let created = event_service.create(new_event)?;
        // Update our stored event with the new ID so redo works
        event.id = created.id;
        Ok(())
    }

    fn description(&self) -> String {
        let event = self.event.lock().unwrap();
        format!("Delete event \"{}\"", event.title)
    }
}

/// Manager for undo/redo stacks
#[derive(Debug, Default)]
pub struct UndoManager {
    /// Stack of commands that can be undone
    undo_stack: Vec<Box<dyn Command + Send + Sync>>,
    /// Stack of commands that can be redone
    redo_stack: Vec<Box<dyn Command + Send + Sync>>,
    /// Maximum number of commands to keep in history
    max_history: usize,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 50,
        }
    }

    /// Push a command onto the undo stack after it has been executed
    pub fn push(&mut self, command: Box<dyn Command + Send + Sync>) {
        // Clear redo stack when a new command is executed
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(command);

        // Trim undo stack if it exceeds max history
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command
    pub fn undo(&mut self, event_service: &EventService) -> Result<Option<String>> {
        if let Some(command) = self.undo_stack.pop() {
            let description = command.description();
            command.undo(event_service)?;
            self.redo_stack.push(command);
            Ok(Some(description))
        } else {
            Ok(None)
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self, event_service: &EventService) -> Result<Option<String>> {
        if let Some(command) = self.redo_stack.pop() {
            let description = command.description();
            command.execute(event_service)?;
            self.undo_stack.push(command);
            Ok(Some(description))
        } else {
            Ok(None)
        }
    }

    /// Check if there are commands to undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands to redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the description of the next command to undo (owned string for menu display)
    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.last().map(|cmd| cmd.description())
    }

    /// Get the description of the next command to redo (owned string for menu display)
    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.last().map(|cmd| cmd.description())
    }

    /// Clear all history
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn create_test_event(title: &str) -> Event {
        Event {
            id: Some(1),
            title: title.to_string(),
            description: None,
            location: None,
            start: Local::now(),
            end: Local::now() + chrono::Duration::hours(1),
            all_day: false,
            category: None,
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_create_event_command_description() {
        let event = create_test_event("Team Meeting");
        let cmd = CreateEventCommand::new(event);
        assert_eq!(cmd.description(), "Create event \"Team Meeting\"");
    }

    #[test]
    fn test_update_event_command_description() {
        let old = create_test_event("Old Title");
        let new = create_test_event("New Title");
        let cmd = UpdateEventCommand::new(old, new);
        assert_eq!(cmd.description(), "Update event \"New Title\"");
    }

    #[test]
    fn test_delete_event_command_description() {
        let event = create_test_event("Meeting to Delete");
        let cmd = DeleteEventCommand::new(event);
        assert_eq!(cmd.description(), "Delete event \"Meeting to Delete\"");
    }

    #[test]
    fn test_undo_manager_can_undo_redo() {
        let mut manager = UndoManager::new();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());

        let event = create_test_event("Test");
        let cmd: Box<dyn Command + Send + Sync> = Box::new(CreateEventCommand::new(event));
        manager.push(cmd);

        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_undo_manager_max_history() {
        let mut manager = UndoManager::new();
        manager.max_history = 5;

        for i in 0..10 {
            let event = create_test_event(&format!("Event {}", i));
            let cmd: Box<dyn Command + Send + Sync> = Box::new(CreateEventCommand::new(event));
            manager.push(cmd);
        }

        // Should only have 5 items
        assert_eq!(manager.undo_stack.len(), 5);
    }

    #[test]
    fn test_undo_manager_redo_cleared_on_new_command() {
        let mut manager = UndoManager::new();

        // Push a command
        let event1 = create_test_event("Event 1");
        let cmd1: Box<dyn Command + Send + Sync> = Box::new(CreateEventCommand::new(event1));
        manager.push(cmd1);

        // Simulate undo by moving to redo stack manually (since we can't call actual undo without DB)
        if let Some(cmd) = manager.undo_stack.pop() {
            manager.redo_stack.push(cmd);
        }
        assert!(manager.can_redo());

        // Push a new command - redo stack should be cleared
        let event2 = create_test_event("Event 2");
        let cmd2: Box<dyn Command + Send + Sync> = Box::new(CreateEventCommand::new(event2));
        manager.push(cmd2);

        assert!(!manager.can_redo());
    }
}
