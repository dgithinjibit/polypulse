import { useState, useCallback } from 'react'
import rustApiClient, { WAGER_ENDPOINTS } from '../config/api'
import type { Wager, WagerListItem, CreateWagerPayload, WagerPortfolio } from '../types/wager'

interface UseWagersReturn {
  wagers: WagerListItem[]
  currentWager: Wager | null
  portfolio: WagerPortfolio | null
  isLoading: boolean
  error: string | null
  fetchWagers: () => Promise<void>
  fetchWager: (id: string) => Promise<void>
  createWager: (payload: CreateWagerPayload) => Promise<Wager | null>
  acceptWager: (id: string) => Promise<boolean>
  cancelWager: (id: string) => Promise<boolean>
  fetchPortfolio: () => Promise<void>
  discoverWagers: (filters?: DiscoverFilters) => Promise<WagerListItem[]>
}

export interface DiscoverFilters {
  category?: string
  minAmount?: number
  maxAmount?: number
  expiresAfter?: string
  page?: number
  limit?: number
}

export function useWagers(): UseWagersReturn {
  const [wagers, setWagers] = useState<WagerListItem[]>([])
  const [currentWager, setCurrentWager] = useState<Wager | null>(null)
  const [portfolio, setPortfolio] = useState<WagerPortfolio | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchWagers = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.get<WagerListItem[]>(WAGER_ENDPOINTS.LIST)
      setWagers(res.data)
    } catch (err: unknown) {
      setError(getErrorMessage(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const fetchWager = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.get<Wager>(WAGER_ENDPOINTS.DETAIL(id))
      setCurrentWager(res.data)
    } catch (err: unknown) {
      setError(getErrorMessage(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const createWager = useCallback(async (payload: CreateWagerPayload): Promise<Wager | null> => {
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.post<Wager>(WAGER_ENDPOINTS.CREATE, payload)
      return res.data
    } catch (err: unknown) {
      setError(getErrorMessage(err))
      return null
    } finally {
      setIsLoading(false)
    }
  }, [])

  const acceptWager = useCallback(async (id: string): Promise<boolean> => {
    setIsLoading(true)
    setError(null)
    try {
      await rustApiClient.post(WAGER_ENDPOINTS.ACCEPT(id))
      return true
    } catch (err: unknown) {
      setError(getErrorMessage(err))
      return false
    } finally {
      setIsLoading(false)
    }
  }, [])

  const cancelWager = useCallback(async (id: string): Promise<boolean> => {
    setIsLoading(true)
    setError(null)
    try {
      await rustApiClient.post(WAGER_ENDPOINTS.CANCEL(id))
      return true
    } catch (err: unknown) {
      setError(getErrorMessage(err))
      return false
    } finally {
      setIsLoading(false)
    }
  }, [])

  const fetchPortfolio = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.get<WagerPortfolio>(WAGER_ENDPOINTS.PORTFOLIO)
      setPortfolio(res.data)
    } catch (err: unknown) {
      setError(getErrorMessage(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const discoverWagers = useCallback(async (filters?: DiscoverFilters): Promise<WagerListItem[]> => {
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.get<WagerListItem[]>(WAGER_ENDPOINTS.DISCOVER, { params: filters })
      return res.data
    } catch (err: unknown) {
      setError(getErrorMessage(err))
      return []
    } finally {
      setIsLoading(false)
    }
  }, [])

  return {
    wagers,
    currentWager,
    portfolio,
    isLoading,
    error,
    fetchWagers,
    fetchWager,
    createWager,
    acceptWager,
    cancelWager,
    fetchPortfolio,
    discoverWagers,
  }
}

function getErrorMessage(err: unknown): string {
  if (err && typeof err === 'object' && 'response' in err) {
    const axiosErr = err as { response?: { data?: { message?: string } } }
    return axiosErr.response?.data?.message || 'An error occurred'
  }
  if (err instanceof Error) return err.message
  return 'An error occurred'
}
