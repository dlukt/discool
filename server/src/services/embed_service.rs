use std::{
    collections::HashSet,
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::OnceLock,
    time::Duration,
};

use chrono::Utc;
use regex::Regex;
use reqwest::{Client, header::CONTENT_TYPE, redirect::Policy};
use scraper::{Html, Selector};
use url::Url;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::message_embed::{self, EmbedUrlCache},
};

const MAX_EMBEDS_PER_MESSAGE: usize = 5;
const FETCH_TIMEOUT_SECS: u64 = 4;
const MAX_FETCH_BYTES: usize = 256 * 1024;
const MAX_TITLE_CHARS: usize = 280;
const MAX_DESCRIPTION_CHARS: usize = 560;
const MAX_DOMAIN_CHARS: usize = 128;

const EMBED_FETCH_USER_AGENT: &str = "discool-embed-fetcher/1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FetchedEmbed {
    normalized_url: String,
    title: Option<String>,
    description: Option<String>,
    thumbnail_url: Option<String>,
    domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedUrlCandidate {
    original_url: String,
    normalized_url: String,
}

type FetchMetadataFuture =
    Pin<Box<dyn Future<Output = Result<Option<FetchedEmbed>, AppError>> + Send>>;

pub async fn sync_message_embeds(
    pool: &DbPool,
    message_id: &str,
    normalized_message_content: &str,
) -> Result<(), AppError> {
    sync_message_embeds_with_fetcher(
        pool,
        message_id,
        normalized_message_content,
        &|url: String| Box::pin(async move { fetch_embed_metadata(&url).await }),
    )
    .await
}

async fn sync_message_embeds_with_fetcher<F>(
    pool: &DbPool,
    message_id: &str,
    normalized_message_content: &str,
    fetcher: &F,
) -> Result<(), AppError>
where
    F: Fn(String) -> FetchMetadataFuture,
{
    message_embed::delete_message_embeds_by_message_id(pool, message_id).await?;
    let candidates = extract_url_candidates(normalized_message_content);
    if candidates.is_empty() {
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    for candidate in candidates {
        let cache = match message_embed::find_embed_url_cache_by_normalized_url(
            pool,
            &candidate.normalized_url,
        )
        .await?
        {
            Some(cache) => cache,
            None => {
                let Some(fetched) = fetcher(candidate.normalized_url.clone()).await? else {
                    continue;
                };
                message_embed::upsert_embed_url_cache(
                    pool,
                    &fetched.normalized_url,
                    fetched.title.as_deref(),
                    fetched.description.as_deref(),
                    fetched.thumbnail_url.as_deref(),
                    &fetched.domain,
                    &now,
                    &now,
                )
                .await?;
                EmbedUrlCache {
                    normalized_url: fetched.normalized_url,
                    title: fetched.title,
                    description: fetched.description,
                    thumbnail_url: fetched.thumbnail_url,
                    domain: fetched.domain,
                    fetched_at: now.clone(),
                    updated_at: now.clone(),
                }
            }
        };

        let inserted = message_embed::insert_message_embed(
            pool,
            &Uuid::new_v4().to_string(),
            message_id,
            &candidate.original_url,
            &cache.normalized_url,
            cache.title.as_deref(),
            cache.description.as_deref(),
            cache.thumbnail_url.as_deref(),
            &cache.domain,
            &now,
        )
        .await?;
        if !inserted {
            tracing::warn!(
                message_id = %message_id,
                normalized_url = %cache.normalized_url,
                "Skipped duplicate message embed insert"
            );
        }
    }
    Ok(())
}

async fn fetch_embed_metadata(url: &str) -> Result<Option<FetchedEmbed>, AppError> {
    let parsed = match Url::parse(url) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let Some(resolved_addrs) = resolve_public_fetch_addrs(&parsed).await else {
        tracing::warn!(url = %url, "Blocked embed fetch target due to SSRF safeguards");
        return Ok(None);
    };

    let host = parsed.host_str().unwrap_or_default();
    let client = build_http_client(host, &resolved_addrs)?;
    let response = match client
        .get(parsed.clone())
        .header("User-Agent", EMBED_FETCH_USER_AGENT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            tracing::debug!(error = ?err, url = %url, "Embed metadata fetch failed");
            return Ok(None);
        }
    };
    if !response.status().is_success() {
        return Ok(None);
    }

    if let Some(content_length) = response.content_length()
        && content_length > MAX_FETCH_BYTES as u64
    {
        return Ok(None);
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !content_type.to_ascii_lowercase().starts_with("text/html") {
        return Ok(None);
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::debug!(error = ?err, url = %url, "Failed to read embed response bytes");
            return Ok(None);
        }
    };
    if bytes.len() > MAX_FETCH_BYTES {
        return Ok(None);
    }

    let html = String::from_utf8_lossy(&bytes).to_string();
    Ok(extract_embed_metadata_from_html(&parsed, &html))
}

