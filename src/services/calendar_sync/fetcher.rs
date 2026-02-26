#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::thread;
use std::time::Duration;

pub struct IcsFetcher {
    client: Client,
    max_response_bytes: usize,
    max_retries: usize,
    retry_delay_ms: u64,
}

impl IcsFetcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("Failed to build ICS fetch HTTP client")?;

        Ok(Self {
            client,
            max_response_bytes: 5 * 1024 * 1024,
            max_retries: 2,
            retry_delay_ms: 400,
        })
    }

    pub fn fetch_ics(&self, url: &str) -> Result<String> {
        if !url.starts_with("https://") {
            return Err(anyhow!("ICS URL must use HTTPS"));
        }

        let redacted = Self::redact_url(url);
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 0..=self.max_retries {
            match self.fetch_once(url) {
                Ok(content) => return Ok(content),
                Err(err) => {
                    let is_last_attempt = attempt == self.max_retries;
                    if is_last_attempt {
                        last_error = Some(err.context(format!(
                            "Failed to fetch ICS from {} after {} attempts",
                            redacted,
                            attempt + 1
                        )));
                    } else {
                        log::warn!(
                            "ICS fetch attempt {} failed for {}: {}",
                            attempt + 1,
                            redacted,
                            err
                        );
                        thread::sleep(Duration::from_millis(self.retry_delay_ms));
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Unknown ICS fetch error")))
    }

    fn fetch_once(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .context("Network error during ICS fetch")?;

        let status = response.status();
        if status != StatusCode::OK {
            return Err(anyhow!("ICS fetch failed with HTTP status {}", status));
        }

        if let Some(content_length) = response.content_length() {
            if content_length as usize > self.max_response_bytes {
                return Err(anyhow!(
                    "ICS response too large ({} bytes > {} bytes)",
                    content_length,
                    self.max_response_bytes
                ));
            }
        }

        let bytes = response
            .bytes()
            .context("Failed to read ICS response body")?;

        if bytes.len() > self.max_response_bytes {
            return Err(anyhow!(
                "ICS response too large ({} bytes > {} bytes)",
                bytes.len(),
                self.max_response_bytes
            ));
        }

        let content = String::from_utf8(bytes.to_vec()).context("ICS response is not valid UTF-8")?;

        if !(content.contains("BEGIN:VCALENDAR") || content.contains("BEGIN:VEVENT")) {
            return Err(anyhow!("Response does not appear to be valid ICS content"));
        }

        Ok(content)
    }

    fn redact_url(url: &str) -> String {
        if let Some(index) = url.find("/calendar/ical/") {
            let prefix_end = index + "/calendar/ical/".len();
            let prefix = &url[..prefix_end];
            return format!("{}***redacted***", prefix);
        }

        "***redacted-url***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::IcsFetcher;

    #[test]
    fn test_redact_url_google_ics() {
        let redacted = IcsFetcher::redact_url(
            "https://calendar.google.com/calendar/ical/debp200517%40gmail.com/private-token/basic.ics",
        );
        assert_eq!(
            redacted,
            "https://calendar.google.com/calendar/ical/***redacted***"
        );
    }

    #[test]
    fn test_redact_url_fallback() {
        let redacted = IcsFetcher::redact_url("https://example.com/calendar.ics");
        assert_eq!(redacted, "***redacted-url***");
    }
}
