#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local};
use reqwest::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::models::google_account::GoogleAccount;

const GOOGLE_DEVICE_CODE_ENDPOINT: &str = "https://oauth2.googleapis.com/device/code";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
const GOOGLE_CALENDAR_SCOPE: &str = "openid email https://www.googleapis.com/auth/calendar";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: Option<String>,
    verification_uri: Option<String>,
    verification_url_complete: Option<String>,
    verification_uri_complete: Option<String>,
    expires_in: i64,
    interval: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenSuccessResponse {
    access_token: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    email: Option<String>,
}

pub struct GoogleAccountService<'a> {
    conn: &'a Connection,
    client: Client,
}

impl<'a> GoogleAccountService<'a> {
    pub fn new(conn: &'a Connection) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .context("Failed to build Google auth HTTP client")?;

        Ok(Self { conn, client })
    }

    pub fn load(&self) -> Result<GoogleAccount> {
        self.conn
            .query_row(
                "SELECT id, oauth_client_id, account_email, access_token, refresh_token,
                        token_type, scope, expires_at, last_error, connected_at, updated_at
                 FROM google_account
                 WHERE id = 1",
                [],
                |row| {
                    Ok(GoogleAccount {
                        id: row.get(0)?,
                        oauth_client_id: row.get(1)?,
                        account_email: row.get(2)?,
                        access_token: row.get(3)?,
                        refresh_token: row.get(4)?,
                        token_type: row.get(5)?,
                        scope: row.get(6)?,
                        expires_at: row.get(7)?,
                        last_error: row.get(8)?,
                        connected_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                },
            )
            .optional()
            .map(|maybe| {
                maybe.unwrap_or(GoogleAccount {
                    id: 1,
                    ..GoogleAccount::default()
                })
            })
            .context("Failed to load Google account state")
    }

    pub fn set_client_id(&self, client_id: &str) -> Result<()> {
        let trimmed = client_id.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Google OAuth client ID cannot be empty"));
        }

        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO google_account (id, oauth_client_id, updated_at)
                 VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET
                   oauth_client_id = excluded.oauth_client_id,
                   updated_at = excluded.updated_at",
                params![trimmed, now],
            )
            .context("Failed to save Google OAuth client ID")?;

        Ok(())
    }

    pub fn disconnect(&self) -> Result<()> {
        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO google_account (id, updated_at)
                 VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET
                   account_email = NULL,
                   access_token = NULL,
                   refresh_token = NULL,
                   token_type = NULL,
                   scope = NULL,
                   expires_at = NULL,
                   connected_at = NULL,
                   last_error = NULL,
                   updated_at = excluded.updated_at",
                params![now],
            )
            .context("Failed to disconnect Google account")?;

