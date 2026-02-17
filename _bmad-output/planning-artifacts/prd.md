---
date: '2026-02-17'
stepsCompleted: [step-01-init, step-02-discovery, step-03-success, step-04-journeys, step-05-domain, step-06-innovation, step-07-project-type, step-08-scoping, step-09-functional, step-10-nonfunctional, step-11-polish]
inputDocuments: [product-brief-discool-2026-02-17.md]
documentCounts:
  briefs: 1
  research: 0
  brainstorming: 0
  projectDocs: 0
classification:
  projectType: web_app
  domain: communication_social
  complexity: high
  projectContext: greenfield
workflowType: 'prd'
---

# Product Requirements Document - Discool

**Author:** Darko
**Date:** 2026-02-17

---

## Executive Summary

**Product:** Discool — a self-hosted, open-source, P2P-federated communication platform combining real-time text chat, voice channels, guilds, and role-based permissions in a single application.

**Vision:** Replace Discord for communities that value ownership, privacy, and freedom from platform risk. Discool instances are independently operated, interconnected via P2P discovery, and require no central authority.

**Differentiator:** Discool is the first communication platform to combine:
1. **No defederation** — moderation stays at the user and guild level; no instance-level blocking
2. **Portable cryptographic identity** — one keypair, usable across all instances, based on DID/VC standards
3. **All-in-one open-source suite** — text, voice, guilds, and permissions in a single platform (no Mumble + Matrix + Jitsi fragmentation)
4. **Rust + Svelte 5 stack** — optimized for performance and low resource consumption on modest hardware

**Target Users:**
- **Community builders** migrating from proprietary platforms (Discord, Slack)
- **Self-hosters** deploying communication infrastructure on their own hardware
- **Privacy-conscious users** needing censorship-resistant, decentralized communication
- **Everyday users** joining via invite links who expect a familiar, frictionless experience

**Technology:** Rust backend (single binary or Docker), Svelte 5 SPA frontend, WebSocket for real-time messaging, WebRTC for voice. No Node.js in production.

**Development Model:** Solo developer + LLM-assisted development. Estimated MVP timeline: 11-17 weeks.

---

## Success Criteria

### User Success

| Criteria | Target | Validation |
|---|---|---|
| **Frictionless join** | Existing identity: invite click → chatting in <10 seconds. New identity: under 60 seconds including keypair creation. | Timed testing across fresh and returning user flows |
| **"It just works" UX** | Users navigate guild channels, find activity, and join conversations without guidance or tutorials | Usability testing with non-technical users; zero "where do I go?" confusion |
| **Voice quality** | Mumble-tier audio quality. ≤150ms latency for same-region users; physics-bound for cross-continent. | A/B benchmarking against Mumble and Discord; latency monitoring per connection |
| **Instance deployment** | Operator deploys a working instance within 30 minutes using Docker or single binary on a $5 VPS | End-to-end deployment testing on minimal hardware |
| **Identity portability** | A user joins guilds on 3+ different instances with the same cryptographic identity, seamlessly | Cross-instance identity validation testing |
| **Activity feels alive** | The UI surfaces active conversations, voice channels, and online presence — spaces feel alive, not empty | Qualitative user feedback; engagement telemetry |

### Business Success

| Timeframe | Success Signal | Measurable Indicator |
|---|---|---|
| **Month 1-3** | Founding community dogfoods daily | ≥10 daily active users on the primary instance; ≥100 messages/day |
| **Month 3-6** | Independent instances appear | ≥3 independently operated instances with real communities |
| **Month 6-12** | Organic word-of-mouth growth | Invite link redemptions trending upward month-over-month; GitHub stars >500 |
| **Year 2** | Managed hosting viable | ≥10 managed hosting subscribers generating revenue to fund 1 developer |
| **Year 3** | Recognized in self-hosted ecosystem | Featured in self-hosted community lists; tech media coverage; >50 known active instances |

### Technical Success

| Criteria | Target | Validation |
|---|---|---|
| **Resource efficiency** | 50 concurrent users on 2 vCPU / 2GB RAM with headroom | Load testing on $5 VPS equivalent hardware |
| **Voice reliability** | <1% call drop rate; Mumble-equivalent audio clarity | Automated voice quality testing; user-reported issues |
| **P2P discovery** | Instances discover each other within 60 seconds of coming online | Multi-instance integration testing |
| **Security hardening** | No critical or high-severity vulnerabilities in production | Periodic specialized security review cycles (automated via Jules); manual audits before major releases |
| **Architecture extensibility** | Phase 2 features (video, streaming, bot API, clustering) can be added without architectural rewrites | Architecture review; proof-of-concept spikes for Phase 2 features |
| **Uptime** | 99.5%+ for well-operated instances | Instance health monitoring and alerting |

### Measurable Outcomes

- **Primary:** "People are sharing invite links without being asked to" — organic growth is the ultimate success signal
- **Secondary:** Instance operators report stable, low-maintenance operation on modest hardware
- **Tertiary:** Contributors submit PRs, file issues, and build integrations — the project has community momentum
- **Maturity gate:** MVP ships only after feature completion + iterative security/performance/UX review cycles have brought the codebase to a hardened state

---

## Product Scope

> Detailed MVP feature set, phased roadmap, risk analysis, and timeline are in the **Project Scoping & Phased Development** section below. This section provides a high-level summary.

