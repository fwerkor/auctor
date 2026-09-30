use anyhow::{Context, bail};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use sqlx::PgPool;
use std::time::Duration;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SmtpSettings {
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_security: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
}

pub async fn load(db: &PgPool) -> Result<SmtpSettings, sqlx::Error> {
    sqlx::query_as(
        "SELECT smtp_host,smtp_port,smtp_security,smtp_username,smtp_password,smtp_from_email,smtp_from_name
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(db)
    .await
}

pub async fn send_verification_code(
    db: &PgPool,
    to: &str,
    code: &str,
    purpose: &str,
) -> anyhow::Result<()> {
    let settings = load(db).await?;
    let (subject, action) = match purpose {
        "registration" => ("Verify your Auctor account", "complete your registration"),
        "email_change" => ("Verify your new email address", "change your email address"),
        _ => bail!("unsupported verification purpose"),
    };
    let body = format!(
        "Your verification code is: {code}\n\nUse this code to {action}. It expires in 10 minutes.\n\nIf you did not request this, you can ignore this message."
    );
    send_text(&settings, to, subject, &body).await
}

pub async fn send_test(db: &PgPool, to: &str) -> anyhow::Result<()> {
    let settings = load(db).await?;
    send_text(
        &settings,
        to,
        "Auctor SMTP test",
        "This message confirms that Auctor can send email using the configured SMTP server.",
    )
    .await
}

async fn send_text(
    settings: &SmtpSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> anyhow::Result<()> {
    if settings.smtp_host.trim().is_empty()
        || settings.smtp_from_email.trim().is_empty()
        || settings.smtp_port <= 0
        || settings.smtp_port > u16::MAX as i32
    {
        bail!("SMTP is not fully configured");
    }

    let from_address = settings
        .smtp_from_email
        .parse()
        .context("invalid SMTP from address")?;
    let to_address = to.parse().context("invalid recipient address")?;
    let from = Mailbox::new(
        (!settings.smtp_from_name.trim().is_empty()).then(|| settings.smtp_from_name.clone()),
        from_address,
    );
    let message = Message::builder()
        .from(from)
        .to(Mailbox::new(None, to_address))
        .subject(subject)
        .body(body.to_owned())
        .context("build email")?;

    let host = settings.smtp_host.trim();
    let mut builder = match settings.smtp_security.as_str() {
        "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .context("configure STARTTLS SMTP")?,
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(host).context("configure TLS SMTP")?,
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
        other => bail!("unsupported SMTP security mode: {other}"),
    }
    .port(settings.smtp_port as u16)
    .timeout(Some(Duration::from_secs(20)));

    if !settings.smtp_username.is_empty() {
        if settings.smtp_password.is_empty() {
            bail!("SMTP password is missing");
        }
        builder = builder.credentials(Credentials::new(
            settings.smtp_username.clone(),
            settings.smtp_password.clone(),
        ));
    }

    builder
        .build()
        .send(message)
        .await
        .context("send SMTP message")?;
    Ok(())
}
