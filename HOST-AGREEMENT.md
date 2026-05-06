# Host Operator Agreement

**Effective date:** 2026-05-05

This Host Operator Agreement ("Host Agreement") governs your operation of a host on the cocompute.ai network. By installing and running `cocompute_host` and registering it against cocompute.ai, you agree to these terms.

This agreement is in addition to the [Terms of Service](./TERMS.md) and the [Acceptable Use Policy](./AUP.md), both of which also apply to your use of the Service.

If you self-host your own orchestrator, this agreement does not apply to your private network. It applies only when your host registers with cocompute.ai.

## 1. What you are

You are providing computational infrastructure. You are not the publisher, author, or speaker of the prompts that arrive at your host or the responses your host generates. Your role is analogous to a hosting provider, an internet service provider, or a CDN.

We treat you that way for purposes of this agreement and our internal policies. We expect this analogy to hold legally in most jurisdictions, but we are not your lawyer; if you have specific legal concerns about your jurisdiction, consult one.

## 2. What you control

- You decide which machine to run `cocompute_host` on.
- You decide which models to install in your local Ollama instance. cocompute will only serve what Ollama on your machine exposes.
- You decide which pools your host belongs to. Hosts default to no pool; you must explicitly add yours via the dashboard.
- You can deactivate or delete your host at any time. Deactivation takes effect within seconds and the host stops receiving requests immediately.
- You can stop the underlying systemd or launchd service at any time. The host disconnects from the orchestrator and stops serving traffic.

## 3. What we collect from your host

The orchestrator collects the following data from your registered host:

- Capability metadata: which models are available, model size, context window, basic system info (OS, architecture)
- Connection metadata: connect time, last-seen timestamp, iroh endpoint id (which rotates each restart)
- Per-request metadata: timestamp, model, token counts, request duration, status (success or error type)

We do **not** collect or store the contents of prompts or responses on the orchestrator side beyond the brief moment needed to relay them between consumer and host. The orchestrator does not log prompt or response bodies.

## 4. What you may collect from prompts that pass through your host

Your host's local Ollama instance processes prompts and generates responses on your hardware. You have technical access to this content. You agree to:

- **Not** log, store, or share prompt or response content for any purpose other than local debugging unless you disclose your logging policy in writing to the operator (operations@cocompute.ai) and obtain approval. We may decline approval and remove your host from the network if your logging is incompatible with user expectations.
- **Not** train models on prompts that pass through your host without explicit consent from the prompt's author. (Asking the consumer is not consent: cocompute users have not opted in to having their prompts used as training data.)
- Treat any prompt content you incidentally see (during local debugging, error inspection, etc.) as confidential.

If you do not want to handle prompt content under these terms, do not register a host. Self-host your own orchestrator instead and apply your own terms.

## 5. Your liability for content

You are not liable to the operator or to other users for the content of prompts routed through your host or responses your host generates, provided that:

- You comply with this Host Agreement and the AUP
- You do not deliberately introduce a model designed to generate harmful content
- You promptly act on any abuse notice we send you (typically: deactivate the offending pool membership, rotate keys, or take the affected model offline)

The operator agrees to defend and indemnify host operators against third-party claims that arise solely from content routed through the host as part of normal Service operation, subject to the limits in section 12 of the [Terms of Service](./TERMS.md). If you fail to comply with this Host Agreement or the AUP, the indemnity does not apply.

## 6. Network identity and your IP address

When inference traffic flows through your host, the destination of the request (some external API call your model makes) sees your IP address. This is true of any internet-connected service you operate.

cocompute does not currently allow models hosted by you to make outbound HTTP calls (no tool use, no function calling that hits external endpoints), but if we add that capability in the future, your host will be the source of those requests. We will give you advance notice and an opt-out before enabling this.

The Service's setup tokens, peer-to-peer transport (iroh), and rotating endpoint IDs are designed to limit the surface area of your home network. Read [ARCHITECTURE.md](./ARCHITECTURE.md) for details.

## 7. Pool credit accounting

Your host earns pool credits for inference cycles it serves. You consume pool credits when you (or another user with an API key in your pool) make inference requests against other people's hosts. Credits are not money, are not transferable, and have no value outside the Service.

## 8. Hardware and electricity

Running a host uses your electricity, your bandwidth, and shortens the lifespan of your hardware (modestly). You acknowledge these costs and agree that the operator is not responsible for any of them. cocompute provides software that lets you opt into this; the decision is yours.

## 9. Termination by you

You can terminate this agreement at any time:

1. Open the dashboard, go to Hosts, click "Deactivate" on the host. The host stops receiving requests immediately.
2. Stop the systemd or launchd service: `systemctl --user stop cocompute-host` (Linux) or `launchctl bootout gui/$(id -u)/ai.cocompute.host` (macOS).
3. Optionally, uninstall: remove `~/.cocompute/`, the systemd unit at `~/.config/systemd/user/cocompute-host.service` (Linux), and the plist at `~/Library/LaunchAgents/ai.cocompute.host.plist` (macOS).

After deactivation, your host record is preserved for accounting reasons but produces no traffic.

## 10. Termination by us

We may remove your host from the network if:

- You violate this Host Agreement or the AUP
- Your host is consistently failing requests, returning malformed responses, or otherwise degrading user experience
- Legal compulsion requires it

For non-emergency removals, we will notify you at the email on your account and give you at least 7 days to address the issue.

## 11. Changes to this agreement

We may update this Host Agreement as the Service evolves. Material changes will be announced at least 30 days in advance via email. If you do not agree to a change, you may terminate by deactivating your host before the change takes effect.

## 12. Contact

Questions: hosts@cocompute.ai
Abuse reports about content routed through your host: abuse@cocompute.ai
