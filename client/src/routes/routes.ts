import type { RouteConfig } from '@mateothegreat/svelte5-router'

export type ShellMode = 'home' | 'channel' | 'dm' | 'settings' | 'admin'

const CHANNEL_LOCATION_RE = /^\/(?!dm\/)([^/]+)\/([^/]+)$/
const DM_LOCATION_RE = /^\/dm\/[^/]+$/

const shellComponent = () => import('$lib/features/shell/ShellRoute.svelte')

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
    shellRoute(/^\/$/, 'home'),
    shellRoute(/^\/(?<fallback>.*)$/, 'home'),
  ]

  if (isAdmin) {
    routes.unshift(shellRoute('/admin', 'admin'))
  }

  return routes
}