        Ok(())
    }

    pub fn refresh_access_token(&self) -> Result<GoogleAccount> {
        let state = self.load()?;
        let client_id = state
            .oauth_client_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow!("OAuth client ID is required before refreshing token"))?
            .to_string();

        let refresh_token = state
            .refresh_token
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow!("No refresh token available; reconnect Google account"))?
            .to_string();

        let response = self
            .client
            .post(GOOGLE_TOKEN_ENDPOINT)
            .form(&[
                ("client_id", client_id.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
            ])
            .send()
            .context("Failed to call Google token refresh endpoint")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let error = Self::decode_token_error(&body).unwrap_or_else(|| body.trim().to_string());
            self.record_last_error(&format!("Token refresh failed ({status}): {error}"))?;
            if error.contains("invalid_grant") {
                // Token revoked or invalid: keep client_id, clear auth material.
                self.disconnect()?;
            }
            return Err(anyhow!("Token refresh failed: {}", error));
        }

        let token = response
            .json::<TokenSuccessResponse>()
            .context("Failed to parse Google token refresh response")?;

        self.persist_token_update(
            state.account_email.as_deref(),
            &token.access_token,
            token
                .refresh_token
                .as_deref()
                .or(Some(refresh_token.as_str())),
            token.token_type.as_deref(),
            token.scope.as_deref(),
            token.expires_in,
        )
        .context("Failed to persist refreshed token")?;

        self.load()
    }

    pub fn connect_with_device_flow(&self, client_id: &str) -> Result<GoogleAccount> {
        let client_id = client_id.trim();
        if client_id.is_empty() {
            return Err(anyhow!("Google OAuth client ID cannot be empty"));
        }

        self.set_client_id(client_id)?;

        let device = self.request_device_code(client_id)?;
        let verification_link = device
            .verification_uri_complete
            .as_deref()
            .or(device.verification_url_complete.as_deref())
            .or(device.verification_url.as_deref())
            .or(device.verification_uri.as_deref())
            .map(str::to_string)
            .ok_or_else(|| anyhow!("Google did not return a verification URL"))?;

        if let Err(err) = webbrowser::open(&verification_link) {
            log::warn!(
                "Failed to open browser for Google device auth link: {}. User can open manually.",
                err
            );
        }

        log::info!(
            "Google device auth started. Verification URL opened; user code: {}",
            device.user_code
        );

        let token = self.poll_for_device_token(client_id, &device)?;
        let email = self.fetch_account_email(&token.access_token)?;

        self.persist_token_update(
            Some(email.as_str()),
            &token.access_token,
            token.refresh_token.as_deref(),
            token.token_type.as_deref(),
            token.scope.as_deref(),
            token.expires_in,
        )
        .context("Failed to persist connected Google account")?;

        self.load()
    }

    fn request_device_code(&self, client_id: &str) -> Result<DeviceCodeResponse> {
        let response = self
            .client
            .post(GOOGLE_DEVICE_CODE_ENDPOINT)
            .form(&[("client_id", client_id), ("scope", GOOGLE_CALENDAR_SCOPE)])
            .send()
            .context("Failed to request Google device code")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Google device code request failed ({status}): {}",
                body.trim()
            ));
        }

        response
            .json::<DeviceCodeResponse>()
            .context("Failed to parse Google device code response")
    }

    fn poll_for_device_token(
        &self,
        client_id: &str,
        device: &DeviceCodeResponse,
    ) -> Result<TokenSuccessResponse> {
        let start = Local::now();
        let interval = device.interval.unwrap_or(5).max(1);

        loop {
            if Local::now() >= start + Duration::seconds(device.expires_in.max(1)) {
                return Err(anyhow!("Google device authorization timed out"));
            }

            std::thread::sleep(std::time::Duration::from_secs(interval as u64));

            let response = self
                .client
                .post(GOOGLE_TOKEN_ENDPOINT)
                .form(&[
                    ("client_id", client_id),
                    ("device_code", device.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .context("Failed to poll Google token endpoint")?;

            if response.status().is_success() {
                return response
                    .json::<TokenSuccessResponse>()
                    .context("Failed to parse Google token response");
            }

            let body = response.text().unwrap_or_default();
            let err = Self::decode_token_error(&body).unwrap_or_else(|| body.trim().to_string());
            match err.as_str() {
                "authorization_pending" => continue,
                "slow_down" => {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    continue;
                }
                "access_denied" => return Err(anyhow!("Google authorization was denied")),
                "expired_token" => {
                    return Err(anyhow!("Google device authorization expired"));
                }
                _ => return Err(anyhow!("Google token polling failed: {}", err)),
            }
        }
    }

    fn fetch_account_email(&self, access_token: &str) -> Result<String> {
        let response = self
            .client
            .get(GOOGLE_USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .context("Failed to fetch Google account user info")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Google user info request failed ({status}): {}",
                body.trim()
            ));
        }

        let info = response
            .json::<UserInfoResponse>()
            .context("Failed to parse Google user info response")?;

        info.email
            .map(|email| email.trim().to_string())
            .filter(|email| !email.is_empty())
            .ok_or_else(|| anyhow!("Google user info response did not include account email"))
    }

    fn persist_token_update(
        &self,
        account_email: Option<&str>,
        access_token: &str,
        refresh_token: Option<&str>,
        token_type: Option<&str>,
        scope: Option<&str>,
        expires_in: Option<i64>,
    ) -> Result<()> {
        let now = Local::now();
        let expires_at =
            expires_in.map(|seconds| (now + Duration::seconds(seconds.max(1))).to_rfc3339());

        self.conn
            .execute(
                "INSERT INTO google_account (
                    id, account_email, access_token, refresh_token,
                    token_type, scope, expires_at, connected_at, updated_at, last_error
                 )
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
                 ON CONFLICT(id) DO UPDATE SET
                   account_email = excluded.account_email,
                   access_token = excluded.access_token,
                   refresh_token = COALESCE(excluded.refresh_token, google_account.refresh_token),
                   token_type = excluded.token_type,
                   scope = excluded.scope,
                   expires_at = excluded.expires_at,
                   connected_at = COALESCE(google_account.connected_at, excluded.connected_at),
                   updated_at = excluded.updated_at,
                   last_error = NULL",
                params![
                    account_email,
                    access_token,
                    refresh_token,
                    token_type,
                    scope,
                    expires_at,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )
            .context("Failed to persist Google account token update")?;

        Ok(())
    }

    fn record_last_error(&self, message: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO google_account (id, last_error, updated_at)
                 VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET
                   last_error = excluded.last_error,
                   updated_at = excluded.updated_at",
                params![message, Local::now().to_rfc3339()],
            )
            .context("Failed to store Google account error")?;
        Ok(())
    }

    fn decode_token_error(body: &str) -> Option<String> {
        serde_json::from_str::<TokenErrorResponse>(body)
            .ok()
            .map(|v| {
                if let Some(desc) = v.error_description {
                    format!("{} ({})", v.error, desc)
                } else {
                    v.error
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::GoogleAccountService;
    use crate::services::database::Database;

    #[test]
    fn saves_client_id_and_disconnects() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        let service = GoogleAccountService::new(db.connection()).unwrap();
        service
            .set_client_id("client-id.apps.googleusercontent.com")
            .unwrap();

        let loaded = service.load().unwrap();
        assert_eq!(
            loaded.oauth_client_id.as_deref(),
            Some("client-id.apps.googleusercontent.com")
        );

        service.disconnect().unwrap();
        let disconnected = service.load().unwrap();
        assert!(disconnected.account_email.is_none());
        assert!(disconnected.access_token.is_none());
    }
}
