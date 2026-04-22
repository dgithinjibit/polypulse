import React from 'react'

/**
 * BalanceSkeleton - Skeleton loader for balance display
 * Shows animated placeholder while balance is loading
 */
export default function BalanceSkeleton() {
  return (
    <div className="flex items-center gap-2 animate-pulse" role="status" aria-label="Loading balance">
      <div className="h-4 w-16 bg-purple-200 rounded" />
      <div className="h-4 w-8 bg-purple-200 rounded" />
      <span className="sr-only">Loading balance...</span>
    </div>
  )
}
