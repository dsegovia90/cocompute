pub struct EmailParts {
    pub subject: String,
    pub html: String,
    pub text: String,
}

pub fn waitlist_email(email: &str) -> EmailParts {
    EmailParts {
        subject: "You're on the cocompute beta list".to_string(),
        html: format!(
            r#"<div style="font-family:sans-serif;max-width:480px;margin:0 auto;padding:32px">
<h2 style="color:#fff;margin:0 0 16px">You're on the list!</h2>
<p style="color:#a1a1aa;line-height:1.6">
Thanks for signing up for the cocompute beta ({email}). We'll reach out when a spot opens up.
</p>
<p style="color:#71717a;font-size:13px;margin-top:24px">— the cocompute team</p>
</div>"#
        ),
        text: format!(
            "You're on the list!\n\nThanks for signing up for the cocompute beta ({email}). We'll reach out when a spot opens up.\n\n— the cocompute team"
        ),
    }
}

pub fn invite_email(name: &str, token: &str, base_url: &str) -> EmailParts {
    let verify_url = format!("{base_url}/verify?token={token}");
    EmailParts {
        subject: "You're invited to cocompute".to_string(),
        html: format!(
            r#"<div style="font-family:sans-serif;max-width:480px;margin:0 auto;padding:32px">
<h2 style="color:#fff;margin:0 0 16px">Welcome, {name}!</h2>
<p style="color:#a1a1aa;line-height:1.6">
You've been invited to the cocompute beta. Click below to set your password and get started.
</p>
<a href="{verify_url}" style="display:inline-block;margin:24px 0;padding:12px 24px;background:#6366f1;color:#fff;text-decoration:none;border-radius:8px;font-weight:600">
Set Your Password
</a>
<p style="color:#71717a;font-size:13px">This link expires in 48 hours.</p>
<p style="color:#52525b;font-size:12px;margin-top:16px">If you didn't request this, you can ignore this email.</p>
</div>"#
        ),
        text: format!(
            "Welcome, {name}!\n\nYou've been invited to the cocompute beta. Visit the link below to set your password:\n\n{verify_url}\n\nThis link expires in 48 hours.\n\n— the cocompute team"
        ),
    }
}

pub fn welcome_email(name: &str, api_key: &str) -> EmailParts {
    EmailParts {
        subject: "Welcome to cocompute — here's your API key".to_string(),
        html: format!(
            r#"<div style="font-family:sans-serif;max-width:480px;margin:0 auto;padding:32px">
<h2 style="color:#fff;margin:0 0 16px">You're all set, {name}!</h2>
<p style="color:#a1a1aa;line-height:1.6">
Your cocompute account is verified. Here's your API key:
</p>
<div style="background:#1a1a2e;border:1px solid #27272a;border-radius:8px;padding:16px;margin:16px 0;font-family:monospace;color:#67e8f9;word-break:break-all">
{api_key}
</div>
<p style="color:#71717a;font-size:13px">Keep this key safe — it won't be shown again.</p>
</div>"#
        ),
        text: format!(
            "You're all set, {name}!\n\nYour cocompute account is verified. Here's your API key:\n\n{api_key}\n\nKeep this key safe — it won't be shown again.\n\n— the cocompute team"
        ),
    }
}

pub fn reset_email(name: &str, token: &str, base_url: &str) -> EmailParts {
    let reset_url = format!("{base_url}/reset?token={token}");
    EmailParts {
        subject: "Reset your cocompute password".to_string(),
        html: format!(
            r#"<div style="font-family:sans-serif;max-width:480px;margin:0 auto;padding:32px">
<h2 style="color:#fff;margin:0 0 16px">Password reset</h2>
<p style="color:#a1a1aa;line-height:1.6">
Hi {name}, we received a request to reset your password. Click below to choose a new one.
</p>
<a href="{reset_url}" style="display:inline-block;margin:24px 0;padding:12px 24px;background:#6366f1;color:#fff;text-decoration:none;border-radius:8px;font-weight:600">
Reset Password
</a>
<p style="color:#71717a;font-size:13px">This link expires in 1 hour.</p>
<p style="color:#52525b;font-size:12px;margin-top:16px">If you didn't request this, you can ignore this email.</p>
</div>"#
        ),
        text: format!(
            "Hi {name},\n\nWe received a request to reset your password. Visit the link below:\n\n{reset_url}\n\nThis link expires in 1 hour.\n\nIf you didn't request this, you can ignore this email.\n\n— the cocompute team"
        ),
    }
}
