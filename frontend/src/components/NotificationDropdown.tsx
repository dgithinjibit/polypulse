/**
 * ============================================================
 * FILE: components/NotificationDropdown.tsx
 * PURPOSE: Dropdown panel showing the user's notification list.
 *          Supports marking individual or all notifications as read.
 *          Displays notification icons based on type and relative timestamps.
 *
 * REQUIREMENTS: 13.7, 13.8, 13.9
 * ============================================================
 */

import { useState, useEffect, useRef } from 'react';
import {
  X,
  CheckCheck,
  Clock,
  Users,
  AlertCircle,
  DollarSign,
  Trophy,
  Bell,
} from 'lucide-react';
import rustApiClient from '../config/api';

interface Notification {
  id: number;
  user_id: string;
  actor_id?: string;
  actor_username?: string;
  notification_type: string;
  message: string;
  is_read: boolean;
  created_at: string;
}

interface NotificationListResponse {
  notifications: Notification[];
  total: number;
  limit: number;
  offset: number;
}

interface NotificationDropdownProps {
  onClose: () => void;
  onNotificationRead: () => void;
  onAllRead: () => void;
}

// ============================================================
// FUNCTION: timeAgo
// PURPOSE: Convert an ISO date string to a human-readable relative time.
//          e.g. "2 minutes ago", "3 hours ago", "1 week ago"
// REQUIREMENT: 13.8 - Display timestamps as relative time
// ============================================================
function timeAgo(dateString: string): string {
  const now = new Date();
  const date = new Date(dateString);
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes === 1) return '1 minute ago';
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.floor(minutes / 60);
  if (hours === 1) return '1 hour ago';
  if (hours < 24) return `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return '1 day ago';
  if (days < 7) return `${days} days ago`;
  const weeks = Math.floor(days / 7);
  if (weeks === 1) return '1 week ago';
  if (weeks < 4) return `${weeks} weeks ago`;
  const months = Math.floor(days / 30);
  if (months === 1) return '1 month ago';
  return `${months} months ago`;
}

// ============================================================
// FUNCTION: getNotificationIcon
// PURPOSE: Return the appropriate icon for each notification type.
//          Maps bet event types to visual icons.
// REQUIREMENT: 13.8 - Display notification icons
// ============================================================
function getNotificationIcon(type: string) {
  switch (type) {
    case 'bet_ending_soon':
      return <Clock className="w-5 h-5 text-orange-500" />;
    case 'bet_ended':
      return <Clock className="w-5 h-5 text-red-500" />;
    case 'participant_joined':
    case 'bet_joined':
      return <Users className="w-5 h-5 text-blue-500" />;
    case 'outcome_reported':
      return <AlertCircle className="w-5 h-5 text-yellow-500" />;
    case 'outcome_verified':
      return <CheckCheck className="w-5 h-5 text-green-500" />;
    case 'payout_executed':
      return <DollarSign className="w-5 h-5 text-green-600" />;
    case 'dispute_raised':
    case 'bet_disputed':
      return <AlertCircle className="w-5 h-5 text-red-600" />;
    case 'bet_cancelled':
      return <X className="w-5 h-5 text-gray-500" />;
    default:
      return <Trophy className="w-5 h-5 text-gray-500" />;
  }
}

// ============================================================
// COMPONENT: NotificationDropdown
// PURPOSE: Renders the notification list panel with mark-as-read controls.
// REQUIREMENT: 13.8, 13.9
// ============================================================
export default function NotificationDropdown({
  onClose,
  onNotificationRead,
  onAllRead,
}: NotificationDropdownProps) {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(true);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchNotifications();

    // Close dropdown when clicking outside
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  // Requirement 13.8: Fetch notifications list from backend
  const fetchNotifications = async () => {
    try {
      const token = localStorage.getItem('access_token');
      if (!token) return;

      const response = await rustApiClient.get<NotificationListResponse>(
        '/api/v1/notifications?limit=20'
      );
      setNotifications(response.data.notifications);
    } catch (error) {
      console.error('Failed to fetch notifications:', error);
    } finally {
      setLoading(false);
    }
  };

  // Requirement 13.9: Mark a single notification as read
  const markAsRead = async (notificationId: number) => {
    try {
      await rustApiClient.post(`/api/v1/notifications/${notificationId}/read`);
      setNotifications(prev =>
        prev.map(n => (n.id === notificationId ? { ...n, is_read: true } : n))
      );
      onNotificationRead();
    } catch (error) {
      console.error('Failed to mark notification as read:', error);
    }
  };

  // Requirement 13.9: Mark all notifications as read
  const markAllAsRead = async () => {
    try {
      await rustApiClient.post('/api/v1/notifications/read-all');
      setNotifications(prev => prev.map(n => ({ ...n, is_read: true })));
      onAllRead();
    } catch (error) {
      console.error('Failed to mark all notifications as read:', error);
    }
  };

  const unreadCount = notifications.filter(n => !n.is_read).length;

  return (
    <div
      ref={dropdownRef}
      className="absolute right-0 mt-2 w-96 bg-white rounded-lg shadow-xl border border-gray-200 z-50 max-h-[600px] flex flex-col"
      role="dialog"
      aria-label="Notifications"
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Notifications</h3>
          {unreadCount > 0 && (
            <p className="text-sm text-gray-500">{unreadCount} unread</p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {unreadCount > 0 && (
            <button
              onClick={markAllAsRead}
              className="text-sm text-blue-600 hover:text-blue-700 font-medium flex items-center gap-1"
              title="Mark all as read"
              aria-label="Mark all notifications as read"
            >
              <CheckCheck className="w-5 h-5" />
            </button>
          )}
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close notifications"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Notifications List */}
      <div className="overflow-y-auto flex-1">
        {loading ? (
          <div className="p-8 text-center text-gray-500">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto" role="status" aria-label="Loading notifications"></div>
            <p className="mt-2">Loading notifications...</p>
          </div>
        ) : notifications.length === 0 ? (
          <div className="p-8 text-center text-gray-500">
            <Bell className="w-12 h-12 mx-auto mb-2 text-gray-300" aria-hidden="true" />
            <p>No notifications yet</p>
          </div>
        ) : (
          <ul className="divide-y divide-gray-100" role="list">
            {notifications.map(notification => (
              <li
                key={notification.id}
                className={`p-4 hover:bg-gray-50 transition-colors cursor-pointer ${
                  !notification.is_read ? 'bg-blue-50' : ''
                }`}
                onClick={() => !notification.is_read && markAsRead(notification.id)}
                aria-label={notification.is_read ? notification.message : `Unread: ${notification.message}`}
              >
                <div className="flex items-start gap-3">
                  <div className="flex-shrink-0 mt-1" aria-hidden="true">
                    {getNotificationIcon(notification.notification_type)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-gray-900">{notification.message}</p>
                    {notification.actor_username && (
                      <p className="text-xs text-gray-500 mt-1">
                        by {notification.actor_username}
                      </p>
                    )}
                    {/* Requirement 13.8: Relative timestamp display */}
                    <p className="text-xs text-gray-400 mt-1">
                      {timeAgo(notification.created_at)}
                    </p>
                  </div>
                  {/* Unread indicator dot */}
                  {!notification.is_read && (
                    <div className="flex-shrink-0" aria-hidden="true">
                      <div className="w-2 h-2 bg-blue-600 rounded-full"></div>
                    </div>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
