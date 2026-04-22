import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'

/**
 * No Web2 registration on a dApp.
 * Just redirect to login (wallet connect).
 */
export default function Register() {
  const navigate = useNavigate()
  useEffect(() => { navigate('/login', { replace: true }) }, [navigate])
  return null
}
