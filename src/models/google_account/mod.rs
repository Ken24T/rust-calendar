#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAccount {
    pub id: i64,
    pub oauth_client_id: Option<String>,
    pub account_email: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub expires_at: Option<String>,
    pub last_error: Option<String>,
    pub connected_at: Option<String>,
    pub updated_at: Option<String>,
}

impl GoogleAccount {
    pub fn is_connected(&self) -> bool {
        self.account_email
            .as_deref()
            .map(str::trim)
            .is_some_and(|v| !v.is_empty())
            && self
                .refresh_token
                .as_deref()
                .map(str::trim)
                .is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::GoogleAccount;

    #[test]
    fn is_connected_requires_email_and_refresh_token() {
        let disconnected = GoogleAccount::default();
        assert!(!disconnected.is_connected());

        let connected = GoogleAccount {
            id: 1,
            account_email: Some("user@example.com".to_string()),
            refresh_token: Some("refresh-token".to_string()),
            ..GoogleAccount::default()
        };
        assert!(connected.is_connected());
    }
}
