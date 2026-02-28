import DOMPurify from 'dompurify'
import hljs from 'highlight.js'
import { Marked } from 'marked'

const MAX_MARKDOWN_CACHE_ENTRIES = 400

const markdown = new Marked({
  gfm: true,
  breaks: true,
})

const SAFE_LANGUAGE_TOKEN = /^[a-z0-9-]+$/i

markdown.use({
  renderer: {
    code(token) {
      const code = token.text ?? ''
      const language = normalizeHighlightLanguage(token.lang ?? '')
      const highlighted = language
        ? hljs.highlight(code, { language, ignoreIllegals: true }).value
        : hljs.highlightAuto(code).value
      const className = language ? `hljs language-${language}` : 'hljs'
      return `<pre><code class="${className}">${highlighted}</code></pre>`
    },
    link(token) {
      const href = token.href ?? ''
      const title = token.title ?? ''
      const label = escapeHtml(token.text ?? href)
      if (!isSafeExternalUrl(href)) {
        return label
      }
      const rel = 'noopener noreferrer'
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : ''
      return `<a href="${escapeHtml(href)}"${titleAttr} target="_blank" rel="${rel}">${label}</a>`
    },
  },
})

const markdownCache = new Map<string, string>()

export function renderMessageMarkdown(content: string): string {
  const source = content.trim()
  if (!source) return ''

  const cached = markdownCache.get(source)
  if (cached !== undefined) return cached

  const rendered = markdown.parse(source) as string
  const sanitized = DOMPurify.sanitize(rendered, {
    USE_PROFILES: { html: true },
    ALLOW_UNKNOWN_PROTOCOLS: false,
  })
  markdownCache.set(source, sanitized)
  if (markdownCache.size > MAX_MARKDOWN_CACHE_ENTRIES) {
    const first = markdownCache.keys().next().value
    if (first) {
      markdownCache.delete(first)
    }
  }
  return sanitized
}

function normalizeHighlightLanguage(value: string): string {
  const normalized = value.trim().toLowerCase()
  if (!normalized || !SAFE_LANGUAGE_TOKEN.test(normalized)) {
    return ''
  }
  return hljs.getLanguage(normalized) ? normalized : ''
}

export function isSafeExternalUrl(value: string): boolean {
  const trimmed = value.trim()
  if (!trimmed) return false
  try {
    const parsed = new URL(trimmed)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')
      return false
    return !isPrivateOrLocalHost(parsed.hostname)
  } catch {
    return false
  }
}

function isPrivateOrLocalHost(hostname: string): boolean {
  const normalized = hostname.trim().toLowerCase()
  if (!normalized) return true
  const host =
    normalized.startsWith('[') && normalized.endsWith(']')
      ? normalized.slice(1, -1)
      : normalized
  if (
    host === 'localhost' ||
    host.endsWith('.localhost') ||
    host.endsWith('.local')
  ) {
    return true
  }

  const mappedMatch = host.match(/^::ffff:(\d{1,3}(?:\.\d{1,3}){3})$/)
  if (mappedMatch) {
    const mappedIpv4 = parseIpv4Octets(mappedMatch[1])
    return mappedIpv4 ? isPrivateOrReservedIpv4(mappedIpv4) : true
  }

  if (host.includes(':')) {
    return (
      host === '::' ||
      host === '::1' ||
      host.startsWith('fc') ||
      host.startsWith('fd') ||
      host.startsWith('fe80:')
    )
  }

  const ipv4 = parseIpv4Octets(host)
  return ipv4 ? isPrivateOrReservedIpv4(ipv4) : false
}

function parseIpv4Octets(
  hostname: string,
): [number, number, number, number] | null {
  const parts = hostname.split('.')
  if (parts.length !== 4) return null

  const octets = parts.map((part) => {
    if (!/^\d{1,3}$/.test(part)) return null
    const value = Number.parseInt(part, 10)
    if (!Number.isInteger(value) || value < 0 || value > 255) return null
    return value
  })
  if (octets.some((value) => value === null)) return null

  return octets as [number, number, number, number]
}

function isPrivateOrReservedIpv4([a, b, c]: [
  number,
  number,
  number,
  number,
]): boolean {
  if (a === 0 || a === 10 || a === 127) return true
  if (a === 169 && b === 254) return true
  if (a === 172 && b >= 16 && b <= 31) return true
  if (a === 192 && b === 168) return true
  if (a === 100 && b >= 64 && b <= 127) return true
  if (a === 198 && (b === 18 || b === 19)) return true
  if (a === 192 && b === 0 && c === 2) return true
  if (a === 198 && b === 51 && c === 100) return true
  if (a === 203 && b === 0 && c === 113) return true
  if (a >= 224) return true
  return false
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}
