import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useShareStore } from '../../stores/shareStore';

export const JoinShare: React.FC = () => {
  const [shareUrl, setShareUrl] = useState('');
  const [connecting, setConnecting] = useState(false);
  const { connectToShare } = useShareStore();
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!shareUrl.trim()) return;

    setConnecting(true);
    try {
      const success = await connectToShare(shareUrl.trim());
      if (success) {
        navigate('/remote-query');
      }
    } finally {
      setConnecting(false);
    }
  };

  return (
    <div className="join-share max-w-md mx-auto p-6">
      <h2 className="text-2xl font-bold mb-4">Join Shared Database</h2>
      <p className="text-gray-600 mb-4">
        Enter a share link to connect to a remote database.
      </p>
      
      <form onSubmit={handleSubmit}>
        <input
          type="url"
          value={shareUrl}
          onChange={(e) => setShareUrl(e.target.value)}
          placeholder="https://share.bennett.studio/db/ACQPFDAQ7P?t=..."
          className="w-full p-3 border rounded mb-3"
          required
        />
        <button
          type="submit"
          disabled={connecting}
          className="w-full py-3 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {connecting ? 'Connecting...' : 'Connect'}
        </button>
      </form>
    </div>
  );
};