fn build_http_client(host: &str, resolved_addrs: &[SocketAddr]) -> Result<Client, AppError> {
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS));
    if host.parse::<IpAddr>().is_err() {
        builder = builder.resolve_to_addrs(host, resolved_addrs);
    }
    builder
        .build()
        .map_err(|err| AppError::Internal(format!("Failed to build embed HTTP client: {err}")))
}

async fn resolve_public_fetch_addrs(parsed: &Url) -> Option<Vec<SocketAddr>> {
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed.host_str()?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_public_ip(ip).then_some(vec![SocketAddr::new(ip, port)]);
    }

    let Ok(addrs) = tokio::net::lookup_host((host, port)).await else {
        return None;
    };
    let mut resolved = Vec::new();
    for addr in addrs {
        if !is_public_ip(addr.ip()) {
            return None;
        }
        resolved.push(addr);
    }
    if resolved.is_empty() {
        return None;
    }
    resolved.sort_unstable();
    resolved.dedup();
    Some(resolved)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                || is_ipv4_shared(v4)
                || is_ipv4_benchmarking(v4)
                || is_ipv4_this_network(v4)
                || is_ipv4_reserved(v4))
        }
        IpAddr::V6(v6) => {
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(mapped_v4));
            }
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || is_ipv6_unique_local(v6)
                || is_ipv6_link_local(v6)
                || is_ipv6_documentation(v6))
        }
    }
}

fn is_ipv4_shared(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_ipv4_benchmarking(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)
}

fn is_ipv4_this_network(ip: Ipv4Addr) -> bool {
    ip.octets()[0] == 0
}

fn is_ipv4_reserved(ip: Ipv4Addr) -> bool {
    ip.octets()[0] >= 240
}

fn is_ipv6_unique_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_ipv6_link_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn is_ipv6_documentation(ip: Ipv6Addr) -> bool {
    (ip.segments()[0], ip.segments()[1]) == (0x2001, 0x0db8)
}

fn extract_embed_metadata_from_html(parsed_url: &Url, html: &str) -> Option<FetchedEmbed> {
    let document = Html::parse_document(html);

    let title = pick_meta_content(&document, "meta[property=\"og:title\"]")
        .or_else(|| pick_meta_content(&document, "meta[name=\"twitter:title\"]"))
        .or_else(|| pick_title_text(&document))
        .and_then(|value| sanitize_text_field(&value, MAX_TITLE_CHARS));
    let description = pick_meta_content(&document, "meta[property=\"og:description\"]")
        .or_else(|| pick_meta_content(&document, "meta[name=\"description\"]"))
        .or_else(|| pick_meta_content(&document, "meta[name=\"twitter:description\"]"))
        .and_then(|value| sanitize_text_field(&value, MAX_DESCRIPTION_CHARS));
    let thumbnail_url = pick_meta_content(&document, "meta[property=\"og:image\"]")
        .or_else(|| pick_meta_content(&document, "meta[name=\"twitter:image\"]"))
        .and_then(|value| sanitize_url_field(&value));

    let domain = sanitize_domain(parsed_url.host_str().unwrap_or_default())?;
    if title.is_none() && description.is_none() && thumbnail_url.is_none() {
        return Some(FetchedEmbed {
            normalized_url: parsed_url.to_string(),
            title: Some(domain.clone()),
            description: None,
            thumbnail_url: None,
            domain,
        });
    }
    Some(FetchedEmbed {
        normalized_url: parsed_url.to_string(),
        title,
        description,
        thumbnail_url,
        domain,
    })
}

fn pick_title_text(document: &Html) -> Option<String> {
    let selector = selector("title");
    document
        .select(&selector)
        .next()
        .map(|item| item.text().collect::<String>())
}

fn pick_meta_content(document: &Html, css: &str) -> Option<String> {
    let selector = selector(css);
    document
        .select(&selector)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(ToString::to_string)
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("valid embed metadata selector")
}

