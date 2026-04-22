/**
 * Error Handler - User-friendly error messages with retry and help links
 */

import { toast } from '../hooks/use-toast'
import { ToastAction } from '../components/ui/toast'
import {
  WalletNotInstalledError,
  WalletConnectionError,
  WalletRejectedError,
  NetworkError,
  SignatureError,
} from './stellar-helper'

export interface ErrorHandlerOptions {
  /** Optional retry callback function */
  onRetry?: () => void | Promise<void>
  /** Optional help link URL */
  helpLink?: string
  /** Custom error title */
  title?: string
  /** Show toast notification (default: true) */
  showToast?: boolean
}

/**
 * Error message templates with user-friendly descriptions and help links
 */
const ERROR_MESSAGES = {
  // Wallet errors
  WALLET_NOT_INSTALLED: {
    title: 'Wallet Not Installed',
    description: 'Please install Freighter wallet to connect.',
    helpLink: 'https://www.freighter.app/',
    helpText: 'Install Freighter',
  },
  WALLET_CONNECTION_FAILED: {
    title: 'Connection Failed',
    description: 'Unable to connect to your wallet. Please try again.',
    helpLink: 'https://docs.freighter.app/docs/guide/gettingStarted',
    helpText: 'View Guide',
  },
  WALLET_REJECTED: {
    title: 'Connection Cancelled',
    description: 'You cancelled the wallet connection. Click retry to try again.',
    helpLink: null,
    helpText: null,
  },
  
  // Authentication errors
  AUTH_FAILED: {
    title: 'Authentication Failed',
    description: 'Unable to authenticate with the server. Please try again.',
    helpLink: '/help/authentication',
    helpText: 'Get Help',
  },
  AUTH_SIGNATURE_REJECTED: {
    title: 'Signature Cancelled',
    description: 'You cancelled the signature request. Authentication requires your signature.',
    helpLink: '/help/authentication',
    helpText: 'Why do I need to sign?',
  },
  AUTH_SIGNATURE_FAILED: {
    title: 'Signature Failed',
    description: 'Unable to sign the authentication message. Please try again.',
    helpLink: '/help/authentication',
    helpText: 'Get Help',
  },
  
  // Network errors
  NETWORK_ERROR: {
    title: 'Network Error',
    description: 'Unable to connect to the network. Please check your internet connection.',
    helpLink: '/help/network-issues',
    helpText: 'Troubleshoot',
  },
  HORIZON_ERROR: {
    title: 'Blockchain Error',
    description: 'Unable to connect to the Stellar network. Please try again later.',
    helpLink: 'https://status.stellar.org/',
    helpText: 'Check Status',
  },
  
  // Transaction errors
  TRANSACTION_FAILED: {
    title: 'Transaction Failed',
    description: 'Your transaction could not be completed. Please try again.',
    helpLink: '/help/transactions',
    helpText: 'Get Help',
  },
  INSUFFICIENT_BALANCE: {
    title: 'Insufficient Balance',
    description: 'You do not have enough XLM to complete this transaction.',
    helpLink: '/help/funding',
    helpText: 'How to Fund',
  },
  
  // Generic errors
  UNKNOWN_ERROR: {
    title: 'Something Went Wrong',
    description: 'An unexpected error occurred. Please try again.',
    helpLink: '/help',
    helpText: 'Get Help',
  },
}

/**
 * Determines the appropriate error message template based on the error type
 */
function getErrorTemplate(error: unknown): typeof ERROR_MESSAGES[keyof typeof ERROR_MESSAGES] {
  if (error instanceof WalletNotInstalledError) {
    return ERROR_MESSAGES.WALLET_NOT_INSTALLED
  }
  
  if (error instanceof WalletRejectedError) {
    const message = error.message.toLowerCase()
    if (message.includes('authentication') || message.includes('signing')) {
      return ERROR_MESSAGES.AUTH_SIGNATURE_REJECTED
    }
    return ERROR_MESSAGES.WALLET_REJECTED
  }
  
  if (error instanceof WalletConnectionError) {
    return ERROR_MESSAGES.WALLET_CONNECTION_FAILED
  }
  
  if (error instanceof SignatureError) {
    return ERROR_MESSAGES.AUTH_SIGNATURE_FAILED
  }
  
  if (error instanceof NetworkError) {
    const message = error.message.toLowerCase()
    if (message.includes('horizon') || message.includes('stellar')) {
      return ERROR_MESSAGES.HORIZON_ERROR
    }
    return ERROR_MESSAGES.NETWORK_ERROR
  }
  
  // Check for axios/API errors
  if (typeof error === 'object' && error !== null) {
    const axiosError = error as any
    
    // Authentication errors
    if (axiosError.response?.status === 401 || axiosError.response?.status === 403) {
      return ERROR_MESSAGES.AUTH_FAILED
    }
    
    // Network errors
    if (axiosError.code === 'ECONNREFUSED' || axiosError.code === 'ERR_NETWORK') {
      return ERROR_MESSAGES.NETWORK_ERROR
    }
    
    // Insufficient balance
    if (axiosError.response?.data?.error?.includes('insufficient')) {
      return ERROR_MESSAGES.INSUFFICIENT_BALANCE
    }
  }
  
  return ERROR_MESSAGES.UNKNOWN_ERROR
}

