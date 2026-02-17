---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments: []
date: 2026-02-17
author: Darko
---

# Product Brief: discool

## Executive Summary

Discool is an open-source, self-hosted communication platform designed as a complete alternative to Discord. It combines real-time text chat, threaded forums, voice, video, screen sharing, and one-to-many streaming into a single cohesive application — a combination no existing open-source solution offers. Built with a Rust backend for high performance and a Svelte 5 SPA frontend for a modern user experience, Discool empowers individuals, communities, and organizations to own their communication infrastructure entirely.

Unlike centralized platforms, Discool instances discover each other through peer-to-peer protocols (e.g., DHT or similar), eliminating any central directory or single point of failure. Unlike federated platforms such as Mastodon, Discool explicitly prevents instance-level blocking — ensuring the network remains censorship-resistant by design. Moderation happens where it should: at the user and guild level, controlled by the people who run and participate in communities.

Discool scales with your hardware. There are no artificial limits on members, channels, message history, or file sizes. Run it on a small VPS for a tight-knit group, or cluster multiple nodes for horizontal scaling to support thousands.

---

## Core Vision

### Problem Statement

Discord has become the de facto standard for online community communication, but it is entirely proprietary. Users have no control over their data, no ability to self-host, and no recourse when the platform changes policies, raises prices, or shuts down communities. Every message, file, and relationship lives on Discord's servers under Discord's rules.

### Problem Impact

- **Data sovereignty:** Users and organizations cannot control where their data lives, who accesses it, or how it's used — a growing concern under regulations like GDPR.
- **Censorship and de-platforming:** Communities can be removed at Discord's discretion, with no appeal process and no data export.
- **Privacy erosion:** User behavior is tracked and monetized. There is no opt-out.
- **Platform lock-in:** Years of community history, bots, and integrations cannot be migrated. Users are trapped.
- **Feature gatekeeping:** Core functionality is paywalled behind Nitro subscriptions and server boost tiers.
- **Developer constraints:** Bot and integration APIs are limited by Discord's priorities, not community needs.

### Why Existing Solutions Fall Short

- **Matrix/Element:** The closest alternative, but the reference server (Synapse) is slow Python, the Conduit Rust server lacks features, and the onboarding experience is confusing for non-technical users. It does not natively combine forum-style threads with voice/video/streaming in a Discord-like UX.
- **Mumble:** Voice-only. No text, no forums, no video.
- **Rocket.Chat:** Primarily a team messaging tool, not designed for large community management with guild structures, roles, and real-time voice/video.
- **Guilded:** Gaming-focused, proprietary, and now owned by Roblox.
- **Mastodon/Fediverse:** Demonstrates self-hosting appeal, but suffers from central instance dominance, instance-level defederation enabling censorship and cancel culture, and is a microblogging platform — not a real-time communication suite.

**The gap:** No open-source, self-hosted platform combines IRC-style real-time chat, forum/thread discussions, voice channels, video calls, screen sharing, and one-to-many streaming in a single cohesive application with a modern UX comparable to Discord.

### Proposed Solution

Discool is a self-hosted, open-source communication platform that achieves full feature parity with Discord while being entirely community-owned and operated. Key architectural pillars:

- **Self-hosted & sovereign:** Run your own instance on your own hardware or VPS. Your data, your rules.
- **P2P instance discovery:** Instances find each other through distributed protocols (DHT or similar), with no central registry or directory. No single entity controls the network.
- **No instance-level blocking:** The network is uncensorable at the infrastructure level. Moderation is user-level (personal blocks) and guild-level (bans, roles, permissions) — exactly like Discord.
- **Multi-guild hosting:** Each instance can host multiple guilds, just like Discord's server model.
- **Horizontal clustering:** Instances can form clusters across multiple machines for scaling and high availability.
- **Resource-only limits:** No artificial caps. Your hardware is your limit.
- **Full real-time media:** Voice channels, video calls, screen sharing, and one-to-many streaming (Go Live) — all built on WebRTC with SFU scaling for larger audiences.
- **Rust backend:** High performance, memory safety, and low resource consumption — directly addressing the performance issues plaguing Python-based alternatives.
- **Svelte 5 SPA frontend:** Modern, reactive, fast UI built with Vite.

### Key Differentiators

