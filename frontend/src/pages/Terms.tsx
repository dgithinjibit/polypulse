import { Link } from 'react-router-dom'

export default function Terms() {
  return (
    <div className="max-w-6xl mx-auto px-6 py-10 text-gray-100">
      <div className="bg-slate-950/90 border border-slate-800 rounded-3xl p-8 shadow-xl shadow-black/20">
        <h1 className="text-4xl font-bold mb-4 text-white">Terms & Conditions</h1>
        <p className="text-sm text-slate-400 mb-8">Last Updated: April 23, 2026</p>

        <p className="mb-6 leading-7 text-slate-300">
          Welcome to PolyPulse! These Terms & Conditions ("Terms") govern your use of the PolyPulse platform (the "Service"), a decentralized prediction market application built on the Stellar blockchain. By accessing or using PolyPulse, you agree to be bound by these Terms. If you do not agree, please do not use the Service.
        </p>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">1. Acceptance of Terms</h2>
          <p className="text-slate-300 leading-7">
            By creating an account or using PolyPulse, you accept these Terms and our <Link to="/privacy" className="text-cyan-400 hover:underline">Privacy Policy</Link>. These Terms apply to all users, including visitors and registered users.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">2. Description of Service</h2>
          <p className="text-slate-300 leading-7">
            PolyPulse allows users to participate in prediction markets, challenges, polls, and wagers using the Logarithmic Market Scoring Rule (LMSR). Transactions are processed via the Stellar blockchain, and payments may involve M-Pesa for fiat conversions. The Service is provided "as is" without warranties.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">3. User Eligibility</h2>
          <p className="text-slate-300 leading-7">
            You must be at least 18 years old and legally capable of entering contracts. Users in jurisdictions where prediction markets or gambling are prohibited are not eligible. PolyPulse does not endorse or facilitate illegal activities.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">4. Account Registration</h2>
          <p className="text-slate-300 leading-7">
            To use certain features, you must create an account via JWT, Web3Auth, or social login. You are responsible for maintaining the confidentiality of your credentials and wallet keys. Notify us immediately of unauthorized access.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">5. User Conduct</h2>
          <p className="text-slate-300 leading-7 mb-4">
            You agree not to:
          </p>
          <ul className="list-disc list-inside text-slate-300 leading-7 space-y-2 ml-5">
            <li>Engage in fraudulent activities, spam, or market manipulation.</li>
            <li>Use the Service for illegal purposes, including money laundering or harassment.</li>
            <li>Attempt to hack, disrupt, or reverse-engineer the platform.</li>
            <li>Post harmful, offensive, or misleading content.</li>
          </ul>
          <p className="text-slate-300 leading-7 mt-4">
            Violation may result in account suspension or termination.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">6. Payments and Transactions</h2>
          <p className="text-slate-300 leading-7">
            Payments are processed via Stellar blockchain or M-Pesa. All transactions are irreversible. PolyPulse is not responsible for losses due to market volatility, user error, or blockchain issues. Fees may apply for transactions.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">7. Intellectual Property</h2>
          <p className="text-slate-300 leading-7">
            All content, trademarks, and code related to PolyPulse are owned by us or licensed. You may not copy, distribute, or use them without permission.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">8. Disclaimers and Limitation of Liability</h2>
          <p className="text-slate-300 leading-7">
            PolyPulse is provided "as is" without warranties. We are not liable for financial losses, data breaches, or third-party actions. Our total liability is limited to the amount paid by you in the last 12 months.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">9. Indemnification</h2>
          <p className="text-slate-300 leading-7">
            You agree to indemnify PolyPulse against claims arising from your use of the Service, including violations of these Terms.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">10. Termination</h2>
          <p className="text-slate-300 leading-7">
            We may terminate your account for violations. You may delete your account at any time.
          </p>
        </section>

        <section className="mb-8">
          <h2 className="text-2xl font-semibold mb-3 text-white">11. Governing Law</h2>
          <p className="text-slate-300 leading-7">
            These Terms are governed by the laws of Kenya, with disputes resolved in Kenyan courts.
          </p>
        </section>

        <section>
          <h2 className="text-2xl font-semibold mb-3 text-white">12. Changes to Terms</h2>
          <p className="text-slate-300 leading-7 mb-4">
            We may update these Terms. Continued use constitutes acceptance.
          </p>
          <p className="text-slate-300 leading-7">
            For questions, contact us at <a href="mailto:support@polypulse.app" className="text-cyan-400 hover:underline">support@polypulse.app</a>.
          </p>
        </section>
      </div>
    </div>
  )
}