/**
 * Main error handler function - displays user-friendly error messages with retry and help options
 */
export function handleError(error: unknown, options: ErrorHandlerOptions = {}): void {
  const {
    onRetry,
    helpLink: customHelpLink,
    title: customTitle,
    showToast = true,
  } = options
  
  // Get error template
  const template = getErrorTemplate(error)
  
  // Use custom values or template defaults
  const title = customTitle || template.title
  const description = template.description
  const helpLink = customHelpLink || template.helpLink
  const helpText = template.helpText
  
  // Log error for debugging
  console.error(`[ErrorHandler] ${title}:`, error)
  
  // Show toast notification
  if (showToast) {
    const toastOptions: any = {
      variant: 'destructive',
      title,
      description,
    }
    
    // Add action button (retry or help link)
    if (onRetry) {
      toastOptions.action = (
        <ToastAction
          altText="Retry"
          onClick={async () => {
            try {
              await onRetry()
            } catch (retryError) {
              console.error('[ErrorHandler] Retry failed:', retryError)
            }
          }}
        >
          Retry
        </ToastAction>
      )
    } else if (helpLink) {
      toastOptions.action = (
        <ToastAction
          altText={helpText || 'Get Help'}
          onClick={() => {
            if (helpLink.startsWith('http')) {
              window.open(helpLink, '_blank', 'noopener,noreferrer')
            } else {
              window.location.href = helpLink
            }
          }}
        >
          {helpText || 'Get Help'}
        </ToastAction>
      )
    }
    
    toast(toastOptions)
  }
}

/**
 * Success message handler
 */
export function handleSuccess(message: string, description?: string): void {
  toast({
    variant: 'success',
    title: message,
    description,
  })
}

/**
 * Warning message handler
 */
export function handleWarning(message: string, description?: string): void {
  toast({
    variant: 'warning',
    title: message,
    description,
  })
}

/**
 * Info message handler
 */
export function handleInfo(message: string, description?: string): void {
  toast({
    variant: 'default',
    title: message,
    description,
  })
}

/**
 * Wallet-specific error handlers
 */
export const walletErrors = {
  notInstalled: (onRetry?: () => void) => {
    handleError(new WalletNotInstalledError('Freighter', 'https://www.freighter.app/'), {
      onRetry,
    })
  },
  
  connectionFailed: (onRetry?: () => void) => {
    handleError(new WalletConnectionError('Connection failed'), {
      onRetry,
    })
  },
  
  userRejected: (action: string = 'connection') => {
    handleError(new WalletRejectedError(action))
  },
  
  authenticationFailed: (onRetry?: () => void) => {
    handleError(new Error('Authentication failed'), {
      title: 'Authentication Failed',
      onRetry,
    })
  },
}

/**
 * Transaction-specific error handlers
 */
export const transactionErrors = {
  failed: (onRetry?: () => void) => {
    toast({
      variant: 'destructive',
      title: 'Transaction Failed',
      description: 'Your transaction could not be completed. Please try again.',
      action: onRetry ? (
        <ToastAction altText="Retry" onClick={onRetry}>
          Retry
        </ToastAction>
      ) : undefined,
    })
  },
  
  insufficientBalance: () => {
    toast({
      variant: 'destructive',
      title: 'Insufficient Balance',
      description: 'You do not have enough XLM to complete this transaction.',
      action: (
        <ToastAction
          altText="Fund Wallet"
          onClick={() => window.location.href = '/wallet'}
        >
          Fund Wallet
        </ToastAction>
      ),
    })
  },
  
  userRejected: () => {
    toast({
      variant: 'warning',
      title: 'Transaction Cancelled',
      description: 'You cancelled the transaction.',
    })
  },
}

/**
 * Network-specific error handlers
 */
export const networkErrors = {
  offline: () => {
    toast({
      variant: 'destructive',
      title: 'No Internet Connection',
      description: 'Please check your internet connection and try again.',
    })
  },
  
  horizonDown: () => {
    toast({
      variant: 'destructive',
      title: 'Network Unavailable',
      description: 'The Stellar network is currently unavailable. Please try again later.',
      action: (
        <ToastAction
          altText="Check Status"
          onClick={() => window.open('https://status.stellar.org/', '_blank')}
        >
          Check Status
        </ToastAction>
      ),
    })
  },
  
  timeout: (onRetry?: () => void) => {
    toast({
      variant: 'destructive',
      title: 'Request Timeout',
      description: 'The request took too long. Please try again.',
      action: onRetry ? (
        <ToastAction altText="Retry" onClick={onRetry}>
          Retry
        </ToastAction>
      ) : undefined,
    })
  },
}
