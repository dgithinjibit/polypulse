/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string
  readonly VITE_RUST_API_URL: string
  readonly VITE_WS_URL: string
  readonly VITE_WEB3AUTH_CLIENT_ID: string
  readonly VITE_WEB3AUTH_GOOGLE_VERIFIER: string
  readonly VITE_WEB3AUTH_APPLE_VERIFIER: string
  readonly VITE_RPC_URL: string
  readonly VITE_SECRET_NETWORK_LCD: string
  readonly VITE_SECRET_NETWORK_CHAIN_ID: string
  readonly VITE_WAGER_CONTRACT_ADDRESS: string
  readonly VITE_TELEGRAM_BOT_USERNAME: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
