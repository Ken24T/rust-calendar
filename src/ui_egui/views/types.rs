//! Shared types for calendar views.
//!
//! This module contains common data structures used across different view types.

use chrono::{DateTime, Local, NaiveDate, NaiveTime};

use crate::models::event::Event;

/// Request for delete confirmation dialog.
#[derive(Clone)]
pub struct DeleteConfirmRequest {
    /// ID of the event to delete
    pub event_id: i64,
    /// Title of the event (for display in confirmation dialog)
    pub event_title: String,
    /// If true, only delete this occurrence (for recurring events)
    pub occurrence_only: bool,
    /// The occurrence date (needed for occurrence-only deletion)
    pub occurrence_date: Option<DateTime<Local>>,
}

/// Result of event interactions in views (context menus, clicks, etc.)
///
/// This struct collects all the actions that need to be processed after
/// rendering a view, such as events to edit, delete confirmations, etc.
#[derive(Default)]
pub struct EventInteractionResult {
    /// Event that was clicked for editing
    pub event_to_edit: Option<Event>,
    /// IDs of events that were deleted (need countdown card cleanup)
    pub deleted_event_ids: Vec<i64>,
    /// Events that were moved via drag-and-drop (need countdown card sync)
    pub moved_events: Vec<Event>,
    /// Request to show delete confirmation dialog
    pub delete_confirm_request: Option<DeleteConfirmRequest>,
    /// Request to create event from template (template_id, date, optional time)
    pub template_selection: Option<(i64, NaiveDate, Option<NaiveTime>)>,
    /// Undo requests: (old_event, new_event) pairs for drag/resize operations
    pub undo_requests: Vec<(Event, Event)>,
}

impl EventInteractionResult {
    /// Merge another result into this one.
    ///
    /// This is useful when multiple sub-components produce results that need
    /// to be combined before returning to the caller.
    pub fn merge(&mut self, other: EventInteractionResult) {
        if other.event_to_edit.is_some() {
            self.event_to_edit = other.event_to_edit;
        }
        self.deleted_event_ids.extend(other.deleted_event_ids);
        self.moved_events.extend(other.moved_events);
        if other.delete_confirm_request.is_some() {
            self.delete_confirm_request = other.delete_confirm_request;
        }
        if other.template_selection.is_some() {
            self.template_selection = other.template_selection;
        }
        self.undo_requests.extend(other.undo_requests);
    }
    
    /// Check if any action needs to be processed.
    pub fn has_actions(&self) -> bool {
        self.event_to_edit.is_some()
            || !self.deleted_event_ids.is_empty()
            || !self.moved_events.is_empty()
            || self.delete_confirm_request.is_some()
            || self.template_selection.is_some()
            || !self.undo_requests.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_interaction_result_default() {
        let result = EventInteractionResult::default();
        assert!(result.event_to_edit.is_none());
        assert!(result.deleted_event_ids.is_empty());
        assert!(result.moved_events.is_empty());
        assert!(result.delete_confirm_request.is_none());
        assert!(result.template_selection.is_none());
        assert!(result.undo_requests.is_empty());
        assert!(!result.has_actions());
    }

    #[test]
    fn test_event_interaction_result_merge() {
        let mut result1 = EventInteractionResult::default();
        result1.deleted_event_ids.push(1);

        let mut result2 = EventInteractionResult::default();
        result2.deleted_event_ids.push(2);
        result2.delete_confirm_request = Some(DeleteConfirmRequest {
            event_id: 3,
            event_title: "Test".to_string(),
            occurrence_only: false,
            occurrence_date: None,
        });

        result1.merge(result2);
        
        assert_eq!(result1.deleted_event_ids.len(), 2);
        assert!(result1.delete_confirm_request.is_some());
        assert!(result1.has_actions());
    }
}
