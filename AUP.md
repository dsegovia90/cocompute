# Acceptable Use Policy

**Effective date:** 2026-05-05

This Acceptable Use Policy ("AUP") governs your use of cocompute.ai (the "Service"). It is incorporated by reference into the [Terms of Service](./TERMS.md). Violations may result in immediate suspension or termination of your account, revocation of your API keys, removal of your hosts from the network, or all of the above.

## 1. What you must not do

You agree not to use the Service to generate, store, transmit, or distribute any of the following:

### 1.1 Illegal content or activity

- Any material that is illegal in the United States, Canada or in your jurisdiction
- Child sexual abuse material (CSAM) or any sexualized content depicting minors
- Content that infringes intellectual property rights you do not hold
- Material that violates export control laws or sanctions
- Content used to plan, facilitate, or commit a crime

### 1.2 Harm to people

- Threats, harassment, stalking, or doxxing
- Content designed to incite violence against any person or group
- Non-consensual intimate imagery (real or generated)
- Content that impersonates a real person in a way intended to deceive or harm

### 1.3 Harm to systems

- Malware generation (viruses, ransomware, exploit code, attack scripts)
- Phishing kits, credential stuffing tools, or content designed to defraud
- Content used to evade content moderation, abuse detection, or security controls on third-party services
- Attempts to attack, overload, or degrade the Service or any host on it

### 1.4 Spam, scams, manipulation

- Bulk unsolicited messaging
- Content designed to deceive (financial scams, fake reviews, mass-generated misinformation)
- Coordinated inauthentic behavior (sockpuppet accounts, fake engagement)
- SEO spam or link manipulation

### 1.5 Sensitive use cases without proper safeguards

cocompute is general-purpose infrastructure. We are not built for and do not warrant suitability for:

- Medical diagnosis or treatment recommendations
- Legal advice that will be relied upon
- Decisions affecting employment, housing, credit, insurance, education, or government benefits

If you build something in these categories on top of cocompute, you accept the obligation to ensure it complies with all applicable laws (HIPAA, GDPR, ECOA, etc.) and to not present cocompute output as authoritative.

## 2. Account and API key hygiene

- Do not share your API key publicly. API keys are bearer tokens; whoever holds one can spend your pool credits.
- Do not commit API keys to public repositories. Treat them like passwords.
- Rotate keys you suspect have been exposed. Revoke unused keys.
- Do not create multiple accounts to evade rate limits, abuse policies, or suspension.

## 3. Host operator responsibilities

If you operate a host, see the [Host Operator Agreement](./HOST-AGREEMENT.md). Brief recap:

- You decide which models to expose. cocompute serves what Ollama exposes on your host; if you are not comfortable serving a particular model, do not pull it.
- You can deactivate your host at any time via the dashboard or by stopping the systemd/launchd service.
- You are not liable for the content of prompts and responses routed through your host, provided you are in compliance with this AUP and the Host Operator Agreement.

## 4. Reporting abuse

If you encounter abuse or suspect a violation of this AUP, please report it to abuse@cocompute.ai. Include:

- The pool, host_id, or API key prefix involved (if known)
- Approximate timestamp
- A description of the behavior
- Any supporting evidence

We treat abuse reports confidentially. We do not share reporter identities with the reported party except as required by law.

## 5. Enforcement

We use a graduated enforcement approach where possible:

| Severity | Action |
|----------|--------|
| Minor or first-time issue (likely accidental) | Warning email to the account owner |
| Repeated minor violations | Temporary API key revocation, account suspension up to 30 days |
| Serious violation (any item in section 1) | Immediate API key revocation, account termination, host removal from the network, ban on future signups |
| Illegal content | All of the above plus referral to law enforcement |

We reserve the right to act immediately at any severity level when the situation warrants it (for example, an active attack or content that endangers a person).

## 6. Right to investigate

We may inspect metering records, request logs, and other server-side data to investigate suspected violations of this AUP. We do not routinely inspect prompt or response content; we will only do so if reasonably necessary to investigate a specific reported violation, comply with a legal request, or respond to an emergency.

## 7. Appeals

If your account is suspended or terminated and you believe the decision was in error, email appeals@cocompute.ai within 30 days. Include the affected account email and a description of why you believe the action was incorrect. We will respond within 14 days.

## 8. Updates to this policy

We may update this AUP as the Service evolves and as we encounter abuse patterns we did not anticipate. Material changes will be announced via email and on cocompute.ai. Continued use of the Service after the changes take effect constitutes acceptance.

## 9. Contact

Questions: aup@cocompute.ai
Abuse reports: abuse@cocompute.ai
Appeals: appeals@cocompute.ai
