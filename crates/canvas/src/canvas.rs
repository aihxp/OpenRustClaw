//! Core Canvas implementation for managing UI elements and state

use crate::elements::CanvasElement;
use crate::error::{CanvasError, CanvasResult};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{debug, trace, warn};
use uuid::Uuid;

/// Default channel capacity for broadcast updates
const DEFAULT_CHANNEL_CAPACITY: usize = 128;

/// A Live Canvas for real-time visual agent interaction
#[derive(Debug)]
pub struct Canvas {
    id: Uuid,
    title: String,
    elements: Vec<CanvasElement>,
    state: HashMap<String, Value>,
    tx: broadcast::Sender<CanvasUpdate>,
}

/// Update broadcast to all connected clients
#[derive(Clone, Debug)]
pub struct CanvasUpdate {
    pub canvas_id: Uuid,
    pub action: CanvasAction,
}

/// Actions that can be performed on a canvas
#[derive(Clone, Debug)]
pub enum CanvasAction {
    /// Push new elements to the canvas
    Push { elements: Vec<CanvasElement> },
    /// Reset/clear all elements
    Reset,
    /// Update a specific element
    Update {
        element_id: Uuid,
        element: CanvasElement,
    },
    /// Remove a specific element
    Remove { element_id: Uuid },
    /// Execute JavaScript on the client
    Eval { script: String },
}

impl Canvas {
    /// Create a new canvas with the given title
    pub fn new(title: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);
        let id = Uuid::new_v4();

        debug!(canvas_id = %id, "Creating new canvas");

        Self {
            id,
            title: title.into(),
            elements: Vec::new(),
            state: HashMap::new(),
            tx,
        }
    }

    /// Create a new canvas with a specific ID
    pub fn with_id(id: Uuid, title: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);

        debug!(canvas_id = %id, "Creating new canvas with specific ID");

        Self {
            id,
            title: title.into(),
            elements: Vec::new(),
            state: HashMap::new(),
            tx,
        }
    }

    /// Get the canvas ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the canvas title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the canvas title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Push new elements to the canvas
    pub fn push(&mut self, elements: Vec<CanvasElement>) {
        if elements.is_empty() {
            return;
        }

        trace!(canvas_id = %self.id, count = elements.len(), "Pushing elements to canvas");

        self.elements.extend(elements.clone());

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Push { elements },
        };

        let _ = self.tx.send(update);
    }

    /// Add a single element to the canvas
    pub fn add(&mut self, element: CanvasElement) {
        self.push(vec![element]);
    }

    /// Reset/clear all elements from the canvas
    pub fn reset(&mut self) {
        debug!(canvas_id = %self.id, "Resetting canvas");

        self.elements.clear();

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Reset,
        };

        let _ = self.tx.send(update);
    }

    /// Update a specific element by ID
    pub fn update(&mut self, element_id: Uuid, element: CanvasElement) -> CanvasResult<()> {
        trace!(canvas_id = %self.id, element_id = %element_id, "Updating element");

        let index = self
            .elements
            .iter()
            .position(|e| e.id() == element_id)
            .ok_or_else(|| CanvasError::element_not_found(element_id.to_string()))?;

        self.elements[index] = element.clone();

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Update {
                element_id,
                element,
            },
        };

        let _ = self.tx.send(update);

        Ok(())
    }

    /// Remove a specific element by ID
    pub fn remove(&mut self, element_id: Uuid) -> CanvasResult<()> {
        trace!(canvas_id = %self.id, element_id = %element_id, "Removing element");

        let index = self
            .elements
            .iter()
            .position(|e| e.id() == element_id)
            .ok_or_else(|| CanvasError::element_not_found(element_id.to_string()))?;

        self.elements.remove(index);

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Remove { element_id },
        };

        let _ = self.tx.send(update);

        Ok(())
    }

    /// Execute JavaScript on connected clients
    pub fn eval(&self, script: impl Into<String>) {
        let script = script.into();
        trace!(canvas_id = %self.id, script_len = script.len(), "Executing script on canvas");

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Eval { script },
        };

        let _ = self.tx.send(update);
    }

    /// Get all elements
    pub fn elements(&self) -> &[CanvasElement] {
        &self.elements
    }

    /// Get a specific element by ID
    pub fn get_element(&self, element_id: Uuid) -> Option<&CanvasElement> {
        self.elements.iter().find(|e| e.id() == element_id)
    }

    /// Get a mutable reference to an element by ID
    pub fn get_element_mut(&mut self, element_id: Uuid) -> Option<&mut CanvasElement> {
        self.elements.iter_mut().find(|e| e.id() == element_id)
    }

    /// Subscribe to canvas updates
    pub fn subscribe(&self) -> broadcast::Receiver<CanvasUpdate> {
        self.tx.subscribe()
    }

    /// Get the number of connected subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Set a state value
    pub fn set_state(&mut self, key: impl Into<String>, value: Value) {
        self.state.insert(key.into(), value);
    }

    /// Get a state value
    pub fn get_state(&self, key: &str) -> Option<&Value> {
        self.state.get(key)
    }

    /// Get all state
    pub fn state(&self) -> &HashMap<String, Value> {
        &self.state
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if canvas has no elements
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Find elements by predicate
    pub fn find_elements<F>(&self, predicate: F) -> Vec<&CanvasElement>
    where
        F: Fn(&CanvasElement) -> bool,
    {
        self.elements.iter().filter(|e| predicate(e)).collect()
    }

    /// Replace all elements with new ones
    pub fn replace(&mut self, elements: Vec<CanvasElement>) {
        debug!(canvas_id = %self.id, count = elements.len(), "Replacing all canvas elements");

        self.elements = elements.clone();

        let update = CanvasUpdate {
            canvas_id: self.id,
            action: CanvasAction::Push { elements },
        };

        let _ = self.tx.send(update);
    }
}

