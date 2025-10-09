// Service Worker for Bridge Router
// Version: 1.0.0

const CACHE_VERSION = "bridge-router-v1";
const STATIC_CACHE = `${CACHE_VERSION}-static`;
const IMAGE_CACHE = `${CACHE_VERSION}-images`;
const API_CACHE = `${CACHE_VERSION}-api`;
const FONT_CACHE = `${CACHE_VERSION}-fonts`;

// Cache duration in seconds
const CACHE_DURATION = {
  STATIC: 7 * 24 * 60 * 60, // 7 days
  IMAGES: 30 * 24 * 60 * 60, // 30 days
  API: 5 * 60, // 5 minutes
  FONTS: 365 * 24 * 60 * 60, // 1 year
};

// Static assets to cache on install
const STATIC_ASSETS = ["/", "/routes", "/support", "/_astro/client.js", "/manifest.json"];

// Install event - cache static assets
self.addEventListener("install", (event) => {
  console.log("[SW] Installing service worker...");

  event.waitUntil(
    (async () => {
      const cache = await caches.open(STATIC_CACHE);
      console.log("[SW] Caching static assets");
      try {
        await cache.addAll(STATIC_ASSETS);
      } catch (err) {
        console.warn("[SW] Failed to cache some static assets:", err);
      }
    })()
  );

  // Activate immediately
  self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener("activate", (event) => {
  console.log("[SW] Activating service worker...");

  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          // Delete old versions of caches
          if (
            cacheName.startsWith("bridge-router-") &&
            cacheName !== STATIC_CACHE &&
            cacheName !== IMAGE_CACHE &&
            cacheName !== API_CACHE &&
            cacheName !== FONT_CACHE
          ) {
            console.log("[SW] Deleting old cache:", cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    })
  );

  // Take control immediately
  return self.clients.claim();
});

// Fetch event - handle requests with caching strategies
self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests
  if (request.method !== "GET") {
    return;
  }

  // Skip chrome extensions and dev server
  if (url.protocol === "chrome-extension:" || url.hostname === "localhost") {
    return;
  }

  // Handle different types of requests
  if (request.destination === "image") {
    event.respondWith(handleImageRequest(request));
  } else if (url.pathname.startsWith("/api/")) {
    event.respondWith(handleApiRequest(request));
  } else if (request.destination === "font") {
    event.respondWith(handleFontRequest(request));
  } else if (request.destination === "script" || request.destination === "style") {
    event.respondWith(handleStaticAsset(request));
  } else {
    event.respondWith(handlePageRequest(request));
  }
});

/**
 * Cache-first strategy for images
 * Try cache first, fall back to network, cache the result
 */
async function handleImageRequest(request) {
  const cache = await caches.open(IMAGE_CACHE);
  const cached = await cache.match(request);

  if (cached) {
    console.log("[SW] Image from cache:", request.url);
    return cached;
  }

  try {
    const response = await fetch(request);

    if (response.ok) {
      // Clone response before caching
      cache.put(request, response.clone());
      console.log("[SW] Image cached:", request.url);
    }

    return response;
  } catch (error) {
    console.error("[SW] Image fetch failed:", error);
    // Return a placeholder or cached version if available
    return new Response("", { status: 404, statusText: "Image not found" });
  }
}

/**
 * Network-first strategy with short cache for API
 * Try network first, fall back to cache
 */
async function handleApiRequest(request) {
  const cache = await caches.open(API_CACHE);

  try {
    const response = await fetch(request);

    if (response.ok) {
      // Cache successful API responses for 5 minutes
      const clonedResponse = response.clone();
      cache.put(request, clonedResponse);
      console.log("[SW] API response cached:", request.url);
    }

    return response;
  } catch (error) {
    console.log("[SW] Network failed, trying cache for:", request.url);
    const cached = await cache.match(request);

    if (cached) {
      console.log("[SW] Returning cached API response");
      return cached;
    }

    return new Response(JSON.stringify({ error: "Network error" }), {
      status: 503,
      headers: { "Content-Type": "application/json" },
    });
  }
}

/**
 * Cache-first strategy for fonts (long-term caching)
 */
async function handleFontRequest(request) {
  const cache = await caches.open(FONT_CACHE);
  const cached = await cache.match(request);

  if (cached) {
    return cached;
  }

  try {
    const response = await fetch(request);

    if (response.ok) {
      cache.put(request, response.clone());
    }

    return response;
  } catch (error) {
    console.error("[SW] Font fetch failed:", error);
    return new Response("", { status: 404 });
  }
}

/**
 * Stale-while-revalidate for static assets
 * Return cached version immediately, update in background
 */
async function handleStaticAsset(request) {
  const cache = await caches.open(STATIC_CACHE);
  const cached = await cache.match(request);

  // Return cached immediately if available
  const fetchPromise = fetch(request)
    .then((response) => {
      if (response.ok) {
        cache.put(request, response.clone());
      }
      return response;
    })
    .catch(() => cached);

  return cached || fetchPromise;
}

/**
 * Network-first for pages, fall back to cache
 */
async function handlePageRequest(request) {
  const cache = await caches.open(STATIC_CACHE);

  try {
    const response = await fetch(request);

    if (response.ok) {
      cache.put(request, response.clone());
    }

    return response;
  } catch (error) {
    const cached = await cache.match(request);

    if (cached) {
      console.log("[SW] Serving cached page:", request.url);
      return cached;
    }

    // Return offline page if available
    const offlinePage = await cache.match("/offline.html");
    return offlinePage || new Response("Offline", { status: 503 });
  }
}

// Handle messages from clients
self.addEventListener("message", (event) => {
  if (event.data && event.data.type === "SKIP_WAITING") {
    self.skipWaiting();
  }

  if (event.data && event.data.type === "CLEAR_CACHE") {
    event.waitUntil(
      caches.keys().then((cacheNames) => {
        return Promise.all(cacheNames.map((name) => caches.delete(name)));
      })
    );
  }
});

console.log("[SW] Service Worker loaded");
