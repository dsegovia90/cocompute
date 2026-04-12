pub mod templates;

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    pub fn new(
        host: &str,
        port: u16,
        user: Option<&str>,
        password: Option<&str>,
        from: &str,
    ) -> anyhow::Result<Self> {
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?
            .port(port);

        if let (Some(user), Some(pass)) = (user, password) {
            builder = builder.credentials(Credentials::new(user.to_string(), pass.to_string()));
        }

        let transport = builder.build();
        let from: Mailbox = from.parse().map_err(|e| anyhow::anyhow!("invalid from address: {e}"))?;

        Ok(Self { transport, from })
    }

    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let to_mailbox: Mailbox = to.parse().map_err(|e| anyhow::anyhow!("invalid to address: {e}"))?;

        let message = Message::builder()
            .from(self.from.clone())
            .to(to_mailbox)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_PLAIN)
                            .body(text.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .content_type(ContentType::TEXT_HTML)
                            .body(html.to_string()),
                    ),
            )?;

        self.transport.send(message).await?;
        Ok(())
    }
}
