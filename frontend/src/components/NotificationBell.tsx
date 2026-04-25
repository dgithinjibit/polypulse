/**
 * ============================================================
 * FILE: components/NotificationBell.tsx
 * PURPOSE: Notification bell icon with unread count badge.
 *          Displays in the app header/navbar.
 *          Integrates with WebSocket for real-time notification delivery.
 *
 * REQUIREMENTS: 13.7, 13.8, 13.9
 * ============================================================
 */

import { useState, useEffect } from 'react';
import { Bell } from 'lucide-react';
import NotificationDropdown from './NotificationDropdown';
import { websocketService } from '../services/websocket';
import rustApiClient from '../config/api';

interface NotificationBellProps {
  className?: string;
}

export default function NotificationBell({ className = '' }: NotificationBellProps) {
  const [unreadCount, setUnreadCount] = useState(0);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    fetchUnreadCount();

    // Poll for unread count every 30 seconds as a fallback
    const interval = setInterval(fetchUnreadCount, 30000);

    // Subscribe to real-time notifications via WebSocket
    // Requirement 13.7: Real-time notification delivery
    const unsubscribe = websocketService.subscribeToNotifications((notification) => {
      // Increment unread count when a new notification arrives
      setUnreadCount(prev => prev + 1);

      // Show browser notification if permission granted
      if (Notification.permission === 'granted') {
        new Notification('PolyPulse', {
          body: notification.message,
          icon: '/favicon.ico',
        });
      }
    });

    return () => {
      clearInterval(interval);
      unsubscribe();
    };
  }, []);

  // Requirement 13.7: Fetch unread notification count from backend
  const fetchUnreadCount = async () => {
    try {
      const token = localStorage.getItem('access_token');
      if (!token) return;

      const response = await rustApiClient.get<{ count: number }>(
        '/api/v1/notifications/unread-count'
      );
      setUnreadCount(response.data.count);
    } catch (error) {
      // Silently fail - unread count is non-critical
      console.error('Failed to fetch unread count:', error);
    }
  };

  // Called by NotificationDropdown when a single notification is marked as read
  const handleNotificationRead = () => {
    setUnreadCount(prev => Math.max(0, prev - 1));
  };

  // Called by NotificationDropdown when all notifications are marked as read
  const handleAllRead = () => {
    setUnreadCount(0);
  };

  return (
    <div className={`relative ${className}`}>
      {/* Requirement 13.7: Bell icon with unread count badge */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="relative p-2 text-purple-100 hover:text-white focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-700 rounded-lg transition-colors"
        aria-label={`Notifications${unreadCount > 0 ? `, ${unreadCount} unread` : ''}`}
        aria-haspopup="true"
        aria-expanded={isOpen}
      >
        <Bell className="w-6 h-6" />
        {unreadCount > 0 && (
          <span
            className="absolute top-0 right-0 inline-flex items-center justify-center px-2 py-1 text-xs font-bold leading-none text-white transform translate-x-1/2 -translate-y-1/2 bg-red-500 rounded-full"
            aria-hidden="true"
          >
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {/* Requirement 13.8: Notification dropdown on bell click */}
      {isOpen && (
        <NotificationDropdown
          onClose={() => setIsOpen(false)}
          onNotificationRead={handleNotificationRead}
          onAllRead={handleAllRead}
        />
      )}
    </div>
  );
}
