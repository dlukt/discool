use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};

use crate::{AppError, config::EmailConfig};

pub async fn send_recovery_verification_email(
    config: &EmailConfig,
    target_email: &str,
    token: &str,
) -> Result<(), AppError> {
    let verification_link = build_token_link(&config.verification_url_base, "token", token);
    let body = format!(
        "Verify your recovery email for Discool by opening this link:\n\n{verification_link}\n\nThis link expires soon and can only be used once."
    );
    send_email(
        config,
        target_email,
        "Verify your Discool recovery email",
        &body,
        "Failed to build verification email",
        "Failed to send verification email",
    )
    .await
}

pub async fn send_identity_recovery_email(
    config: &EmailConfig,
    target_email: &str,
    token: &str,
) -> Result<(), AppError> {
    let recovery_link = build_token_link(&config.recovery_url_base, "recovery_token", token);
    let body = format!(
        "Recover your Discool identity by opening this link:\n\n{recovery_link}\n\nThis link expires soon and can only be used once."
    );
    send_email(
        config,
        target_email,
        "Recover your Discool identity",
        &body,
        "Failed to build recovery email",
        "Failed to send recovery email",
    )
    .await
}

async fn send_email(
    config: &EmailConfig,
    target_email: &str,
    subject: &str,
    body: &str,
    build_error_message: &str,
    send_error_message: &str,
) -> Result<(), AppError> {
    if config.smtp_host.eq_ignore_ascii_case("stub") {
        return Ok(());
    }

    let from = Mailbox::new(
        Some(config.from_name.clone()),
        config
            .from_address
            .parse()
            .map_err(|_| AppError::Internal("Invalid email sender configuration".to_string()))?,
    );
    let to = Mailbox::new(
        None,
        target_email
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid email address".to_string()))?,
    );

    let message = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .body(body.to_string())
        .map_err(|_| AppError::Internal(build_error_message.to_string()))?;

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
        .port(config.smtp_port);
    if let (Some(username), Some(password)) =
        (config.smtp_username.as_ref(), config.smtp_password.as_ref())
    {
        builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
    }
    let sender = builder.build();

    sender
        .send(message)
        .await
        .map_err(|_| AppError::Internal(send_error_message.to_string()))?;

    Ok(())
}

fn build_token_link(base: &str, token_param: &str, token: &str) -> String {
    let base = base.trim();
    let separator = if base.contains('?') { '&' } else { '?' };
    let encoded_token = encode_query_value(token);
    format!("{base}{separator}{token_param}={encoded_token}")
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let is_unreserved = matches!(
            byte,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if is_unreserved {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::build_token_link;

    #[test]
    fn builds_token_links_with_query_separator() {
        assert_eq!(
            build_token_link(
                "https://example.com/api/v1/auth/recovery-email/verify",
                "token",
                "abc"
            ),
            "https://example.com/api/v1/auth/recovery-email/verify?token=abc"
        );
        assert_eq!(
            build_token_link(
                "https://example.com/api/v1/auth/recovery-email/verify?source=email",
                "token",
                "abc"
            ),
            "https://example.com/api/v1/auth/recovery-email/verify?source=email&token=abc"
        );
        assert_eq!(
            build_token_link("https://example.com/", "recovery_token", "abc"),
            "https://example.com/?recovery_token=abc"
        );
    }

    #[test]
    fn encodes_token_query_values() {
        assert_eq!(
            build_token_link("https://example.com/recover", "token", "a b&c=d%"),
            "https://example.com/recover?token=a%20b%26c%3Dd%25"
        );
    }
}
