/**
 * PWA Service - Handles service worker registration and push notifications
 */

export class PWAService {
  private static registration: ServiceWorkerRegistration | null = null;

  /**
   * Register service worker
   */
  static async registerServiceWorker(): Promise<void> {
    if (!('serviceWorker' in navigator)) {
      console.warn('Service workers not supported');
      return;
    }

    try {
      const registration = await navigator.serviceWorker.register('/sw.js', {
        scope: '/',
      });

      this.registration = registration;
      console.log('Service worker registered:', registration.scope);

      // Check for updates
      registration.addEventListener('updatefound', () => {
        const newWorker = registration.installing;
        if (newWorker) {
          newWorker.addEventListener('statechange', () => {
            if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
              // New service worker available
              console.log('New service worker available');
              this.showUpdateNotification();
            }
          });
        }
      });
    } catch (error) {
      console.error('Service worker registration failed:', error);
    }
  }

  /**
   * Request push notification permission
   */
  static async requestNotificationPermission(): Promise<boolean> {
    if (!('Notification' in window)) {
      console.warn('Notifications not supported');
      return false;
    }

    if (Notification.permission === 'granted') {
      return true;
    }

    if (Notification.permission === 'denied') {
      return false;
    }

    const permission = await Notification.requestPermission();
    return permission === 'granted';
  }

  /**
   * Subscribe to push notifications
   */
  static async subscribeToPush(userId: number): Promise<boolean> {
    if (!this.registration) {
      console.error('Service worker not registered');
      return false;
    }

    try {
      // Request permission
      const hasPermission = await this.requestNotificationPermission();
      if (!hasPermission) {
        return false;
      }

      // Get push subscription
      const subscription = await this.registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: this.urlBase64ToUint8Array(
          import.meta.env.VITE_VAPID_PUBLIC_KEY || ''
        ) as Uint8Array,
      });

      // Send subscription to server
      await fetch('/api/v1/push/subscribe', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          userId,
          subscription,
        }),
      });

      console.log('Push subscription successful');
      return true;
    } catch (error) {
      console.error('Push subscription failed:', error);
      return false;
    }
  }

  /**
   * Unsubscribe from push notifications
   */
  static async unsubscribeFromPush(userId: number): Promise<void> {
    if (!this.registration) {
      return;
    }

    try {
      const subscription = await this.registration.pushManager.getSubscription();
      if (subscription) {
        await subscription.unsubscribe();

        // Notify server
        await fetch('/api/v1/push/unsubscribe', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ userId }),
        });

        console.log('Push unsubscription successful');
      }
    } catch (error) {
      console.error('Push unsubscription failed:', error);
    }
  }

  /**
   * Check if app is installed (running as PWA)
   */
  static isInstalled(): boolean {
    return window.matchMedia('(display-mode: standalone)').matches ||
           (window.navigator as any).standalone === true;
  }

  /**
   * Show install prompt
   */
  static showInstallPrompt(): void {
    // This is handled by the browser's beforeinstallprompt event
    // We can store the event and trigger it when needed
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      // Store event for later use
      (window as any).deferredPrompt = e;
      
      // Show custom install button
      const installButton = document.getElementById('install-button');
      if (installButton) {
        installButton.style.display = 'block';
      }
    });
  }

  /**
   * Trigger install prompt
   */
  static async triggerInstall(): Promise<boolean> {
    const deferredPrompt = (window as any).deferredPrompt;
    if (!deferredPrompt) {
      return false;
    }

    deferredPrompt.prompt();
    const { outcome } = await deferredPrompt.userChoice;
    
    if (outcome === 'accepted') {
      console.log('PWA installed');
      (window as any).deferredPrompt = null;
      return true;
    }

    return false;
  }

  /**
   * Show update notification
   */
  private static showUpdateNotification(): void {
    // Show toast or modal to user
    const updateBanner = document.createElement('div');
    updateBanner.className = 'fixed bottom-4 right-4 bg-blue-600 text-white px-6 py-3 rounded-lg shadow-lg z-50';
    updateBanner.innerHTML = `
      <p class="font-medium">New version available!</p>
      <button onclick="window.location.reload()" class="mt-2 bg-white text-blue-600 px-4 py-1 rounded text-sm font-medium">
        Update Now
      </button>
    `;
    document.body.appendChild(updateBanner);
  }

  /**
   * Convert VAPID key to Uint8Array
   */
  private static urlBase64ToUint8Array(base64String: string): Uint8Array {
    const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
    const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
    const rawData = window.atob(base64);
    const outputArray = new Uint8Array(rawData.length);
    for (let i = 0; i < rawData.length; ++i) {
      outputArray[i] = rawData.charCodeAt(i);
    }
    return outputArray;
  }

  /**
   * Initialize PWA features
   */
  static async initialize(userId?: number): Promise<void> {
    // Register service worker
    await this.registerServiceWorker();

    // Show install prompt handler
    this.showInstallPrompt();

    // Request notification permission if user is logged in
    if (userId && !this.isInstalled()) {
      // Wait a bit before asking for permissions
      setTimeout(() => {
        this.requestNotificationPermission();
      }, 5000);
    }
  }
}
