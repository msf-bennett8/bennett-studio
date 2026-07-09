/**
 * Bennett Studio Host Resolver
 * Resolves share code to host endpoint via multiple strategies
 */

export interface ResolvedHost {
  baseUrl: string;
  resolvedAt: string;
  ttlSeconds: number;
}

const DNS_CACHE = new Map<string, ResolvedHost>();
const DEFAULT_TTL = 300; // 5 minutes
const RESOLVER_TIMEOUT = 5000; // 5 seconds

/**
 * Resolve host for a share code
 * Strategy:
 * 1. Check memory cache
 * 2. Check localStorage (browser) / env (node)
 * 3. Extract from JWT token (self-describing token)
 * 4. Well-known endpoint (cloud resolver, for future)
 * 5. Fallback to default pattern
 */
export async function resolveHost(code: string, token?: string): Promise<string> {
  // 1. Check memory cache
  const cached = DNS_CACHE.get(code);
  if (cached) {
    const expiresAt = new Date(cached.resolvedAt).getTime() + cached.ttlSeconds * 1000;
    if (Date.now() < expiresAt) {
      return cached.baseUrl;
    }
  }
  
  // 2. Check persistent cache
  const persistent = getPersistentCache(code);
  if (persistent) {
    DNS_CACHE.set(code, persistent);
    return persistent.baseUrl;
  }
  
  // 3. Extract host from JWT token (self-describing token)
  if (token) {
    try {
      const parts = token.split('.');
      if (parts.length === 3) {
        const base64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
        const padLen = (4 - (base64.length % 4)) % 4;
        const padded = base64 + '='.repeat(padLen);
        const payload = JSON.parse(atob(padded));
        
        if (payload.host && payload.port) {
          const resolved: ResolvedHost = {
            baseUrl: `http://${payload.host}:${payload.port}`,
            resolvedAt: new Date().toISOString(),
            ttlSeconds: DEFAULT_TTL,
          };
          setPersistentCache(code, resolved);
          DNS_CACHE.set(code, resolved);
          return resolved.baseUrl;
        }
      }
    } catch {
      // Token doesn't contain host info
    }
  }

  // 4. Try resolver endpoint (cloud, for future)
  try {
    const resolved = await fetchResolver(code);
    if (resolved) {
      setPersistentCache(code, resolved);
      DNS_CACHE.set(code, resolved);
      return resolved.baseUrl;
    }
  } catch {
    // Resolver unavailable, continue to fallback
  }

  // 5. Fallback to pattern-based URL
  const fallback: ResolvedHost = {
    baseUrl: `https://${code.toLowerCase()}.share.bennett.studio`,
    resolvedAt: new Date().toISOString(),
    ttlSeconds: 60, // Short TTL for fallback
  };
  
  DNS_CACHE.set(code, fallback);
  return fallback.baseUrl;
}

/**
 * Pre-resolve hosts for known share codes
 */
export function preloadHosts(hosts: Record<string, string>): void {
  for (const [code, baseUrl] of Object.entries(hosts)) {
    DNS_CACHE.set(code, {
      baseUrl,
      resolvedAt: new Date().toISOString(),
      ttlSeconds: DEFAULT_TTL,
    });
  }
}

/**
 * Resolve relay URL for fallback connections
 * Tries well-known relay endpoints
 */
export async function resolveRelayUrl(code: string): Promise<string> {
  const relays = [
    'https://bennett-relay.onrender.com',
    'https://relay.bennett.studio',
    'https://bennett-relay.fly.dev',
  ];

  for (const relay of relays) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);
      const resp = await fetch(`${relay}/api/share/${code}`, { 
        method: 'HEAD',
        signal: controller.signal 
      });
      clearTimeout(timeout);
      if (resp.ok || resp.status === 405) { // 405 = HEAD not allowed but endpoint exists
        return relay;
      }
    } catch {
      // Try next relay
    }
  }

  throw new Error(`No relay available for share ${code}`);
}

/**
 * Clear resolver cache
 */
export function clearResolverCache(): void {
  DNS_CACHE.clear();
  if (typeof localStorage !== 'undefined') {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith('bennett-resolver-')) {
        localStorage.removeItem(key);
      }
    }
  }
}

// Private helpers

async function fetchResolver(code: string): Promise<ResolvedHost | null> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), RESOLVER_TIMEOUT);
  
  try {
    const response = await fetch(`https://resolver.bennett.studio/resolve/${code}`, {
      signal: controller.signal,
      headers: { Accept: 'application/json' },
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) return null;
    
    const data = await response.json();
    if (!data.baseUrl) return null;
    
    return {
      baseUrl: data.baseUrl,
      resolvedAt: new Date().toISOString(),
      ttlSeconds: data.ttlSeconds || DEFAULT_TTL,
    };
  } catch {
    clearTimeout(timeoutId);
    return null;
  }
}

function getPersistentCache(code: string): ResolvedHost | null {
  if (typeof localStorage === 'undefined') return null;
  
  try {
    const raw = localStorage.getItem(`bennett-resolver-${code}`);
    if (!raw) return null;
    return JSON.parse(raw) as ResolvedHost;
  } catch {
    return null;
  }
}

function setPersistentCache(code: string, host: ResolvedHost): void {
  if (typeof localStorage === 'undefined') return;
  
  try {
    localStorage.setItem(`bennett-resolver-${code}`, JSON.stringify(host));
  } catch {
    // Storage full or unavailable
  }
}
