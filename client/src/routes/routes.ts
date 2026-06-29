import type { RouteConfig } from '@mateothegreat/svelte5-router'

export type ShellMode = 'home' | 'channel' | 'dm' | 'settings' | 'admin'

const CHANNEL_LOCATION_RE = /^\/(?!dm\/)([^/]+)\/([^/]+)$/
const DM_LOCATION_RE = /^\/dm\/[^/]+$/

// Must be `async`: @mateothegreat/svelte5-router only awaits the dynamic import
// when the loader's constructor.name === "AsyncFunction". A plain `() => import`
// is a "Function", so the router would assign the loader itself as the component
// and render nothing.
const shellComponent = async () =>
  import('$lib/features/shell/ShellRoute.svelte')

function normalizePath(path: string): string {
  const [pathname] = path.trim().split('?')
  if (!pathname) return '/'
  return pathname.startsWith('/') ? pathname : `/${pathname}`
}

export function isPersistableLocation(path: string): boolean {
  const normalizedPath = normalizePath(path)
  if (
    normalizedPath === '/' ||
    normalizedPath.startsWith('/admin') ||
    normalizedPath.startsWith('/settings')
  ) {
    return false
  }
  if (DM_LOCATION_RE.test(normalizedPath)) return true
  return CHANNEL_LOCATION_RE.test(normalizedPath)
}

export function resolveInitialLocation(
  currentPath: string,
  lastLocation: string | null | undefined,
): string {
  const normalizedCurrentPath = normalizePath(currentPath)
  if (normalizedCurrentPath !== '/') return normalizedCurrentPath
  if (!lastLocation) return '/'
  const normalizedLastLocation = normalizePath(lastLocation)
  return isPersistableLocation(normalizedLastLocation)
    ? normalizedLastLocation
    : '/'
}

function shellRoute(path: RouteConfig['path'], mode: ShellMode): RouteConfig {
  return {
    path,
    component: shellComponent,
    props: { mode },
  }
}

export function parsePersistedChannelLocation(
  path: string,
): { guild: string; channel: string } | null {
  const normalizedPath = normalizePath(path)
  const match = normalizedPath.match(CHANNEL_LOCATION_RE)
  if (!match?.[1] || !match?.[2]) return null
  return {
    guild: match[1],
    channel: match[2],
  }
}

export function createAuthenticatedRoutes(isAdmin: boolean): RouteConfig[] {
  // @mateothegreat/svelte5-router resolves the LAST matching route — its match
  // loop tests every route and overwrites the candidate without breaking, so
  // the final match wins. A catch-all regex matches every path, so if it's last
  // it silently overrides /settings, /dm/:dm, /:guild/:channel (everything
  // resolves to home). Order routes generic -> specific: the catch-all is first
  // (overwritten by anything more specific that also matches), patterns next
  // (/:guild/:channel before /dm/:dm so /dm/x resolves to dm), and exact routes
  // last so they win. Admin is pushed (last), not unshifted.
  const routes = [
    shellRoute(/^\/(?<fallback>.*)$/, 'home'),
    shellRoute('/', 'home'),
    // Regex named groups, not ":param": @mateothegreat/svelte5-router treats a
    // string path as a literal (or regex if it has metacharacters); ":guild" has
    // no metacharacters, so "/:guild/:channel" would only match that exact
    // literal and never a real path like /lobby-2/general. Named groups are
    // populated into route.params (guild/channel/dm) by the regexp evaluator.
    shellRoute(/^\/(?<guild>[^/]+)\/(?<channel>[^/]+)$/, 'channel'),
    shellRoute(/^\/dm\/(?<dm>[^/]+)$/, 'dm'),
    shellRoute('/settings', 'settings'),
  ]

  if (isAdmin) {
    routes.push(shellRoute('/admin', 'admin'))
  }

  return routes
}
