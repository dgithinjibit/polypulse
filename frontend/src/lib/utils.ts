/**
 * ============================================================
 * FILE: utils.ts
 * PURPOSE: Shared utility functions used across the entire frontend.
 *          Currently contains the `cn` helper for merging Tailwind CSS classes.
 *
 * DEPENDENCIES:
 *   - clsx       : Conditionally joins class names (handles arrays, objects, falsy values)
 *   - tailwind-merge: Merges Tailwind classes intelligently, resolving conflicts.
 *                     e.g., 'p-4 p-2' becomes 'p-2' (last one wins)
 * ============================================================
 */

// clsx: utility for conditionally combining class name strings
// Example: clsx('foo', condition && 'bar') => 'foo bar' or 'foo'
import { type ClassValue, clsx } from "clsx"

// twMerge: resolves Tailwind CSS class conflicts
// Example: twMerge('text-red-500 text-blue-500') => 'text-blue-500'
import { twMerge } from "tailwind-merge"

// ============================================================
// FUNCTION: cn (class names)
// PURPOSE: Combines multiple class name inputs into a single string,
//          handling conditional classes and Tailwind conflicts.
// PARAM ...inputs: Any number of class values (strings, arrays, objects, undefined)
// RETURNS: A single merged class name string
// USAGE: className={cn('base-class', condition && 'conditional-class', props.className)}
// ============================================================
export function cn(...inputs: ClassValue[]) {
  // First clsx joins and filters the inputs, then twMerge resolves Tailwind conflicts
  return twMerge(clsx(inputs))
} // end cn