fn sanitize_text_field(value: &str, max_chars: usize) -> Option<String> {
    let normalized = value
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let condensed = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if condensed.is_empty() {
        return None;
    }

    let mut result = String::new();
    for ch in condensed.chars().take(max_chars) {
        result.push(ch);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn sanitize_url_field(value: &str) -> Option<String> {
    normalize_url(value)
}

fn sanitize_domain(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    let mut result = String::new();
    for ch in trimmed.chars().take(MAX_DOMAIN_CHARS) {
        if ch.is_control() {
            continue;
        }
        result.push(ch);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn extract_url_candidates(content: &str) -> Vec<ParsedUrlCandidate> {
    let markdown_link_regex = markdown_link_regex();
    let bare_url_regex = bare_url_regex();
    let mut resolved = Vec::new();
    let mut seen = HashSet::<String>::new();

    for capture in markdown_link_regex.captures_iter(content) {
        let Some(raw_match) = capture.get(1) else {
            continue;
        };
        if let Some(candidate) = parse_candidate_url(raw_match.as_str())
            && seen.insert(candidate.normalized_url.clone())
        {
            resolved.push(candidate);
        }
        if resolved.len() >= MAX_EMBEDS_PER_MESSAGE {
            return resolved;
        }
    }

    for matched in bare_url_regex.find_iter(content) {
        if let Some(candidate) = parse_candidate_url(matched.as_str())
            && seen.insert(candidate.normalized_url.clone())
        {
            resolved.push(candidate);
        }
        if resolved.len() >= MAX_EMBEDS_PER_MESSAGE {
            break;
        }
    }

    resolved
}

fn markdown_link_regex() -> &'static Regex {
    static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
    MARKDOWN_LINK_RE.get_or_init(|| {
        Regex::new(r"\[[^\]]+\]\((https?://[^)\s]+)\)").expect("valid markdown link regex")
    })
}

fn bare_url_regex() -> &'static Regex {
    static BARE_URL_RE: OnceLock<Regex> = OnceLock::new();
    BARE_URL_RE.get_or_init(|| Regex::new(r"https?://[^\s<>()]+").expect("valid URL regex"))
}

fn parse_candidate_url(raw: &str) -> Option<ParsedUrlCandidate> {
    let normalized = normalize_url(raw)?;
    Some(ParsedUrlCandidate {
        original_url: normalized.clone(),
        normalized_url: normalized,
    })
}

fn normalize_url(raw: &str) -> Option<String> {
    let decoded = decode_basic_html_entities(raw.trim());
    let stripped_wrapping = decoded
        .trim_matches(|ch| matches!(ch, '<' | '>' | '"' | '\'' | '`'))
        .trim_end_matches(['.', ',', '!', '?', ';', ':']);
    if stripped_wrapping.is_empty() {
        return None;
    }

    let mut parsed = Url::parse(stripped_wrapping).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    if parsed.host_str()?.trim().is_empty() {
        return None;
    }
    if parsed.scheme() == "http" && parsed.port_or_known_default() == Some(80) {
        let _ = parsed.set_port(None);
    }
    if parsed.scheme() == "https" && parsed.port_or_known_default() == Some(443) {
        let _ = parsed.set_port(None);
    }
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

fn decode_basic_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use crate::{
        config::DatabaseConfig,
        db::{init_pool, run_migrations},
        models::message,
    };

    async fn setup_embed_pool() -> DbPool {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        seed_embed_fixture(&pool).await;
        pool
    }

    async fn seed_embed_fixture(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("embed service tests expect sqlite pool");
        };
        let created_at = "2026-02-28T00:00:00Z";

        sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("author-user-id")
        .bind("did:key:z6MkAuthor")
        .bind("zAuthor")
        .bind("author-user")
        .bind("#3366ff")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
        )
        .bind("guild-id")
        .bind("embed-guild")
        .bind("Embed Guild")
        .bind("author-user-id")
        .bind("general")
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-id")
        .bind("guild-id")
        .bind("general")
        .bind("general")
        .bind("text")
        .bind(0_i64)
        .bind(created_at)
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        message::insert_message(
            &DbPool::Sqlite(pool.clone()),
            "message-1",
            "guild-id",
            "channel-id",
            "author-user-id",
            "hello",
            false,
            "2026-02-28T00:00:01Z",
            "2026-02-28T00:00:01Z",
        )
        .await
        .unwrap();
        message::insert_message(
            &DbPool::Sqlite(pool.clone()),
            "message-2",
            "guild-id",
            "channel-id",
            "author-user-id",
            "hello-2",
            false,
            "2026-02-28T00:00:02Z",
            "2026-02-28T00:00:02Z",
        )
        .await
        .unwrap();
    }

    #[test]
    fn extract_url_candidates_deduplicates_equivalent_urls() {
        let content = "See https://example.com/path?a=1&amp;b=2 and [link](https://example.com/path?a=1&b=2#frag)";
        let urls = extract_url_candidates(content);
        assert_eq!(urls.len(), 1);
        assert_eq!(
            urls[0].normalized_url,
            "https://example.com/path?a=1&b=2".to_string()
        );
    }

    #[test]
    fn normalize_url_rejects_unsafe_schemes() {
        assert!(normalize_url("javascript:alert(1)").is_none());
        assert!(normalize_url("data:text/plain;base64,SGk=").is_none());
        assert_eq!(
            normalize_url("https://example.com/path#fragment").as_deref(),
            Some("https://example.com/path")
        );
    }

    #[test]
    fn is_public_ip_rejects_private_and_loopback_targets() {
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(0, 1, 2, 3))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(198, 18, 0, 1))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1))));
        assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
        assert!(!is_public_ip(
            "::ffff:127.0.0.1".parse().expect("valid mapped loopback"),
        ));
        assert!(!is_public_ip(
            "::ffff:10.0.0.1"
                .parse()
                .expect("valid mapped private address"),
        ));
        assert!(is_public_ip(
            "::ffff:93.184.216.34"
                .parse()
                .expect("valid mapped public address"),
        ));
    }

    #[tokio::test]
    async fn resolve_public_fetch_addrs_rejects_private_ip_literal_targets() {
        let parsed = Url::parse("http://127.0.0.1/path").expect("valid url");
        assert!(resolve_public_fetch_addrs(&parsed).await.is_none());
    }

    #[tokio::test]
    async fn resolve_public_fetch_addrs_keeps_public_ip_literal_targets() {
        let parsed = Url::parse("https://93.184.216.34/path").expect("valid url");
        let addrs = resolve_public_fetch_addrs(&parsed)
            .await
            .expect("expected resolved addrs");
        assert_eq!(
            addrs,
            vec![SocketAddr::new("93.184.216.34".parse().unwrap(), 443)]
        );
    }

    #[tokio::test]
    async fn sync_message_embeds_uses_cache_without_refetching() {
        let pool = setup_embed_pool().await;
        message_embed::upsert_embed_url_cache(
            &pool,
            "https://example.com/article",
            Some("Example title"),
            Some("Example description"),
            Some("https://example.com/image.png"),
            "example.com",
            "2026-02-28T00:00:03Z",
            "2026-02-28T00:00:03Z",
        )
        .await
        .unwrap();

        let fetch_calls = Arc::new(AtomicUsize::new(0));
        let calls_ref = Arc::clone(&fetch_calls);
        sync_message_embeds_with_fetcher(
            &pool,
            "message-1",
            "https://example.com/article",
            &|_url: String| -> FetchMetadataFuture {
                let calls_ref = Arc::clone(&calls_ref);
                Box::pin(async move {
                    calls_ref.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                })
            },
        )
        .await
        .unwrap();

        assert_eq!(fetch_calls.load(Ordering::SeqCst), 0);
        let embeds =
            message_embed::list_message_embeds_by_message_ids(&pool, &["message-1".to_string()])
                .await
                .unwrap();
        let message_embeds = embeds.get("message-1").expect("message-1 embeds missing");
        assert_eq!(message_embeds.len(), 1);
        assert_eq!(message_embeds[0].title.as_deref(), Some("Example title"));
        assert_eq!(message_embeds[0].domain, "example.com");
    }

    #[tokio::test]
    async fn sync_message_embeds_caches_fetched_metadata_for_equivalent_urls() {
        let pool = setup_embed_pool().await;

        let fetch_calls = Arc::new(AtomicUsize::new(0));
        let calls_ref = Arc::clone(&fetch_calls);
        let fetcher = |url: String| -> FetchMetadataFuture {
            let calls_ref = Arc::clone(&calls_ref);
            Box::pin(async move {
                calls_ref.fetch_add(1, Ordering::SeqCst);
                Ok(Some(FetchedEmbed {
                    normalized_url: url,
                    title: Some("Fetched &lt;b&gt;Title&lt;/b&gt;".to_string()),
                    description: Some("Fetched description".to_string()),
                    thumbnail_url: Some("https://example.com/thumb.png".to_string()),
                    domain: "example.com".to_string(),
                }))
            })
        };

        sync_message_embeds_with_fetcher(
            &pool,
            "message-1",
            "https://example.com/item?x=1#frag",
            &fetcher,
        )
        .await
        .unwrap();
        sync_message_embeds_with_fetcher(
            &pool,
            "message-2",
            "[again](https://example.com/item?x=1)",
            &fetcher,
        )
        .await
        .unwrap();

        assert_eq!(fetch_calls.load(Ordering::SeqCst), 1);
        let cache = message_embed::find_embed_url_cache_by_normalized_url(
            &pool,
            "https://example.com/item?x=1",
        )
        .await
        .unwrap()
        .expect("expected cached metadata");
        assert_eq!(
            cache.thumbnail_url.as_deref(),
            Some("https://example.com/thumb.png")
        );
    }
}