### MVP Core

- P2P instance discovery + portable cryptographic identity (architectural foundation)
- Text channels, voice channels (WebRTC), direct messages, file sharing, rich embeds
- Guilds, roles, channel-level permission overrides, invite links
- User blocking, guild-level moderation (mute/kick/ban), mod log, UGC reporting
- No instance-level blocking (by design)
- Svelte 5 SPA with activity-driven UI, frictionless onboarding
- Rust backend, single-binary or Docker, optimized for 2 vCPU / 2GB RAM
- Quality gates: feature complete → security review cycles → performance benchmarking → maturation period

### Post-MVP

- **Phase 1.5:** Advanced moderation queue, Tor hardening, key export/import, instance health dashboard
- **Phase 2:** Video/screen sharing, forum channels, bot API, E2E encryption, Discord migration, clustering
- **Phase 3:** Streaming (Go Live), quantum-proof crypto, stage channels, app directory, native mobile

---

## User Journeys

### Journey 1: Maya — "Building a Home for Her Community"

**Opening Scene:**
Maya's open-source project has 2,000 members on Discord. She's just received her third DMCA takedown for a bot that archives messages — Discord doesn't allow it. Her project's entire knowledge base, 3 years of technical discussions, contributor relationships, and onboarding flows are locked inside a platform she can't control. She finds Discool through a post on Hacker News.

