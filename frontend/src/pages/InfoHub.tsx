import ZoomReveal from '../components/ZoomReveal';

export default function InfoHub() {
  const buttons = [
    {
      id: 'faq',
      label: 'FAQ',
      icon: '?',
      content: (
        <div className="space-y-4">
          <div>
            <h3 className="font-semibold">What is PolyPulse?</h3>
            <p className="text-gray-600">PolyPulse is a prediction market platform built on Stellar.</p>
          </div>
          <div>
            <h3 className="font-semibold">How do I get started?</h3>
            <p className="text-gray-600">Connect your wallet and start trading on markets.</p>
          </div>
        </div>
      )
    },
    {
      id: 'terms',
      label: 'Terms & Conditions',
      icon: '📄',
      content: (
        <div className="space-y-2 text-sm text-gray-700">
          <p>By using PolyPulse, you agree to these terms...</p>
          <p>1. You must be 18 years or older</p>
          <p>2. You are responsible for your wallet security</p>
          <p>3. Trading involves risk</p>
        </div>
      )
    },
    {
      id: 'leaderboard',
      label: 'Leaderboard',
      icon: '🏆',
      content: (
        <div className="space-y-2">
          <div className="flex justify-between p-2 bg-gray-100 rounded">
            <span>1. User123</span>
            <span className="font-bold">1,250 pts</span>
          </div>
          <div className="flex justify-between p-2 bg-gray-50 rounded">
            <span>2. Trader456</span>
            <span className="font-bold">980 pts</span>
          </div>
        </div>
      )
    }
  ];

  return <ZoomReveal buttons={buttons} />;
}
