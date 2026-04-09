use crate::db::{self, DbPool};
use lettre::{
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub async fn send_report_email(
    pool: &DbPool,
    subject: &str,
    html_body: &str,
    attachment_path: Option<&str>,
) -> Result<(), String> {
    let smtp_server = db::get_setting(pool, "smtp_server").await;
    let smtp_port = db::get_setting(pool, "smtp_port").await;
    let smtp_username = db::get_setting(pool, "smtp_username").await;
    let smtp_password = db::get_setting(pool, "smtp_password").await;
    let from = db::get_setting(pool, "email_from").await;
    let to = db::get_setting(pool, "email_to").await;

    if smtp_server.is_empty() || from.is_empty() || to.is_empty() {
        return Err("Email settings not configured".into());
    }

    let port: u16 = smtp_port.parse().unwrap_or(587);

    let email_builder = Message::builder()
        .from(from.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .to(to.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject(subject);

    let body = if let Some(path) = attachment_path {
        let file_content = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let filename = std::path::Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let attachment = Attachment::new(filename)
            .body(file_content, ContentType::parse("application/octet-stream").unwrap());

        MultiPart::mixed()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            )
            .singlepart(attachment)
    } else {
        MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            )
    };

    let email = email_builder
        .multipart(body)
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(smtp_username, smtp_password);

    let mailer = if port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)
            .map_err(|e| e.to_string())?
            .credentials(creds)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_server)
            .map_err(|e| e.to_string())?
            .credentials(creds)
            .build()
    };

    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}
