import { useState } from 'react';

interface ZoomRevealProps {
  buttons: Array<{
    id: string;
    label: string;
    icon?: string;
    content: React.ReactNode;
  }>;
}

export default function ZoomReveal({ buttons }: ZoomRevealProps) {
  const [activeId, setActiveId] = useState<string | null>(null);

  const active = buttons.find(b => b.id === activeId);

  return (
    <div className="relative w-full h-screen flex items-center justify-center overflow-hidden bg-gray-50">
      {/* Navigation Buttons */}
      <div className={`flex gap-4 transition-all duration-500 ${activeId ? 'scale-50 opacity-0' : 'scale-100 opacity-100'}`}>
        {buttons.map((btn, idx) => (
          <button
            key={btn.id}
            onClick={() => setActiveId(btn.id)}
            className="w-16 h-16 rounded-full bg-blue-600 text-white font-bold text-xl hover:bg-blue-700 transition-colors shadow-lg"
          >
            {btn.icon || idx + 1}
          </button>
        ))}
      </div>

      {/* Zoom Reveal Component */}
      {activeId && (
        <div className={`absolute inset-0 flex items-center justify-center transition-all duration-700 ${activeId ? 'scale-100 opacity-100' : 'scale-0 opacity-0'}`}>
          <div className="flex items-center gap-0">
            {/* Button Circle */}
            <div className="w-16 h-16 rounded-full bg-blue-600 text-white font-bold text-xl flex items-center justify-center shadow-lg z-10">
              {active?.icon || buttons.findIndex(b => b.id === activeId) + 1}
            </div>
            
            {/* Connector Rectangle */}
            <div className="w-12 h-16 bg-blue-600"></div>
            
            {/* Content Canvas */}
            <div className="w-[600px] h-[400px] bg-white rounded-r-lg shadow-2xl p-6 overflow-auto">
              <button
                onClick={() => setActiveId(null)}
                className="float-right text-gray-500 hover:text-gray-700 text-2xl font-bold"
              >
                ×
              </button>
              <h2 className="text-2xl font-bold mb-4">{active?.label}</h2>
              {active?.content}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