impl Clone for Canvas {
    fn clone(&self) -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);
        Self {
            id: self.id,
            title: self.title.clone(),
            elements: self.elements.clone(),
            state: self.state.clone(),
            tx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::TextStyle;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new("Test Canvas");
        assert_eq!(canvas.title(), "Test Canvas");
        assert!(canvas.is_empty());
    }

    #[test]
    fn test_canvas_with_id() {
        let id = Uuid::new_v4();
        let canvas = Canvas::with_id(id, "Named Canvas");
        assert_eq!(canvas.id(), id);
    }

    #[test]
    fn test_push_elements() {
        let mut canvas = Canvas::new("Test");
        let element = CanvasElement::Text {
            id: Uuid::new_v4(),
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        canvas.add(element);
        assert_eq!(canvas.len(), 1);
    }

    #[test]
    fn test_reset_canvas() {
        let mut canvas = Canvas::new("Test");
        let element = CanvasElement::Text {
            id: Uuid::new_v4(),
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        canvas.add(element);
        canvas.reset();
        assert!(canvas.is_empty());
    }

    #[test]
    fn test_subscribe_receives_updates() {
        let mut canvas = Canvas::new("Test");
        let mut rx = canvas.subscribe();

        let element = CanvasElement::Text {
            id: Uuid::new_v4(),
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        canvas.add(element);

        // Should receive the update
        let update = rx.try_recv();
        assert!(update.is_ok());
    }

    #[tokio::test]
    async fn test_update_element() {
        let mut canvas = Canvas::new("Test");
        let id = Uuid::new_v4();
        let element = CanvasElement::Text {
            id,
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        canvas.add(element);

        let updated = CanvasElement::Text {
            id,
            content: "Updated".to_string(),
            style: TextStyle::new(),
        };

        assert!(canvas.update(id, updated).is_ok());
    }

    #[tokio::test]
    async fn test_remove_element() {
        let mut canvas = Canvas::new("Test");
        let id = Uuid::new_v4();
        let element = CanvasElement::Text {
            id,
            content: "Hello".to_string(),
            style: TextStyle::new(),
        };

        canvas.add(element);
        assert_eq!(canvas.len(), 1);

        assert!(canvas.remove(id).is_ok());
        assert!(canvas.is_empty());
    }

    #[test]
    fn test_state_management() {
        let mut canvas = Canvas::new("Test");
        canvas.set_state("key", Value::String("value".to_string()));

        assert_eq!(
            canvas.get_state("key"),
            Some(&Value::String("value".to_string()))
        );
    }
}
