import { useLocation, Link } from 'react-router-dom'

interface LegalPageNavProps {
  currentPage: 'terms' | 'privacy' | 'faq'
}

export default function LegalPageNav({ currentPage }: LegalPageNavProps) {
  const pages = [
    { id: 'terms', label: 'Terms', path: '/terms' },
    { id: 'privacy', label: 'Privacy', path: '/privacy' },
    { id: 'faq', label: 'FAQ', path: '/faq' },
  ]

  return (
    <div className="mb-8 flex items-center justify-center gap-4">
      {pages.map((page, idx) => (
        <div key={page.id} className="flex items-center gap-4">
          {/* Clickable nav item with circle for active state */}
          <Link
            to={page.path}
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
  )
}
