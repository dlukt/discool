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
    let verification_link = build_verification_link(&config.verification_url_base, token);
    let body = format!(
        "Verify your recovery email for Discool by opening this link:\n\n{verification_link}\n\nThis link expires soon and can only be used once."
    );

    let message = Message::builder()
        .from(from)
        .to(to)
        .subject("Verify your Discool recovery email")
        .body(body)
        .map_err(|_| AppError::Internal("Failed to build verification email".to_string()))?;

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
        .map_err(|_| AppError::Internal("Failed to send verification email".to_string()))?;

    Ok(())
}

fn build_verification_link(base: &str, token: &str) -> String {
    let base = base.trim();
    let separator = if base.contains('?') { '&' } else { '?' };
    format!("{base}{separator}token={token}")
}

#[cfg(test)]
mod tests {
    use super::build_verification_link;

    #[test]
    fn builds_verification_link_with_query_separator() {
        assert_eq!(
            build_verification_link(
                "https://example.com/api/v1/auth/recovery-email/verify",
                "abc"
            ),
            "https://example.com/api/v1/auth/recovery-email/verify?token=abc"
        );
        assert_eq!(
            build_verification_link(
                "https://example.com/api/v1/auth/recovery-email/verify?source=email",
                "abc"
            ),
            "https://example.com/api/v1/auth/recovery-email/verify?source=email&token=abc"
        );
    }
}