**Rising Action:**
Maya asks her friend Tomás (who's already running a Discool instance) to create a guild for her project. Tomás creates the guild in seconds. Maya gets an admin invite link. She opens it in her browser, creates her portable identity (a keypair generated client-side), picks a display name, and she's in — her own guild, on infrastructure she trusts.

She starts setting up: creates channels (#general, #dev, #support, #announcements), configures roles (Maintainer, Contributor, Community), sets channel permissions so only Maintainers can post in #announcements. The permission system feels like Discord's — she doesn't have to relearn anything. She creates an invite link and shares it in her Discord server: "We're moving. Join us here."

**Climax:**
The first 50 members join over a weekend. The activity feed shows conversations happening in real-time. Someone drops into the voice channel to pair-program. Maya realizes: this is *her* space. Nobody can shut it down, throttle it, or change the rules. The message history is on Tomás's server, backed up, exportable. She can see the life in the guild — active channels glow, voice channels show who's talking. It feels alive.

**Resolution:**
Three months later, Maya's guild has 400 active members. She's archived her Discord and put a redirect notice. Contributors who were skeptical now say "this is actually better." Maya's portable identity works across two other Discool instances she's joined for other projects. She never thinks about platform risk again.

**Requirements revealed:** Guild creation, channel management, role/permission system, invite links, activity surfacing, cross-instance identity, message persistence.

---

### Journey 2: Liam — "Clicking an Invite Link for the First Time"

**Opening Scene:**
Liam's friend sends him a link in a group chat: "we're hanging out here tonight, join." Liam has never heard of Discool. He clicks the link on his phone browser.

**Rising Action:**
The Discool web client loads. It's fast — no app download required. A clean onboarding screen says: "Create your identity to join." Liam taps a button. Behind the scenes, a cryptographic keypair is generated and stored in his browser. He picks a username and avatar. Total time: 8 seconds. He's in the guild.

The UI feels familiar — a channel list on the left, messages in the center, members on the right. But it's cleaner than Discord. He sees a green pulse on the voice channel — his friends are already in there. He taps it and drops into voice. Audio is crisp, no lag. He types "yo" in #general. Someone reacts with an emoji.

**Climax:**
An hour in, Liam hasn't thought about the platform once. Voice didn't cut out. Nobody asked him to pay for better quality. He shared a screenshot and it uploaded instantly — no 8MB limit. He scrolled up in chat history and found a link from last week. "This is just... Discord but free?"

**Resolution:**
Liam joins three more guilds over the next month, all on different instances. His identity carries over — same username, same avatar, seamless. He doesn't know or care what "P2P discovery" or "portable identity" means. It just works.

**Requirements revealed:** Frictionless onboarding (no app install), client-side identity generation, fast load times, familiar UX patterns, voice channel drop-in, file upload without arbitrary limits, cross-instance identity portability.

---

### Journey 3: Tomás — "Spinning Up His Own Instance"

**Opening Scene:**
Tomás sees Discool on GitHub. The README says: "Deploy in 5 minutes. Docker or single binary." He has a $5/month VPS running Debian that hosts his personal services.

**Rising Action:**
Tomás SSHs into his VPS. He pulls the Docker image, writes a quick `docker-compose.yml` with the example config, sets his domain name and TLS cert path, and runs `docker compose up -d`. The instance starts. He opens `discool.tomas.dev` in his browser — the admin setup screen appears. He creates the first identity (which becomes the instance admin), sets a few instance-level defaults, and creates his first guild for his friend group.

He checks resource usage: 80MB RAM, barely touching the CPU. He smiles.

**Climax:**
Over the next week, 15 friends join. Voice channels work flawlessly. Tomás checks the P2P network status page — his instance has automatically discovered 12 other instances. Users from those instances could join his guild with an invite link, using their existing identities. No federation setup, no manual peering. It just happened.

He runs `discool backup` and gets a clean database export. His data. His hardware. His rules.

**Resolution:**
Six months later, Tomás is running 5 guilds on his instance for different friend groups and a local community. RAM usage is at 400MB for 50 regular users. He's set up automated backups. When a Discool update drops, he pulls the new image and restarts — zero downtime with the rolling update. He recommends it to every self-hoster he knows.

**Requirements revealed:** Docker/single-binary deployment, minimal configuration, low resource footprint, P2P auto-discovery, instance admin panel, backup/export, rolling updates, resource monitoring.

---

### Journey 4: Aisha — "A Space No One Can Take Away"

**Opening Scene:**
Aisha coordinates a network of journalists and activists across three countries. Their previous Telegram group was infiltrated. Their Signal group is too limited for structured community discussion. They need channels, roles, persistent history — but on infrastructure no government can subpoena or shut down through a support ticket.

**Rising Action:**
Aisha asks a trusted technical contact to deploy a Discool instance on a VPS in a privacy-friendly jurisdiction. The instance is set up with a `.onion` address as well as a clearnet domain. Aisha creates her identity and sets up the guild with strict roles: Verified Journalist, Source, Coordinator. Channel permissions ensure Sources can only see #secure-contact. She generates single-use invite links for each new member.

**Climax:**
During a political crisis, a government orders the clearnet domain seized. The instance is still accessible via its `.onion` address. More importantly, the P2P network means other Discool instances still see it — there's no central registry that was updated to remove it. The instance simply exists. Members with the `.onion` bookmark reconnect within minutes.

Aisha's group continues operating. No data was lost. No accounts were banned. The cryptographic identities can't be impersonated.

**Resolution:**
Aisha sets up redundant instances across jurisdictions. Members' portable identities work on all of them. If one goes down, the community continues on another — same identities, new invite links, no disruption. She has, for the first time, communication infrastructure that matches the threat model of her work.

**Requirements revealed:** P2P discovery resilience (no central registry), portable identity across instances, fine-grained role/permission control, single-use invite links, Tor-compatible deployment, no instance-level blocking.

---

### Journey 5: Rico — "Keeping the Peace"

**Opening Scene:**
Rico is a moderator in Maya's open-source guild. He's not a developer — he's a community manager who volunteers because he believes in the project. The guild has 400 members and growing. Most are great. Some aren't.

**Rising Action:**
Rico notices a user spamming links in #general. He right-clicks the username → "Mute for 1 hour." Done. Later, a user is being abusive in voice chat. Rico opens the voice channel panel, sees the user's audio waveform, and clicks "Kick from voice." The user is removed from the voice channel but can still access text channels.

A more serious situation: someone is posting harmful content across multiple channels. Rico opens their profile, sees their message history across the guild (moderator-level visibility), and decides to ban. He clicks "Ban from guild" — the user is removed and can't rejoin with the same identity. Rico logs the reason in the mod log.

**Climax:**
Maya reviews the mod log at the end of the week. Clean, auditable actions with timestamps, reasons, and moderator names. She adjusts Rico's permissions to also manage roles for newcomers. The permission system is granular enough that Rico can moderate without having admin access to server settings.

**Resolution:**
The guild grows to 800 members. Rico recruits two more moderators. They coordinate in a moderators-only channel. The moderation tools are powerful enough to handle a growing community but simple enough that non-technical volunteers can use them confidently.

**Requirements revealed:** Moderator role with granular permissions, mute/kick/ban actions, mod log with audit trail, per-channel moderation, voice channel moderation, role management delegation, moderator-only channels.

---

### Journey 6: Liam — "Something Went Wrong" (Edge Case)

**Opening Scene:**
Liam is in a voice channel with 8 friends gaming. His internet connection drops for 30 seconds, then comes back.

**Rising Action:**
When Liam's connection returns, the Discool client automatically reconnects to the voice channel. He's back in — no manual rejoin needed. A small toast notification says "Reconnected." He missed about 20 seconds of conversation but the text chat was still receiving messages (they load when he reconnects). His friends didn't even notice he dropped — the voice channel showed his avatar greyed out briefly, then back to normal.

**Climax:**
Later, Liam tries to join a guild on a different instance, but that instance is temporarily down. The client shows a clear message: "This instance is currently unreachable. Your invite link will work when it's back online." No cryptic error. No confusion. Liam tries again an hour later and gets in. His existing identity is recognized instantly.

Another day, Liam accidentally closes his browser tab. He reopens the URL and the client loads his identity from local browser storage — he's logged in automatically. All his guilds, all his channels, exactly where he left off.

**Resolution:**
Liam never loses data, never loses his identity, and never has to troubleshoot anything. The worst that happens is a brief reconnection. The platform handles failures gracefully, communicates clearly, and recovers automatically.

**Requirements revealed:** Auto-reconnect for voice and text, graceful degradation on connection loss, persistent local identity storage, clear error messaging, offline message queuing, instance availability indication.

---

### Journey Requirements Summary

| Journey | Key Capabilities Revealed |
|---|---|
| **Maya (Community Builder)** | Guild creation, channel management, role/permission system, invite links, activity surfacing, message persistence |
| **Liam (New User)** | Frictionless onboarding, client-side identity generation, familiar UX, voice drop-in, file upload, cross-instance identity |
| **Tomás (Instance Operator)** | Docker/binary deployment, minimal config, low resource usage, P2P auto-discovery, admin panel, backup/export |
| **Aisha (Privacy Advocate)** | P2P resilience, portable identity, fine-grained permissions, single-use invites, Tor compatibility, no instance blocking |
| **Rico (Guild Moderator)** | Moderator permissions, mute/kick/ban, mod log, voice moderation, role delegation, moderator channels |
| **Liam (Edge Case)** | Auto-reconnect, graceful degradation, persistent identity storage, clear errors, offline queuing |

---

## Domain-Specific Requirements

### Compliance & Regulatory

- **GDPR support:** The software must enable instance operators to comply with GDPR — user data export, account deletion, data portability. The project itself is not a data controller; each instance operator is.
- **Operator responsibility model:** Discool provides moderation tools and operator controls. Operators are responsible for the content hosted on their instances and compliance with their local jurisdiction.
- **No central authority:** No DMCA takedown, no government request can target "Discool" as an entity — only individual instances. This is by design.

### Security Constraints

- **Transport encryption:** All WebSocket and WebRTC connections must use TLS/DTLS. Voice/video streams use SRTP (standard WebRTC).
- **Cryptographic identity security:** Private keys stored client-side must be protected (encrypted local storage). Key material never leaves the client unencrypted.
- **Permission system hardening:** Role-based permission system is the primary security boundary within guilds. Privilege escalation must be prevented through rigorous testing and principle of least privilege.
- **Input sanitization:** All user-generated content must be sanitized against XSS, injection, and payload attacks. Chat platforms are high-value targets. Content Security Policy (CSP) headers enforced.
- **Rate limiting:** API endpoints rate-limited to prevent abuse, spam, and denial-of-service.
- **Identity recovery via email:** Users can optionally associate an email address with their cryptographic identity for key recovery. If browser storage is lost, the user can recover their identity through email verification. Not the most secure mechanism, but pragmatic — losing your identity entirely is worse.

### UGC Reporting & Content Moderation

- **User reporting:** Any user can report a message, file, or user to guild moderators via a "Report" action. Reports include the content, reporter identity, and optional reason.
- **Moderator report queue:** Guild moderators see a report queue with pending reports, reported content preview, and action buttons (dismiss, warn, mute, kick, ban).
- **Report lifecycle:** Reports are tracked with status (pending, reviewed, actioned, dismissed) and linked to mod log entries.
- **No platform-level reporting:** Reports go to guild moderators, not to a central authority. Consistent with the decentralized model.

### P2P & Distributed System Constraints

- **Sybil resistance:** DHT/gossip protocol must include mechanisms to resist Sybil attacks (proof-of-work, reputation, or rate-based participation controls).
- **Private instance support:** Instances can opt out of P2P discovery (unlisted mode). Discovery only exposes instances that choose to be discoverable.
- **Identity verification:** Cross-instance identity verification uses public key cryptography. Identity impersonation is prevented by cryptographic proof — a user proves they hold the private key.
- **No single point of failure:** No bootstrap node, directory, or registry is required for the network to function. Loss of any single node does not fragment the network.

### Domain-Specific Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Privilege escalation** | Unauthorized admin/mod access | Rigorous permission testing; least privilege; security review cycles |
| **Identity impersonation** | Attacker poses as another user | Cryptographic identity verification; public key challenges |
| **Illegal content on instances** | Legal liability for operators | Operator documentation; UGC reporting tools; moderator queue; guild-level moderation |
| **Sybil attacks on P2P** | Fake instances flood network | Proof-of-work or reputation-based DHT participation |
| **XSS via chat messages** | Client-side code execution | Input sanitization; CSP headers; sandboxed rendering |
| **Voice/WebRTC DoS** | Resource exhaustion via voice | Connection rate limiting; TURN server resource caps |
| **Identity key loss** | User loses access permanently | Email-based recovery (opt-in); key export/backup tooling |
| **Operator abuse** | Instance operator surveils users | Transparent architecture; users can migrate identity to other instances |

---

## Innovation & Novel Patterns

### Detected Innovation Areas

**1. Censorship-Resistant Federation Without Defederation**
- **Innovation:** Discool is the first communication platform to explicitly reject instance-level blocking as an architectural principle. Unlike Mastodon/ActivityPub-based platforms where defederation fragments the network and enables cancel culture at the infrastructure level, Discool keeps moderation where it belongs: users block users, guilds ban members.
- **Assumption challenged:** "Federated networks need instance-level blocking for safety." Discool argues this creates more problems than it solves — it centralizes power in instance operators and fragments communities.
- **Prior art:** None identified. Forks may add defederation, but the upstream project will not officially support it.
- **Why it matters:** This is a philosophical stance embedded in architecture, not a feature flag. It defines the network's character.

**2. Portable Cryptographic Identity Based on Existing Standards**
- **Innovation:** Users are their keypair. One identity across all instances, cryptographically verified. Built on existing standards (DID, Verifiable Credentials, OIDC-compatible) rather than inventing a custom protocol.
- **Assumption challenged:** "Users need an account on each server." Discool says the identity layer is independent of any instance.
- **Why it matters:** Eliminates account proliferation, enables true portability, and makes the user sovereign over their identity.

**3. All-in-One Open-Source Communication Suite**
- **Innovation:** No existing open-source project combines real-time text chat, voice channels, video (future), streaming (future), guilds, and role-based permissions in a single platform with a modern UX.
- **Assumption challenged:** "Open-source communication tools should specialize." Discool says integration beats fragmentation — users shouldn't need Mumble + Matrix + Jitsi + a custom forum.

**4. Rust + Svelte 5 Full-Stack for Real-Time Communication**
- **Innovation:** Uncommon technology pairing — Rust backend for a communication platform (most are Node/Python/Go) combined with Svelte 5 SPA frontend. Optimized for performance and low resource consumption from the ground up.
- **Why it matters:** Directly addresses the #1 complaint about Matrix/Synapse (Python = slow, resource-hungry). A Discool instance should run circles around equivalent Matrix deployments on the same hardware.

### Market Context & Competitive Landscape

| Competitor | What They Do Well | Where Discool Differs |
|---|---|---|
| **Discord** | UX, feature completeness, network effects | Proprietary, surveillance, paywalled features, de-platforming risk |
| **Matrix/Element** | Open protocol, federation, encryption | Slow (Python), complex UX, no integrated voice/video/streaming, defederation possible |
| **Mumble** | Excellent voice quality, low latency | Voice only, no text, no modern UX |
| **Rocket.Chat** | Team messaging, self-hosted | Not designed for community guilds, no voice/video integration |
| **Revolt** | Open-source Discord clone | No P2P discovery, no portable identity, smaller feature set |
| **Guilded** | Gaming features | Proprietary (Roblox-owned) |

### Validation Approach

| Innovation | How to Validate | When |
|---|---|---|
| **No defederation** | Deploy multi-instance network; observe if communities form healthily without instance-level blocking | MVP launch + 6 months observation |
| **Portable identity** | Cross-instance identity verification testing; user testing of identity creation and recovery flows | During MVP development |
| **All-in-one suite** | User satisfaction compared to using multiple specialized tools; migration rate from Discord | MVP launch + community feedback |
| **Rust performance** | Benchmarking against Matrix/Synapse on equivalent hardware; load testing | During MVP development |

### Risk Mitigation

| Innovation Risk | Fallback |
|---|---|
| **P2P discovery too complex for MVP** | Central directory as interim discovery mechanism; migrate to P2P when ready |
| **No defederation causes abuse** | Guild-level moderation tools are robust; user-level blocking is comprehensive; instance operators can moderate their own guilds |
| **Portable identity UX is confusing** | Email-based recovery simplifies the experience; progressive disclosure of cryptographic details |
| **All-in-one scope too ambitious** | Text + voice MVP first; video/streaming in Phase 2; each component is independently valuable |

---

## Web Application Specific Requirements

### Project-Type Overview

Discool is a **pure Single Page Application (SPA)** built with Svelte 5, served as static assets from the Rust backend. There is no Node.js in the production stack — the Rust server handles HTTP, WebSocket, WebRTC signaling, and serves the pre-built SPA bundle.

**Key architectural decision:** No SSR, no Node runtime. The Rust backend serves the initial HTML with pre-populated meta tags (OpenGraph, title, description) for link unfurling. SEO is secondary to keeping the stack simple and Node-free.

### Browser Support

| Browser | Minimum Version | Rationale |
|---|---|---|
| Chrome | Latest 2 versions | WebRTC + modern JS required |
| Firefox | Latest 2 versions | WebRTC + modern JS required |
| Safari | Latest 2 versions | WebRTC + modern JS required |
| Edge | Latest 2 versions | Chromium-based, same as Chrome |
| Mobile Chrome | Latest 2 versions | Responsive SPA on mobile |
| Mobile Safari | Latest 2 versions | iOS users |

**Not supported:** IE11, legacy Edge, browsers without WebRTC. No polyfills for deprecated engines.

### Responsive Design

- **Desktop-first** design — this is a communication platform, desktop is the primary form factor
- **Responsive down to mobile** — the SPA must work on phone browsers (invite links will be opened on mobile)
- **Breakpoints:** Desktop (≥1024px), Tablet (768–1023px), Mobile (<768px)
- **Mobile adaptations:** Collapsible sidebar, single-panel navigation, voice channel controls adapted for touch
- **No native mobile app for MVP** — the responsive SPA is the mobile experience

### Performance Targets

> Specific measurable targets with measurement methods are in **Non-Functional Requirements > Performance** (NFR1–NFR10).

### SEO Strategy

- **App itself:** No SEO needed — authenticated SPA behind identity creation
- **Invite link pages:** Rust backend serves HTML with OpenGraph meta tags for link preview unfurling (title, description, guild icon). No SSR required — static meta tags in the HTML template.
- **Public landing page (future):** Static HTML served by Rust, or a separate static site. Not a priority for MVP.
- **Philosophy:** SEO is not worth adding Node to the stack. If it can be done from Rust-served static HTML, great. Otherwise, skip it.

### Accessibility

> Specific measurable targets with measurement methods are in **Non-Functional Requirements > Accessibility** (NFR30–NFR34). Discord's poor accessibility is a competitive advantage for Discool.

### Real-Time Communication

- **Primary transport:** WebSocket for all real-time text, presence, and signaling
- **Voice/video:** WebRTC with STUN/TURN infrastructure
- **Fallback transport:** None for MVP. WebSocket is universally supported. If a user's network blocks WebSocket, they likely can't use voice either. No SSE fallback.
- **Reconnection:** Automatic reconnection with exponential backoff on both WebSocket and WebRTC

### Implementation Considerations

- **Build pipeline:** Svelte 5 app compiled at build time (Vite-based). Output is static JS/CSS/HTML. Node is a dev dependency only, never in production.
- **Asset serving:** Rust backend serves the SPA bundle. Single binary includes embedded assets or serves from a directory.
- **Code splitting:** Lazy-load guild/channel views to keep initial bundle small
- **State management:** Svelte 5 runes for reactive state. No external state library unless complexity demands it.

---

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP Approach:** Architecture-first, experience-focused MVP
**Development Model:** Solo developer + LLM-assisted development
**Resource Reality:** One person building everything — backend, frontend, infrastructure, testing, security hardening. LLM tooling (Jules, Antigravity) handles implementation velocity; periodic automated review cycles handle quality.

**Strategic implication:** The architecture (P2P, portable identity) must be right from the start because one person can't afford a rewrite. But feature scope must be aggressive in what it cuts. Every feature must earn its place.

### MVP Feature Set (Phase 1)

**Core User Journeys Supported:**
- ✅ **Liam (New User)** — Full journey: invite link → identity creation → chatting + voice
- ✅ **Tomás (Instance Operator)** — Full journey: deploy → configure → invite friends
- ✅ **Maya (Community Builder)** — Full journey: create guild → channels → roles → invite → manage
- ✅ **Rico (Moderator)** — Essential journey: mute, kick, ban, mod log (advanced moderation queue is Phase 1.5)
- ⚠️ **Aisha (Privacy Advocate)** — Architecture supports her journey; Tor-specific hardening is Phase 1.5
- ✅ **Liam (Edge Case)** — Auto-reconnect, error handling, identity persistence

**Must-Have Capabilities:**

| Capability | Justification |
|---|---|
| P2P instance discovery (or central directory fallback) | Architectural foundation — non-negotiable |
| Portable cryptographic identity (DID/VC-based) | Architectural foundation — non-negotiable |
| Text channels with persistent history | Core use case |
| Voice channels via WebRTC | Core differentiator vs. text-only alternatives |
| Guild creation and management | Community structure |
| Role-based permissions with channel overrides | Essential for any community of >5 people |
| Invite link system | Growth mechanism — the only way guilds grow |
| User blocking + guild-level kick/ban/mute | Minimum moderation for a usable community |
| Mod log | Accountability for moderation actions |
| UGC reporting (basic) | Users need a "report" button; mods need a queue |
| Identity recovery via email | Without this, one cleared browser = identity lost |
| Docker + single-binary deployment | Operator experience — must be trivial to deploy |
| Auto-reconnect (WebSocket + WebRTC) | Reliability — connection drops are inevitable |
| Clean, intuitive Svelte 5 SPA | UX is a core differentiator |

**Explicitly NOT in Phase 1:**

| Feature | Reason |
|---|---|
| Video calls & screen sharing | Complex WebRTC scaling; voice is sufficient |
| Streaming (Go Live) | Requires SFU infrastructure |
| Bot/plugin API | Ecosystem play — needs communities first |
| Forum/thread channels | Additive; text channels cover core needs |
| E2E encryption | Significant complexity; design for it, ship later |
| Horizontal clustering | Single-node Rust handles MVP scale |
| Quantum-proof crypto | Algorithm-agile design is sufficient for now |
| Advanced UGC report queue UI | Basic reporting works; fancy queue is Phase 2 |
| Tor-specific hardening | Architecture supports it; hardening needs dedicated testing |

### Post-MVP Features

**Phase 1.5 (Quick Wins After Launch):**
- Advanced moderator report queue with filtering and bulk actions
- Tor compatibility testing and `.onion` deployment documentation
- Key export/import for identity backup (beyond email recovery)
- Instance health dashboard for operators

**Phase 2 (Growth):**
- Video calls and screen sharing in voice channels
- Forum/thread channels for async discussion
- Public bot/plugin API with developer documentation
- E2E encryption (opt-in per channel)
- Discord import/migration tooling
- Horizontal clustering

**Phase 3 (Vision):**
- One-to-many streaming (Go Live)
- Quantum-proof cryptographic identity upgrade
- Stage channels for events
- App/integration directory
- Custom emoji and sticker support
- Native mobile apps (if demand warrants)

### Risk Mitigation Strategy

**Technical Risks:**

| Risk | Likelihood | Mitigation |
|---|---|---|
| P2P discovery is harder than expected | Medium | Central directory as fallback; implement P2P incrementally |
| WebRTC voice quality issues | Medium | Use proven libraries (libwebrtc bindings); benchmark against Mumble early |
| Permission system has security holes | High (for any RBAC) | Dedicated security review cycles via Jules; fuzz testing |
| Svelte 5 SPA performance with large guilds | Low | Virtual scrolling; lazy loading; Svelte compiles efficient code |
| Single-binary embedding of SPA assets | Low | Rust-embed crate is mature; well-trodden path |

**Market Risks:**

| Risk | Likelihood | Mitigation |
|---|---|---|
| Not enough people care about self-hosted Discord | Low | Reddit thread already showed demand; self-hosted community is growing |
| Discord improves enough to remove motivation | Low | Discord's business model inherently conflicts with user freedom |
| Matrix/Revolt improve faster | Medium | Discool's no-defederation + all-in-one stack is unique positioning |
| No contributors join | Medium | LLM-assisted development reduces dependency on contributors; announce early, build in public |

**Resource Risks:**

| Risk | Mitigation |
|---|---|
| Solo dev burns out | Aggressive scope cuts; ship MVP fast; automate quality with Jules cycles |
| LLM-generated code has security issues | Periodic security review prompts; never ship without hardening cycles |
| Architecture decisions are wrong | Prototype P2P and identity early; validate before building features on top |
| Timeline slips | Phase 1.5 exists as a buffer; some Phase 1 features can be deferred if needed |

### Minimum Viable Timeline (Estimate)

| Phase | Scope | Solo + LLM Estimate |
|---|---|---|
| **Foundation** | Rust backend scaffold, identity system, P2P/directory, WebSocket | 2-3 weeks |
| **Core features** | Text channels, guilds, roles, permissions, invites | 2-3 weeks |
| **Voice** | WebRTC integration, voice channels | 1-2 weeks |
| **Frontend** | Svelte 5 SPA, all UI, onboarding flow | 2-3 weeks |
| **Moderation + UGC** | Kick/ban/mute, mod log, reporting | 1 week |
| **Deployment** | Docker, single binary, documentation | 1 week |
| **Hardening** | Security review cycles, performance testing, bug fixes | 2-4 weeks |
| **Total MVP** | | **~11-17 weeks** |

---

## Functional Requirements

### Identity & Authentication

- FR1: Users can create a portable cryptographic identity (keypair) client-side without requiring a server account
- FR2: Users can set a display name and avatar for their identity
- FR3: Users can authenticate across multiple Discool instances using a single identity
- FR4: Users can optionally associate an email address with their identity for recovery
- FR5: Users can recover their identity via email if browser storage is lost
- FR6: Users can persist their identity in browser storage and resume sessions without re-authentication
- FR7: The system can verify a user's identity cryptographically when they join an instance

### Instance Management

- FR8: Operators can deploy a Discool instance via Docker or single binary
- FR9: Operators can configure instance settings (domain, TLS, defaults) through a configuration file
- FR10: Operators can access an admin setup screen on first launch to initialize the instance
- FR11: Operators can view instance resource usage and health status
- FR12: Operators can export and back up instance data
- FR13: Instances can discover other Discool instances via P2P network (or central directory fallback)
- FR14: Operators can opt their instance out of P2P discovery (unlisted mode)

### Guild Management

- FR15: Users can create guilds on an instance
- FR16: Guild owners can configure guild settings (name, icon, description)
- FR17: Guild owners can create, rename, reorder, and delete text channels within a guild
- FR18: Guild owners can create, rename, reorder, and delete voice channels within a guild
- FR19: Guild owners can create channel categories to organize channels
- FR20: Guild owners can generate invite links for their guild
- FR21: Guild owners can generate single-use invite links
- FR22: Guild owners can revoke invite links
- FR23: Users can join a guild via an invite link

### Roles & Permissions

- FR24: Guild owners can create, edit, and delete custom roles
- FR25: Guild owners can assign permissions to roles (send messages, manage channels, kick members, ban members, manage roles, etc.)
- FR26: Guild owners can set role hierarchy to determine permission precedence
- FR27: Guild owners can assign roles to guild members
- FR28: Guild owners can set channel-level permission overrides for specific roles
- FR29: Guild owners can delegate role management to specific roles (e.g., moderators can assign roles)

### Text Communication

- FR30: Users can send text messages in text channels
- FR31: Users can view persistent message history in text channels
- FR32: Users can edit their own messages
- FR33: Users can delete their own messages
- FR34: Users can react to messages with emoji
- FR35: Users can upload and share files in channels
- FR36: Users can view rich embeds for shared links and images
- FR37: Users can send direct messages to other users
- FR38: Users can scroll through message history with messages loading progressively

### Voice Communication

- FR39: Users can join and leave voice channels
- FR40: Users can see who is currently in a voice channel before joining
- FR41: Users can mute and unmute their own microphone
- FR42: Users can deafen and undeafen their own audio
- FR43: Users can adjust individual user volumes in a voice channel
- FR44: The system can automatically reconnect users to voice after a brief connection loss

### Moderation & Safety

- FR45: Moderators can mute a user in a guild (timed or permanent)
- FR46: Moderators can kick a user from a guild
- FR47: Moderators can ban a user from a guild (preventing rejoin with the same identity)
- FR48: Moderators can kick a user from a voice channel
- FR49: Moderators can view a user's message history within the guild
- FR50: Moderators can delete any user's messages within the guild
- FR51: The system logs all moderation actions in an auditable mod log with timestamps, actions, and moderator identity
- FR52: Users can block other users (personal, client-side)
- FR53: Users can report messages, files, or users to guild moderators
- FR54: Moderators can view and act on a report queue (dismiss, warn, mute, kick, ban)
- FR55: Reports are tracked with lifecycle status (pending, reviewed, actioned, dismissed)

### User Experience & Navigation

- FR56: Users can see activity indicators on channels and guilds (active conversations, voice participants, online members)
- FR57: Users can view a member list for any guild they belong to
- FR58: Users can see online/offline status of guild members
- FR59: Users can navigate between multiple guilds they've joined
- FR60: Users can access the platform via an invite link without installing an application
- FR61: The system provides clear error messages when operations fail (instance unreachable, permission denied, etc.)
- FR62: The system automatically reconnects WebSocket connections after brief disconnections

### Data & Privacy

- FR63: Users can export their personal data from an instance (GDPR support)
- FR64: Users can delete their account from an instance (GDPR support)
- FR65: The system sanitizes all user-generated content to prevent XSS and injection attacks
- FR66: The system rate-limits API endpoints to prevent abuse

---

## Non-Functional Requirements

### Performance

| NFR | Target | Measurement |
|---|---|---|
| NFR1: WebSocket message delivery latency | <100ms same-region, <300ms cross-continent | Server-side message timestamp delta |
| NFR2: Voice channel join time | <2 seconds from click to connected | Client-side timing |
| NFR3: Voice audio latency | ≤150ms same-region (Mumble-tier) | Round-trip audio measurement |
| NFR4: SPA initial load | <3 seconds on 4G connection | Lighthouse / WebPageTest |
| NFR5: Time to interactive | <2 seconds after initial load | Lighthouse TTI metric |
| NFR6: SPA bundle size | <500KB gzipped (initial chunk) | Build output measurement |
| NFR7: Client memory usage | <200MB with 5 guilds active | Browser DevTools heap snapshot |
| NFR8: Message history scroll | Smooth 60fps scrolling with 10,000+ messages in channel | FPS monitoring during scroll |
| NFR9: Server resource usage | 50 concurrent users on 2 vCPU / 2GB RAM with headroom | Load testing |
| NFR10: Server cold start time | <5 seconds from binary launch to accepting connections | Startup timing |

### Security

| NFR | Target | Measurement |
|---|---|---|
| NFR11: Transport encryption | All connections use TLS 1.3+ (HTTP, WebSocket) and DTLS (WebRTC) | TLS configuration audit |
| NFR12: Voice encryption | All voice streams use SRTP (standard WebRTC) | WebRTC connection inspection |
| NFR13: Identity key protection | Private keys encrypted at rest in browser storage; never transmitted | Code review; network traffic audit |
| NFR14: Input sanitization | Zero XSS vulnerabilities in UGC rendering | Automated XSS fuzzing; CSP header validation |
| NFR15: Rate limiting | All API endpoints rate-limited; abuse attempts rejected within 1 second | Rate limit testing under load |
| NFR16: Permission enforcement | Zero privilege escalation paths; every API call validates permissions server-side | Security review cycles; fuzz testing |
| NFR17: Dependency security | No known critical CVEs in production dependencies | Automated dependency scanning (cargo audit, npm audit) |
| NFR18: Security review cadence | Automated security review cycle runs at minimum before each release | Jules security review prompt execution |

### Scalability

| NFR | Target | Measurement |
|---|---|---|
| NFR19: Single-instance capacity | 50 concurrent users with <10% performance degradation vs. 1 user | Load testing with graduated user counts |
| NFR20: Message throughput | 100 messages/second sustained on reference hardware (2 vCPU/2GB) | Throughput benchmarking |
| NFR21: Voice channel capacity | 15 simultaneous voice users per channel without quality degradation | Voice quality testing |
| NFR22: Guild capacity | 50+ guilds per instance without performance impact | Guild creation stress test |
| NFR23: Data growth | Database handles 1 million messages without query degradation | Database benchmarking with synthetic data |

### Reliability

| NFR | Target | Measurement |
|---|---|---|
| NFR24: Instance uptime | 99.5%+ for well-operated instances | Uptime monitoring |
| NFR25: WebSocket auto-reconnect | Automatic reconnection within 5 seconds of connection restoration | Disconnect/reconnect testing |
| NFR26: Voice auto-reconnect | Automatic voice reconnection within 5 seconds; no manual rejoin needed | Network interruption simulation |
| NFR27: Data durability | Zero message loss under normal operation; messages persisted before acknowledgment | Write-ahead-log verification |
| NFR28: Graceful degradation | If voice fails, text continues working; partial failures don't cascade | Fault injection testing |
| NFR29: Backup integrity | Exported backups can be fully restored to a new instance | Backup/restore cycle testing |

### Accessibility

| NFR | Target | Measurement |
|---|---|---|
| NFR30: WCAG compliance | Level AA conformance per WCAG 2.1 | Automated accessibility audit (axe-core) + manual testing |
| NFR31: Keyboard navigation | All interactive elements reachable and operable via keyboard | Manual keyboard-only testing |
| NFR32: Screen reader support | All dynamic content announced via ARIA live regions; all controls labelled | Screen reader testing (NVDA, VoiceOver) |
| NFR33: Color contrast | Minimum 4.5:1 ratio for normal text, 3:1 for large text | Automated contrast checking |
| NFR34: Motion sensitivity | Reduced motion option disables all animations and transitions | prefers-reduced-motion media query verification |

### Operational

| NFR | Target | Measurement |
|---|---|---|
| NFR35: Deployment time | New instance operational within 30 minutes including configuration | End-to-end deployment testing |
| NFR36: Update process | Zero-downtime updates via container restart or binary swap | Update procedure testing |
| NFR37: Monitoring | Instance exposes health check endpoint and basic metrics | Health endpoint verification |
| NFR38: Logging | Structured logging with configurable verbosity; no PII in logs by default | Log output review |
