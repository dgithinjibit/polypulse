import { useState } from 'react';
import { EncryptionService } from '../services/encryption';

interface BetCreationFormProps {
  onSuccess: (betId: string, shareableUrl: string) => void;
  onCancel: () => void;
}

interface BetFormData {
  question: string;
  stakeAmount: number;
  endTime: Date;
}

export function BetCreationForm({ onSuccess, onCancel }: BetCreationFormProps) {
  const [formData, setFormData] = useState<BetFormData>({
    question: '',
    stakeAmount: 0,
    endTime: new Date(),
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (formData.question.length < 10) {
      newErrors.question = 'Question must be at least 10 characters';
    }
    if (formData.question.length > 200) {
      newErrors.question = 'Question must be at most 200 characters';
    }
    if (!formData.question.includes('?')) {
      newErrors.question = 'Question must end with a question mark';
    }

    if (formData.stakeAmount <= 0) {
      newErrors.stakeAmount = 'Stake amount must be positive';
    }

    if (formData.endTime <= new Date()) {
      newErrors.endTime = 'End time must be in the future';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    setLoading(true);

    try {
      // Call API to create bet
      const response = await fetch('/api/v1/p2p-bets', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          question: formData.question,
          stake_amount: formData.stakeAmount,
          end_time: formData.endTime.toISOString(),
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to create bet');
      }

      const data = await response.json();

      // TODO: Call smart contract create_bet via Freighter wallet

      onSuccess(data.bet_id, data.shareable_url);
    } catch (error) {
      console.error('Error creating bet:', error);
      setErrors({ submit: 'Failed to create bet. Please try again.' });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-white rounded-lg shadow-lg p-6 max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold mb-6">Create Your Bet</h2>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="question" className="block text-sm font-medium text-gray-700 mb-1">
            Question
          </label>
          <input
            type="text"
            id="question"
            value={formData.question}
            onChange={(e) => setFormData({ ...formData, question: e.target.value })}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Will it rain tomorrow?"
          />
          {errors.question && <p className="text-red-500 text-sm mt-1">{errors.question}</p>}
        </div>

        <div>
          <label htmlFor="stakeAmount" className="block text-sm font-medium text-gray-700 mb-1">
            Stake Amount (XLM)
          </label>
          <input
            type="number"
            id="stakeAmount"
            value={formData.stakeAmount}
            onChange={(e) => setFormData({ ...formData, stakeAmount: parseFloat(e.target.value) })}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="10"
            step="0.01"
          />
          {errors.stakeAmount && <p className="text-red-500 text-sm mt-1">{errors.stakeAmount}</p>}
        </div>

        <div>
          <label htmlFor="endTime" className="block text-sm font-medium text-gray-700 mb-1">
            End Time
          </label>
          <input
            type="datetime-local"
            id="endTime"
            value={formData.endTime.toISOString().slice(0, 16)}
            onChange={(e) => setFormData({ ...formData, endTime: new Date(e.target.value) })}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          {errors.endTime && <p className="text-red-500 text-sm mt-1">{errors.endTime}</p>}
        </div>

        {errors.submit && <p className="text-red-500 text-sm">{errors.submit}</p>}

        <div className="flex gap-4 pt-4">
          <button
            type="submit"
            disabled={loading}
            className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            {loading ? 'Creating...' : 'Create Bet'}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="flex-1 bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
