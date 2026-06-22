import React, { useEffect } from 'react';
import { useShareStore } from '../../stores/shareStore';

export const GuestList: React.FC = () => {
  const { shares } = useShareStore();

  const totalGuests = shares.reduce((sum, s) => sum + s.guest_count, 0);

  if (totalGuests === 0) return null;

  return (
    <div className="guest-list mt-4">
      <h3 className="text-lg font-semibold mb-2">Connected Guests</h3>
      <p className="text-sm text-gray-600">
        {totalGuests} guest{totalGuests !== 1 ? 's' : ''} currently connected across {shares.length} share{shares.length !== 1 ? 's' : ''}
      </p>
    </div>
  );
};