| Differentiator | Why It Matters |
|---|---|
| **All-in-one platform** | Only OSS solution combining chat + forums + voice + video + streaming in a Discord-like UX |
| **Censorship-resistant by design** | No instance-level blocking; avoids Mastodon's defederation problem |
| **True P2P discovery** | No central directory = no single point of failure, no gatekeeping |
| **Rust performance** | Orders of magnitude faster than Python/Node alternatives; runs well on minimal hardware |
| **Zero artificial limits** | Scale limited only by your infrastructure, not by corporate pricing tiers |
| **Full self-sovereignty** | Complete data ownership, no corporate dependency, no de-platforming risk |

---

## Target Users

### Primary Users

#### 1. Community Builders — "The Architects"

**Persona: Maya, 28, Open Source Project Lead**
Maya maintains a popular open-source project. Her community currently lives on Discord, but she's uncomfortable that her community's entire history, knowledge base, and relationships are locked inside a proprietary platform she doesn't control. She's seen other projects get disrupted by policy changes and API restrictions. She wants to host her own communication hub that she controls completely.

- **Motivation:** Sovereignty over her community's home. No corporate landlord.
- **Current workaround:** Discord, reluctantly. Supplements with Matrix bridge for the principled users, which fragments the community.
- **Success vision:** A single platform her community actually *prefers* to Discord, where she controls the rules, the data, and the future.
- **Key need:** Easy guild setup, powerful role/permission management, bot/integration support.

#### 2. Everyday Users — "The Residents"

**Persona: Liam, 22, Gamer and Student**
Liam doesn't think about privacy or self-hosting. He's in 15+ Discord servers. He wants voice chat that doesn't lag, text channels with persistent history, and the ability to share screens while gaming. He'd switch to Discool if someone he trusts says "join here" and it *just works* — as smooth as Discord, maybe smoother, with zero friction.

