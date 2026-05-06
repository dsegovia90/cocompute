# Terms of Service

**Effective date:** 2026-05-05

These Terms govern your use of cocompute.ai (the "Service"), operated by cocompute (the "Operator"). By signing up, registering a host, or making API calls against the Service, you agree to these Terms.

These Terms cover the cocompute.ai hosted service. The cocompute software is open source under AGPLv3; running your own orchestrator from the source code is not subject to these Terms (it's subject to the AGPL license alone).

## 1. The Service

cocompute.ai provides a coordination layer that lets users share GPU compute and consume LLM inference through a cooperative pool. The Service consists of:

- A web dashboard for account management, host registration, pool creation, and API key management
- An OpenAI-compatible HTTP API at `/v1/`
- A peer-to-peer transport (iroh) connecting registered hosts to the orchestrator

## 2. Accounts

You must be at least 13 years old (or the age of digital consent in your jurisdiction, whichever is higher) to create an account. You are responsible for keeping your account credentials and API keys secure. You are responsible for all activity under your account, including any host you register and any API key you generate.

You agree to provide accurate information at signup and to keep it current.

We may suspend or terminate accounts that violate these Terms or the Acceptable Use Policy.

## 3. Acceptable Use

Your use of the Service is subject to the [Acceptable Use Policy](./AUP.md), which is incorporated into these Terms by reference. AUP violations may result in immediate suspension or termination.

## 4. Host operators

If you register a host with the Service, you also agree to the [Host Operator Agreement](./HOST-AGREEMENT.md). Among other things, you acknowledge that you are providing computational infrastructure, that prompts and responses routed through your host are not your speech, and that you may opt out at any time by deactivating your host.

## 5. Pool credits

The Service tracks pool-credit accounting to balance contribution and consumption across the cooperative pool. Credits are not a currency, are not transferable outside the Service, have no monetary value, and confer no ownership or property right. We may adjust credit accounting rules at any time.

## 6. Paid features (future)

Some features of the Service may become paid in the future (for example, marketplace compute beyond what your pool credits cover). Paid features will be governed by additional terms presented at the point of purchase. Free use of the cooperative pool will remain available.

## 7. Service availability

The Service is provided "as is" and "as available." We do not guarantee uptime, latency, or that any specific model will be available in any pool at any time. Models are provided by host operators on best-effort terms; the set of available models can change at any moment.

We may modify, suspend, or discontinue any part of the Service at any time, with or without notice.

## 8. Intellectual property

The cocompute software is licensed under AGPLv3 and is available at github.com/dsegovia90/cocompute. The "cocompute" name and logo are trademarks of the Operator; you may not use them to imply endorsement or to brand a forked service without permission.

You retain all rights to the prompts you submit and the responses you receive. We do not claim ownership of your inference traffic. We may collect aggregated, non-identifying usage metadata for capacity planning and abuse monitoring.

## 9. Privacy

We collect and process the minimum data needed to operate the Service:

- Account information (email, name, role) you provide at signup
- API key hashes (we cannot recover the original key after creation)
- Per-request metadata (timestamp, model, token counts, host_id) for routing, accounting, and abuse detection
- Server logs for operational and security purposes

We do not store the contents of your prompts or model responses except as needed to deliver them. Hosts may log traffic at their own discretion; the [Host Operator Agreement](./HOST-AGREEMENT.md) requires hosts to disclose any logging that goes beyond cocompute's standard metering.

We do not sell your data. We do not use your prompts or responses to train models.

## 10. Indemnification

You agree to indemnify and hold harmless the Operator, its contributors, and its host operators from any claim arising out of your use of the Service in violation of these Terms or the AUP, including any claim related to content you submit through the Service.

## 11. Disclaimer of warranties

THE SERVICE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING WITHOUT LIMITATION ANY IMPLIED WARRANTY OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, OR NON-INFRINGEMENT. WE DO NOT WARRANT THAT THE SERVICE WILL BE UNINTERRUPTED, ERROR-FREE, OR FREE OF HARMFUL COMPONENTS.

## 12. Limitation of liability

TO THE MAXIMUM EXTENT PERMITTED BY LAW, THE OPERATOR'S AGGREGATE LIABILITY FOR ANY CLAIM ARISING OUT OF OR RELATING TO THESE TERMS OR THE SERVICE IS LIMITED TO THE GREATER OF (A) THE AMOUNT YOU HAVE PAID THE OPERATOR IN THE PAST TWELVE MONTHS, OR (B) ONE HUNDRED US DOLLARS. THE OPERATOR IS NOT LIABLE FOR INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, OR PUNITIVE DAMAGES.

## 13. Termination

You may terminate your account at any time by deleting it from the dashboard or by emailing terms@cocompute.ai. We may terminate your account immediately for material violations of these Terms or the AUP. Upon termination, your data may be deleted; metering records and audit logs may be retained for legal compliance.

The following sections survive termination: 8 (Intellectual Property), 10 (Indemnification), 11 (Disclaimer), 12 (Limitation of Liability), 14 (Governing Law), and 15 (Changes).

## 14. Governing law

These Terms are governed by the laws of [JURISDICTION TBD — Daniel to fill in based on company formation], without regard to conflict-of-law principles. Any dispute arising from these Terms shall be resolved in the courts of that jurisdiction.

## 15. Changes

We may update these Terms from time to time. Material changes will be announced via email to the address on your account or via a notice on cocompute.ai. Continued use of the Service after the changes take effect constitutes acceptance.

## 16. Contact

Questions about these Terms: terms@cocompute.ai

---

**Note for self-hosters:** if you run your own orchestrator from the open-source code, these Terms do not apply to your deployment. You become the Operator for your users and are responsible for whatever terms you choose to apply (or not). The AGPLv3 license still governs your use of the cocompute software itself.
