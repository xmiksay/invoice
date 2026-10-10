//! Account e-mails (verification, "already registered", password reset):
//! plain text in cs / en, sent through the instance SMTP in a spawned task.
//! A send failure is logged and never changes the response (no account
//! enumeration; the user can ask again).

use crate::app::AppState;
use crate::email::input::parse_address;
use crate::email::sender::Outgoing;
use crate::pdf::format::Locale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mail {
    /// `{baseUrl}/verify?token=…`
    Verify(String),
    /// Registration with a known e-mail: `{baseUrl}/reset?token=…`.
    AlreadyRegistered(String),
    /// `{baseUrl}/reset?token=…`
    Reset(String),
}

/// `cs` unless the request asked for `en`.
pub fn locale(raw: Option<&str>) -> Locale {
    raw.and_then(|l| Locale::parse(l.trim()))
        .unwrap_or(Locale::Cs)
}

/// (subject, body).
pub fn render(mail: &Mail, locale: Locale) -> (&'static str, String) {
    match (mail, locale) {
        (Mail::Verify(url), Locale::Cs) => (
            "Potvrďte svůj e-mail",
            format!(
                "Dobrý den,\n\npro dokončení registrace v aplikaci Invoice potvrďte svůj e-mail:\n\n{url}\n\n\
                 Odkaz platí 24 hodin. Pokud jste se neregistrovali, tento e-mail ignorujte.\n"
            ),
        ),
        (Mail::Verify(url), Locale::En) => (
            "Confirm your e-mail",
            format!(
                "Hello,\n\nto finish your Invoice registration, confirm your e-mail address:\n\n{url}\n\n\
                 The link is valid for 24 hours. If you did not register, ignore this e-mail.\n"
            ),
        ),
        (Mail::AlreadyRegistered(url), Locale::Cs) => (
            "Účet už existuje",
            format!(
                "Dobrý den,\n\nněkdo se pokusil zaregistrovat s touto adresou, ale účet s ní už máte.\n\
                 Pokud jste zapomněli heslo, nastavte si nové:\n\n{url}\n\n\
                 Odkaz platí 1 hodinu. Pokud jste o nic nežádali, tento e-mail ignorujte.\n"
            ),
        ),
        (Mail::AlreadyRegistered(url), Locale::En) => (
            "You already have an account",
            format!(
                "Hello,\n\nsomeone tried to register with this address, but you already have an account.\n\
                 If you forgot your password, set a new one:\n\n{url}\n\n\
                 The link is valid for 1 hour. If you did not ask for anything, ignore this e-mail.\n"
            ),
        ),
        (Mail::Reset(url), Locale::Cs) => (
            "Obnovení hesla",
            format!(
                "Dobrý den,\n\nnové heslo k aplikaci Invoice si nastavíte zde:\n\n{url}\n\n\
                 Odkaz platí 1 hodinu. Pokud jste o obnovení nežádali, tento e-mail ignorujte.\n"
            ),
        ),
        (Mail::Reset(url), Locale::En) => (
            "Password reset",
            format!(
                "Hello,\n\nset a new Invoice password here:\n\n{url}\n\n\
                 The link is valid for 1 hour. If you did not ask for a reset, ignore this e-mail.\n"
            ),
        ),
    }
}

/// Send `mail` to `to` in the background: the request answers at once in
/// every branch (whether an account exists must not show in the timing);
/// failures (no SMTP, refused) are logged only.
pub fn send(state: &AppState, to: &str, mail: Mail, locale: Locale) {
    let Some(mailer) = state.email.clone() else {
        tracing::error!("account e-mail not sent: SMTP is not configured");
        return;
    };
    let Some(to) = parse_address(to) else {
        tracing::warn!("account e-mail not sent: the stored address does not parse");
        return;
    };
    let (subject, body) = render(&mail, locale);
    let msg = Outgoing {
        reply_to: None,
        to: vec![to],
        cc: vec![],
        bcc: vec![],
        subject: subject.into(),
        body,
        files: vec![],
    };
    tokio::spawn(async move {
        if let Err(e) = mailer.send(msg, &mailer.message_id()).await {
            tracing::error!(error = %e, "account e-mail not sent");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_mail_in_both_locales() {
        let url = "https://invoiceapp.cz/verify?token=abc".to_string();
        for mail in [
            Mail::Verify(url.clone()),
            Mail::AlreadyRegistered(url.clone()),
            Mail::Reset(url.clone()),
        ] {
            for l in [Locale::Cs, Locale::En] {
                let (subject, body) = render(&mail, l);
                assert!(!subject.is_empty());
                assert!(body.contains(&url), "{body}");
            }
        }
        assert_eq!(locale(Some("en")), Locale::En);
        assert_eq!(locale(Some("de")), Locale::Cs);
        assert_eq!(locale(None), Locale::Cs);
    }
}