- **Motivation:** Reliability. Speed. No Nitro paywall for basic features like higher upload limits or stream quality.
- **Current workaround:** Discord free tier, frustrated by quality caps and boost requirements.
- **Success vision:** "It's like Discord but everything just works and I don't have to pay for stuff that should be free."
- **Pain point with alternatives:** Matrix feels like a dead library with too many rooms. Discord improved but still overwhelms new users with too many channels. Liam wants to feel the *life* — "something is happening here" — without being buried in a wall of channels he doesn't care about.
- **Key need:** Frictionless onboarding (click an invite link, create portable identity, you're in), smooth voice/video/streaming, persistent chat history.

#### 3. Instance Operators — "The Builders"

**Persona: Tomás, 35, Sysadmin and Self-Hosting Enthusiast**
Tomás runs his own email, Nextcloud, and Gitea. He's been waiting for a Discord alternative that's actually *performant* and *self-hostable*. He tried Matrix/Synapse and gave up when it ate 4GB of RAM for 20 users. He wants something in Rust that runs lean on a $5/month VPS and scales horizontally when he needs it.

- **Motivation:** Infrastructure sovereignty. Performance. The joy of running your own stack.
- **Current workaround:** Matrix/Conduit (too limited) or just tolerating Discord.
- **Success vision:** A Discool instance running on a 2 vCPU / 2GB VPS serving his friend group and local community flawlessly. Upgrades the hardware when the community grows.
- **Key need:** Easy deployment (Docker, single binary), low resource footprint, clear documentation, clustering when scaling up.

#### 4. Privacy & Freedom Advocates — "The Principled"

**Persona: Aisha, 40, Digital Rights Activist**
Aisha works with vulnerable communities — journalists, dissidents, activists in hostile political environments. She needs communication infrastructure that no government or corporation can shut down, surveil, or censor. She's watched centralized platforms be weaponized — accounts banned, communities erased, data subpoenaed. The global push toward surveillance, digital ID mandates, and authoritarian control of social media makes this existentially urgent.

- **Motivation:** Censorship resistance. Protecting vulnerable people. Digital freedom in an era of rising authoritarianism.
- **Current workaround:** A fragmented mess of Signal, Matrix, IRC, and Tor-based tools, none of which provide the full community experience.
- **Success vision:** A resilient communication network where communities can exist without permission, with portable identities and no central kill switch.
- **Key need:** P2P discovery (no central directory to seize), quantum-proof portable identity, E2E encryption options, no instance-level blocking.

### Secondary Users

#### 5. Businesses & Organizations

Companies and organizations that need private, controlled communication infrastructure — either for regulatory compliance (GDPR, data residency), security requirements, or simply the desire to own their tooling. Guilds don't need to be public; invite-only private guilds within a self-hosted instance serve this use case perfectly.

- **Key need:** Private instances, fine-grained permissions, audit logging, compliance controls.

#### 6. Developers & Integrators

Bot builders, integration developers, and contributors to the Discool ecosystem. They need open, well-documented APIs that aren't artificially rate-limited or restricted by corporate priorities. The open-source nature means they can extend the platform however they need.

- **Key need:** Comprehensive API, bot framework, plugin/extension system, open-source contribution pathways.

### User Journey

| Stage | Experience |
|---|---|
| **Discovery** | User learns about Discool through word of mouth, a project website, tech communities, or by clicking a guild invite link shared by a friend or community. |
| **Onboarding** | User clicks an invite link → the web client loads → they create a portable cryptographic identity (quantum-proof keypair, OIDC-compatible) → they're in the guild immediately. Same identity works across any Discool instance. |
| **Core Usage** | Day-to-day chatting in text channels, dropping into voice channels, watching streams, participating in forum threads. Persistent history means nothing is lost. The UI surfaces activity — live conversations, active voice channels, ongoing streams — so the space always feels alive without being overwhelming. Smart channel organization and progressive disclosure mean users see what matters, not a flat list of 200 channels. It feels like Discord but without the paywalls or the "where do I even go?" problem. |
| **"Aha!" Moment** | "It just works." Voice doesn't lag. Streaming quality isn't gated. Chat history goes back forever. No Nitro upsells. It's *at least* as good as Discord — and it's free, open, and nobody can take it away. |
| **Long-term** | The user's portable identity becomes their communication home. They join guilds across multiple instances. Some power users start hosting their own instances. Community builders migrate their communities from Discord. The network grows organically through P2P discovery. |

---

## Success Metrics

### Vision of Success

Discool succeeds when it becomes the communication platform people *choose* and *recommend* — when invite links are being shared organically, when communities are building homes on it, and when "join our Discool" becomes as natural as "join our Discord." Not because of marketing spend, but because it genuinely works better for people who value freedom, reliability, and community ownership.

### User Success Metrics

| Metric | What It Measures | How We Know |
|---|---|---|
| **Organic invite sharing** | Users are actively bringing others to the platform | Invite link generation and redemption rates grow month-over-month |
| **Community migration** | Groups are choosing Discool over Discord for new communities, or migrating existing ones | New guild creation rate; Discord-to-Discool migration tooling usage |
| **Daily active engagement** | Users are coming back because the platform is where their community lives | Daily active users per instance; messages sent; voice channel minutes |
| **Instance reliability** | Operators can run Discool without headaches on modest hardware | Uptime metrics; resource consumption per user; support issue volume |
| **Real-time media quality** | Voice, video, and streaming work as well as or better than Discord | Latency benchmarks; user-reported quality scores; stream viewer retention |
| **"It just works" satisfaction** | The UX is smooth enough that non-technical users don't notice the infrastructure | Onboarding completion rate; time from invite click to first message |

### Business Objectives

Discool is not a commercial product — it's an open-source project with a sustainability model. Business objectives serve the mission, not the other way around.

| Timeframe | Objective |
|---|---|
| **Year 1** | Working product that the founding community uses daily. Core features stable (text, voice, video, streaming, guilds, roles). At least a few independent instances running. Early adopter feedback loop established. |
| **Year 2** | Growing ecosystem. Multiple independent instances with real communities. Managed hosting service operational and generating revenue to fund development. Contributor community forming. |
| **Year 3** | Discool is a recognized name in the self-hosted / open-source space. Organic adoption is visible — people share invite links, tech media covers it, communities recommend it. Donations and managed hosting sustain ongoing development. |

### Key Performance Indicators

**Adoption KPIs:**
- Number of known active instances (via opt-in telemetry or P2P network visibility)
- Total guilds hosted across the network
- Monthly invite link redemptions (new user joins)
- GitHub stars, forks, and contributor count

**Engagement KPIs:**
- Daily/weekly active users per instance
- Messages sent per day
- Voice/video minutes per day
- User retention at 7, 30, and 90 days

**Sustainability KPIs:**
- Monthly donation/sponsorship revenue
- Managed hosting subscribers and revenue
- Development velocity (releases, bug fixes, feature delivery)
- Community contributor growth (PRs, issues, discussions)

**Quality KPIs:**
- Voice/video latency (p50, p95)
- Instance resource consumption per concurrent user
- Crash/error rates
- Time from invite click to first message sent

### Revenue Model

- **Primary (mission):** Pure open-source, community-funded through donations and sponsorships
- **Secondary (sustainability):** Managed Discool hosting — turnkey instances for communities that don't want to self-host
- **Philosophy:** Revenue funds development. The product remains fully open-source. No open-core gatekeeping — managed hosting sells convenience, not features.

---

## MVP Scope

### Core Features (v1)

**Architecture Foundation (Priority: Get It Right)**
- P2P instance discovery — instances find each other without a central directory
- Portable cryptographic identity — users create a keypair-based identity that works across all instances (OIDC-compatible, algorithm-agile for future quantum-proofing)
- Multi-guild hosting — a single instance can host multiple guilds
- Modern cryptography (Ed25519/X25519) with architecture designed for future quantum-proof algorithm swap

**Communication Features**
- Text channels with persistent message history — the core promise
- Voice channels via WebRTC — drop-in/drop-out group voice
- Direct messages between users
- File sharing and rich embeds (images, links)

**Community Management**
- Guild creation and management
- Role-based permission system (Discord-equivalent: roles, channel-level overrides, hierarchical permissions)
- Invite link system — generate and share invite links to guilds
- User blocking (personal) and guild-level bans/kicks
- No instance-level blocking (by design)

**User Experience**
- Clean, modern Svelte 5 SPA that doesn't overwhelm
- Activity-driven UI — surfaces where the life is, not a flat channel list
- Progressive disclosure — discover more as you explore
- Frictionless onboarding — click invite link → create identity → chatting

**Infrastructure**
- Rust backend optimized for low resource consumption
- Single-binary or Docker deployment for instance operators
- Runs well on minimal hardware (2 vCPU / 2GB RAM for small communities)

### Out of Scope for MVP

| Feature | Rationale | Target Phase |
|---|---|---|
| Video calls & screen sharing | Complex WebRTC scaling; voice is sufficient for MVP | Phase 2 |
| One-to-many streaming (Go Live) | Requires SFU infrastructure; additive to the core | Phase 2-3 |
| Horizontal clustering | Single-node performance with Rust should handle MVP scale; clustering is a scaling optimization | Phase 2 |
| Public bot/plugin API | Ecosystem play — needs communities first. Internal API designed for future exposure. | Phase 2 |
| Forum/thread channels | Valuable but additive; text channels cover core real-time chat need | Phase 2 |
| Quantum-proof cryptography | Architecture is algorithm-agile; standard modern crypto is secure for now | Phase 2-3 |
| E2E encryption | Important for privacy users, but adds significant complexity; design for it, ship later | Phase 2 |
| Stage channels, activities, stickers/emoji marketplace | Polish features for mature platform | Phase 3+ |

### MVP Success Criteria

| Criteria | Validation Signal |
|---|---|
| **"I hosted my own and it works"** | Instance operators can deploy on a $5 VPS and serve a small community reliably |
| **"I joined and it felt like Discord"** | Non-technical users join via invite link and start chatting without confusion |
| **"The UI is nice and I'm not lost"** | Users navigate intuitively; activity surfaces naturally; no channel overload |
| **Voice just works** | Voice channels perform with low latency and clear audio |
| **P2P discovery works** | Instances find each other without manual configuration or central registry |
| **Identity is portable** | A user can join guilds on different instances with the same identity |
| **Architecture validates** | The P2P, identity, and guild architecture can support future features without rewrites |

### Future Vision

**Phase 2: Rich Media & Ecosystem**
- Video calls and screen sharing in voice channels
- Forum/thread channels for async discussion
- Public bot/plugin API with developer documentation
- Horizontal clustering for scaling
- E2E encryption (opt-in per channel)
- Discord import/migration tooling

**Phase 3: Full Platform**
- One-to-many streaming (Go Live)
- Quantum-proof cryptographic identity upgrade
- Stage channels for events
- App/integration directory
- Custom emoji and sticker support
- Mobile apps (if demand warrants beyond responsive SPA)

**Long-term Vision:**
Discool becomes the default open-source communication platform — the thing people recommend when someone asks "where should our community live?" Feature parity with Discord, but owned by nobody and everybody. A network of thousands of instances, discoverable through P2P, where communities thrive without permission.
