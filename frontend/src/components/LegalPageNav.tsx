import { useLocation, Link, useNavigate } from 'react-router-dom'
import { useState } from 'react'

interface LegalPageNavProps {
  currentPage: 'terms' | 'privacy' | 'faq' | 'leaderboard'
}

export default function LegalPageNav({ currentPage }: LegalPageNavProps) {
  const navigate = useNavigate()
  const [isZooming, setIsZooming] = useState(false)
  const [targetPage, setTargetPage] = useState<string | null>(null)

  const pages = [
    { id: 'faq', label: 'FAQ', path: '/faq' },
    { id: 'terms', label: 'Terms', path: '/terms' },
    { id: 'privacy', label: 'Privacy', path: '/privacy' },
    { id: 'leaderboard', label: 'Leaderboard', path: '/leaderboard' },
  ]

  const handleNavClick = (e: React.MouseEvent, page: typeof pages[0]) => {
    if (page.id === currentPage) return // Already on this page
    
    e.preventDefault()
    setTargetPage(page.id)
    setIsZooming(true)
    
    // Navigate after zoom animation
    setTimeout(() => {
      navigate(page.path)
      setIsZooming(false)
      setTargetPage(null)
    }, 700)
  }

  const activePage = pages.find(p => p.id === (targetPage || currentPage))

  return (
    <>
      {/* Navigation Circles */}
      <div className={`mb-8 flex items-center justify-center gap-4 transition-all duration-500 ${
        isZooming ? 'scale-50 opacity-0' : 'scale-100 opacity-100'
      }`}>
        {pages.map((page, idx) => (
          <div key={page.id} className="flex items-center gap-4">
            {/* Clickable nav item with circle for active state */}
            <Link
              to={page.path}
              onClick={(e) => handleNavClick(e, page)}
              className={`flex items-center justify-center w-12 h-12 rounded-full transition-all duration-300 ${
                currentPage === page.id
                  ? 'bg-gradient-to-r from-cyan-400 to-blue-500 text-gray-950 font-bold shadow-lg shadow-cyan-500/50'
                  : 'border-2 border-slate-600 text-slate-300 hover:border-cyan-400 hover:text-cyan-400'
              }`}
              title={page.label}
            >
              <span className="text-sm font-semibold">{page.label.charAt(0)}</span>
            </Link>

            {/* Hollow rectangle connector - only between items */}
            {idx < pages.length - 1 && (
              <div
                className={`w-12 h-1 transition-all duration-300 ${
                  currentPage === page.id || currentPage === pages[idx + 1].id
                    ? 'bg-gradient-to-r from-cyan-400 to-blue-500'
                    : 'border-t-2 border-slate-600'
                }`}
              />
            )}
          </div>
        ))}

        {/* Labels below circles */}
        <div className="absolute mt-20 flex items-center gap-16 pointer-events-none">
          {pages.map((page) => (
            <div key={`label-${page.id}`} className="text-center w-12">
              <p className="text-xs font-medium text-slate-400 mt-2 pointer-events-auto">
                {page.label}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Zoom Reveal Component */}
      {isZooming && activePage && (
        <div className={`fixed inset-0 z-50 flex items-center justify-center bg-gray-950/95 transition-all duration-700 ${
          isZooming ? 'scale-100 opacity-100' : 'scale-0 opacity-0'
        }`}>
          <div className="flex items-center gap-0">
            {/* Button Circle */}
            <div className="w-16 h-16 rounded-full bg-gradient-to-r from-cyan-400 to-blue-500 text-gray-950 font-bold text-xl flex items-center justify-center shadow-lg shadow-cyan-500/50 z-10">
              {activePage.label.charAt(0)}
            </div>
            
            {/* Connector Rectangle */}
            <div className="w-16 h-16 bg-gradient-to-r from-cyan-400 to-blue-500"></div>
            
            {/* Content Canvas Preview */}
            <div className="w-[200px] h-[100px] bg-gradient-to-r from-cyan-400 to-blue-500 rounded-r-lg shadow-2xl flex items-center justify-center">
              <span className="text-gray-950 font-bold text-lg">{activePage.label}</span>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
