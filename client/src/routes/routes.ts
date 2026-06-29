import type { RouteConfig } from '@mateothegreat/svelte5-router'

export type ShellMode = 'home' | 'channel' | 'dm' | 'settings' | 'admin'

const CHANNEL_LOCATION_RE = /^\/(?!dm\/)([^/]+)\/([^/]+)$/
const DM_LOCATION_RE = /^\/dm\/[^/]+$/

// Must be `async`: @mateothegreat/svelte5-router only awaits the dynamic import
// when the loader's constructor.name === "AsyncFunction". A plain `() => import`
// is a "Function", so the router would assign the loader itself as the component
// and render nothing.
const shellComponent = async () => import('$lib/features/shell/ShellRoute.svelte')

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
  const routes = [
    shellRoute('/settings', 'settings'),
    shellRoute('/dm/:dm', 'dm'),
    shellRoute('/:guild/:channel', 'channel'),
    // Use a plain "/" (not a /^\/$/ regex) so the router recognizes this as its
    // default route: with no basePath, the router normalizes "/" to "" before
    // matching, no regex matches "", and a non-string path isn't in the
    // router's default-route list — so "/" would otherwise render nothing.
    shellRoute('/', 'home'),
    shellRoute(/^\/(?<fallback>.*)$/, 'home'),
  ]

  if (isAdmin) {
    routes.unshift(shellRoute('/admin', 'admin'))
  }

  return routes
}
