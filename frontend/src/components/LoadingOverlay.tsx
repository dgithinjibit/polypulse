import React from 'react'

interface LoadingOverlayProps {
  message?: string
  isVisible: boolean
}

/**
 * LoadingOverlay - Full-screen overlay with spinner for blocking operations
 * Used during authentication and other critical async operations
 */
export default function LoadingOverlay({ message = 'Loading...', isVisible }: LoadingOverlayProps) {
  if (!isVisible) return null

  return (
    <div
      className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="loading-message"
    >
      <div className="bg-white rounded-2xl shadow-2xl p-8 flex flex-col items-center gap-4 max-w-sm mx-4">
        <div
          className="animate-spin rounded-full h-12 w-12 border-4 border-indigo-200 border-t-indigo-600"
          role="status"
          aria-label="Loading"
        />
        <p id="loading-message" className="text-gray-700 font-medium text-center">
          {message}
        </p>
      </div>
    </div>
  )
}
