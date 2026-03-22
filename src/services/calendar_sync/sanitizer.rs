pub(crate) const REDACTED_URL: &str = "***redacted-url***";

pub(crate) fn redact_url(url: &str) -> String {
    if let Some(index) = url.find("/calendar/ical/") {
        let prefix_end = index + "/calendar/ical/".len();
        let prefix = &url[..prefix_end];
        return format!("{}***redacted***", prefix);
    }

    REDACTED_URL.to_string()
}

pub(crate) fn sanitize_error_message(message: &str, source_url: &str) -> String {
    let mut sanitized = if source_url.is_empty() {
        message.to_string()
    } else {
        message.replace(source_url, REDACTED_URL)
    };

    sanitized = redact_embedded_urls(&sanitized);
    redact_bearer_tokens(&sanitized)
}

fn redact_embedded_urls(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        if input[i..].starts_with("https://") || input[i..].starts_with("http://") {
            let start = i;
            let mut end = i;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_whitespace() || matches!(c, '"' | '\'' | ')' | ']' | '>') {
                    break;
                }
                end += 1;
            }

            let url = &input[start..end];
            out.push_str(&redact_url(url));
            i = end;
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

fn redact_bearer_tokens(input: &str) -> String {
    let marker = "Bearer ";
    let lower = input.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while let Some(rel_pos) = lower[cursor..].find(&marker_lower) {
        let marker_pos = cursor + rel_pos;
        out.push_str(&input[cursor..marker_pos]);
        out.push_str(marker);

        let token_start = marker_pos + marker.len();
        let mut token_end = token_start;
        let bytes = input.as_bytes();
        while token_end < input.len() {
            let c = bytes[token_end] as char;
            if c.is_whitespace() || matches!(c, ',' | ';' | ')' | ']' | '>') {
                break;
            }
            token_end += 1;
        }

        out.push_str("***redacted***");
        cursor = token_end;
    }

    out.push_str(&input[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::{redact_url, sanitize_error_message, REDACTED_URL};

    #[test]
    fn redact_url_google_ics() {
        let redacted = redact_url(
            "https://calendar.google.com/calendar/ical/debp200517%40gmail.com/private-token/basic.ics",
        );
        assert_eq!(
            redacted,
            "https://calendar.google.com/calendar/ical/***redacted***"
        );
    }

    #[test]
    fn redact_url_fallback() {
        let redacted = redact_url("https://example.com/calendar.ics");
        assert_eq!(redacted, REDACTED_URL);
    }

    #[test]
    fn sanitize_error_message_redacts_any_embedded_url() {
        let msg = "request failed: https://calendar.google.com/calendar/ical/u%40mail/private-token/basic.ics timed out";
        let sanitized = sanitize_error_message(msg, "");

        assert!(!sanitized.contains("private-token"));
        assert!(sanitized.contains("https://calendar.google.com/calendar/ical/***redacted***"));
    }

    #[test]
    fn sanitize_error_message_redacts_bearer_token() {
        let msg = "upstream auth rejected Authorization: Bearer abc123.def456";
        let sanitized = sanitize_error_message(msg, "");

        assert!(!sanitized.contains("abc123.def456"));
        assert!(sanitized.contains("Bearer ***redacted***"));
    }
}
