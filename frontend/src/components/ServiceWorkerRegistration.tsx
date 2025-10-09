import { useEffect, useState } from "react";
import toast from "react-hot-toast";

export function ServiceWorkerRegistration() {
  const [registration, setRegistration] = useState<ServiceWorkerRegistration | null>(null);

  useEffect(() => {
    // Only register in production
    if (import.meta.env.PROD && "serviceWorker" in navigator) {
      registerServiceWorker();
    }
  }, []);

  const registerServiceWorker = async () => {
    try {
      const reg = await navigator.serviceWorker.register("/sw.js", {
        scope: "/",
      });

      console.log("[SW] Service Worker registered:", reg.scope);
      setRegistration(reg);

      // Check for updates
      reg.addEventListener("updatefound", () => {
        const newWorker = reg.installing;

        if (newWorker) {
          newWorker.addEventListener("statechange", () => {
            if (newWorker.state === "installed" && navigator.serviceWorker.controller) {
              // New service worker installed, update available
              toast.success("New version available! Reload to update.", {
                duration: 10000,
              });
            }
          });
        }
      });

      // Check for updates every hour
      setInterval(
        () => {
          reg.update();
        },
        60 * 60 * 1000
      );
    } catch (error) {
      console.error("[SW] Service Worker registration failed:", error);
    }
  };

  const activateUpdate = () => {
    if (registration?.waiting) {
      // Tell the waiting service worker to skip waiting
      registration.waiting.postMessage({ type: "SKIP_WAITING" });

      // Reload the page when the new service worker activates
      navigator.serviceWorker.addEventListener("controllerchange", () => {
        window.location.reload();
      });
    }
  };

  const clearCache = async () => {
    if (registration) {
      registration.active?.postMessage({ type: "CLEAR_CACHE" });
      toast.success("Cache cleared!");
    }
  };

  // Expose cache clearing for debugging (dev only)
  useEffect(() => {
    if (import.meta.env.DEV) {
      (window as any).clearCache = clearCache;
    }
  }, [registration]);

  return null; // This component doesn't render anything
}
