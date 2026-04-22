/**
 * ============================================================
 * FILE: main.tsx
 * PURPOSE: The entry point of the React application.
 *          This is the first file that runs when the browser loads the app.
 *          It:
 *            1. Sets up global error handlers to catch crashes and show them on screen
 *            2. Wraps the app in an ErrorBoundary to catch React render errors
 *            3. Mounts the React app into the #root div in index.html
 *
 * JUNIOR DEV NOTE:
 *   The error handlers here are for DEVELOPMENT debugging.
 *   In production, you'd want to send errors to a logging service (e.g., Sentry).
 * ============================================================
 */

// React is required for JSX syntax (even if not explicitly used in newer React versions)
import React from 'react'

// ReactDOM renders the React component tree into the actual browser DOM
import ReactDOM from 'react-dom/client'

// The root App component that contains all routes and providers
import App from './App'

// Global CSS styles applied to the entire app
import './main.css'

// ============================================================
// GLOBAL ERROR HANDLER: window.addEventListener('error')
// PURPOSE: Catches synchronous JavaScript errors that happen outside React's lifecycle.
//          e.g., import errors, syntax errors in modules, errors in event handlers.
//          When caught, replaces the entire page with a readable error message.
//          This is especially useful for catching the "white screen of death" during dev.
// ============================================================
window.addEventListener('error', (e) => {
  // Replace the entire page body with a dark-themed error display
  // Shows the error message and the file/line where it occurred
  document.body.innerHTML = `<div style="padding:32px;font-family:monospace;background:#1a1a1a;color:#ff4444;min-height:100vh">
    <h2>JS Error</h2>
    <pre style="color:#ff8888">${e.message}</pre>
    <pre style="color:#888;font-size:11px">${e.filename}:${e.lineno}</pre>
  </div>`
}) // end window error handler

// ============================================================
// GLOBAL PROMISE REJECTION HANDLER: window.addEventListener('unhandledrejection')
// PURPOSE: Catches Promise rejections that were never caught with .catch() or try/catch.
//          e.g., async functions that throw without being awaited in a try/catch.
//          Shows the rejection reason and stack trace on screen.
// ============================================================
window.addEventListener('unhandledrejection', (e) => {
  // Replace the page body with the promise rejection details
  document.body.innerHTML = `<div style="padding:32px;font-family:monospace;background:#1a1a1a;color:#ff4444;min-height:100vh">
    <h2>Unhandled Promise Rejection</h2>
    <pre style="color:#ff8888">${String(e.reason)}</pre>
    <pre style="color:#888;font-size:11px">${e.reason?.stack || ''}</pre>
  </div>`
}) // end unhandledrejection handler

// ============================================================
// CLASS: ErrorBoundary
// PURPOSE: A React class component that catches errors thrown during rendering.
//          React's error boundary mechanism only works with class components.
//          If any child component throws during render, this catches it and
//          shows a fallback UI instead of crashing the whole app silently.
//
// JUNIOR DEV NOTE:
//   getDerivedStateFromError is a static lifecycle method called when a child throws.
//   It updates state to trigger the fallback render.
// ============================================================
class ErrorBoundary extends React.Component<
  { children: React.ReactNode },  // Props: just the child components to wrap
  { error: Error | null }         // State: the caught error, or null if no error
> {
  // Initial state: no error
  state = { error: null }

  // Called when a child component throws during rendering.
  // Returns the new state to set (triggers re-render with error UI).
  static getDerivedStateFromError(error: Error) { return { error } }

  // Render method: shows error UI if there's an error, otherwise renders children normally
  render() {
    // If we have a caught error, show the error details instead of the app
    if (this.state.error) {
      return (
        <div style={{ padding: 32, fontFamily: 'monospace', background: '#1a1a1a', minHeight: '100vh' }}>
          <h2 style={{ color: '#ff4444' }}>App crashed -- check console</h2>
          {/* Show the error message in red */}
          <pre style={{ whiteSpace: 'pre-wrap', color: '#ff8888', fontSize: 13 }}>
            {(this.state.error as Error).message}
          </pre>
          {/* Show the stack trace in grey for debugging */}
          <pre style={{ whiteSpace: 'pre-wrap', color: '#888', fontSize: 11 }}>
            {(this.state.error as Error).stack}
          </pre>
        </div>
      )
    } // end error state check

    // No error - render children normally
    return this.props.children
  } // end render
} // end class ErrorBoundary

// ============================================================
// APP MOUNT
// PURPOSE: Creates the React root and renders the app into the DOM.
//          document.getElementById('root') finds the <div id="root"> in index.html.
//          React.StrictMode enables extra development warnings (double-renders components
//          to detect side effects - this is normal in development, not a bug).
//          ErrorBoundary wraps App to catch any render-time crashes.
// ============================================================
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    {/* ErrorBoundary catches any render errors from App or its children */}
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
)

// Remove loading screen once React has mounted
document.body.classList.add('loaded')
