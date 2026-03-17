//! Slots API for llama.cpp.

use crate::client::LlamaCppClient;
use crate::client::endpoints;
use crate::error::Result;
use crate::types::{SlotInfo, SlotState, SlotsResponse};

/// Client for the slots API.
#[derive(Debug)]
pub struct Slots<'a> {
    client: &'a LlamaCppClient,
}

impl<'a> Slots<'a> {
    /// Create a new slots client.
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self { client }
    }

    /// Get information about all slots.
    pub async fn list(&self) -> Result<SlotsResponse> {
        let response = self.client.get(endpoints::SLOTS).await?;
        let body = self.client.handle_response(response).await?;
        let slots_response: SlotsResponse = serde_json::from_value(body)?;
        Ok(slots_response)
    }

    /// Get the number of total slots.
    pub async fn count(&self) -> Result<usize> {
        let slots = self.list().await?;
        Ok(slots.slots.len())
    }

    /// Get the number of idle slots.
    pub async fn idle_count(&self) -> Result<usize> {
        let slots = self.list().await?;
        Ok(slots.slots.iter().filter(|s| s.state == SlotState::Idle).count())
    }

    /// Get the number of processing slots.
    pub async fn processing_count(&self) -> Result<usize> {
        let slots = self.list().await?;
        Ok(slots.slots.iter().filter(|s| s.state == SlotState::Processing).count())
    }

    /// Get the available (idle) slots.
    pub async fn available(&self) -> Result<Vec<SlotInfo>> {
        let slots = self.list().await?;
        Ok(slots.slots.into_iter().filter(|s| s.state == SlotState::Idle).collect())
    }

    /// Check if there are any available slots.
    pub async fn has_available(&self) -> Result<bool> {
        Ok(self.idle_count().await? > 0)
    }

    /// Wait for an available slot.
    pub async fn wait_for_available(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Option<SlotInfo>> {
        let start = std::time::Instant::now();
        let check_interval = std::time::Duration::from_millis(100);

        loop {
            let available = self.available().await?;
            if let Some(slot) = available.into_iter().next() {
                return Ok(Some(slot));
            }

            if start.elapsed() > timeout {
                return Ok(None);
            }

            tokio::time::sleep(check_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_state() {
        let idle = SlotState::Idle;
        let processing = SlotState::Processing;

        assert_ne!(idle, processing);
    }

    #[test]
    fn test_slot_info() {
        let slot = SlotInfo {
            id: 0,
            state: SlotState::Idle,
            prompt: Some("Hello".to_string()),
            params: None,
        };

        assert_eq!(slot.id, 0);
        assert_eq!(slot.state, SlotState::Idle);
    }
}
