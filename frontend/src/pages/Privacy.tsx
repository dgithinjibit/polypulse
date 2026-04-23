export default function Privacy() {
  return (
    <div className="max-w-6xl mx-auto px-6 py-10 text-gray-100">
      <div className="bg-slate-950/90 border border-slate-800 rounded-3xl p-8 shadow-xl shadow-black/20">
        <h1 className="text-4xl font-bold mb-4 text-white">Privacy Policy</h1>
        <p className="text-sm text-slate-400 mb-8">Last Updated: April 23, 2026</p>

        <p className="mb-6 leading-7 text-slate-300">
          PolyPulse ("we," "us," or "our") is committed to protecting your privacy. This Privacy Policy explains how we collect, use, and safeguard your information when you use our Service.
        </p>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">1. Information We Collect</h2>
          <ul className="list-disc list-inside text-slate-300 leading-7 space-y-2 ml-5">
            <li><strong>Personal Information:</strong> Email, wallet addresses, transaction data, and profile details provided during registration.</li>
            <li><strong>Usage Data:</strong> IP addresses, browser type, device info, and analytics from your interactions.</li>
            <li><strong>Payment Data:</strong> Transaction amounts and M-Pesa/Stellar details (processed securely via third parties).</li>
            <li><strong>Communications:</strong> Comments, chat messages, and notification preferences.</li>
          </ul>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">2. How We Use Your Information</h2>
          <ul className="list-disc list-inside text-slate-300 leading-7 space-y-2 ml-5">
            <li>To provide and improve the Service.</li>
            <li>To process payments and resolve disputes.</li>
            <li>To comply with legal obligations and prevent fraud.</li>
            <li>To send notifications and updates (with opt-out options).</li>
          </ul>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">3. Information Sharing</h2>
          <p className="text-slate-300 leading-7 mb-4">
            We do not sell your data. We may share with:
          </p>
          <ul className="list-disc list-inside text-slate-300 leading-7 space-y-2 ml-5">
            <li>Service providers (e.g., Stellar, M-Pesa) for processing.</li>
            <li>Legal authorities if required by law.</li>
            <li>In case of business transfers.</li>
          </ul>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">4. Data Security</h2>
          <p className="text-slate-300 leading-7">
            We use encryption, secure servers, and access controls. However, no system is 100% secure. You are responsible for your wallet security.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">5. Data Retention</h2>
          <p className="text-slate-300 leading-7">
            We retain data as long as your account is active or as required by law. You can request deletion.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">6. Your Rights</h2>
          <p className="text-slate-300 leading-7 mb-4">
            Depending on your jurisdiction, you may have rights to:
          </p>
          <ul className="list-disc list-inside text-slate-300 leading-7 space-y-2 ml-5">
            <li>Access, correct, or delete your data.</li>
            <li>Opt-out of marketing.</li>
            <li>Data portability.</li>
          </ul>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">7. Cookies and Tracking</h2>
          <p className="text-slate-300 leading-7">
            We use cookies for functionality and analytics. You can disable them in your browser.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">8. International Transfers</h2>
          <p className="text-slate-300 leading-7">
            Data may be transferred globally. We ensure adequate protections.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">9. Children's Privacy</h2>
          <p className="text-slate-300 leading-7">
            We do not collect data from children under 18.
          </p>
        </section>

        <section>
          <h2 className="text-2xl font-semibold mb-3 text-white">10. Changes to Policy</h2>
          <p className="text-slate-300 leading-7 mb-4">
            We may update this Policy. We will notify you of material changes.
          </p>
          <p className="text-slate-300 leading-7">
            For privacy questions, email <a href="mailto:privacy@polypulse.app" className="text-cyan-400 hover:underline">privacy@polypulse.app</a>.
          </p>
        </section>
      </div>
    </div>
  )
}
