import { useState, useEffect } from 'react';
import { Bell, Users, WifiOff } from 'lucide-react';
import { useNotificationPreferencesStore } from '../../stores/notificationPreferencesStore';

export function NotificationSettings() {
  const { enabled, setEnabled, guestConnected, setGuestConnected, engineOffline, setEngineOffline } = useNotificationPreferencesStore();
  const [permission, setPermission] = useState<NotificationPermission>(
    typeof Notification !== 'undefined' ? Notification.permission : 'denied'
  );

  useEffect(() => {
    if (enabled && typeof Notification !== 'undefined' && Notification.permission === 'default') {
      Notification.requestPermission().then(setPermission);
    }
  }, [enabled]);

  const handleMasterToggle = async () => {
    if (!enabled && typeof Notification !== 'undefined' && Notification.permission === 'default') {
      const result = await Notification.requestPermission();
      setPermission(result);
      if (result !== 'granted') return;
    }
    setEnabled(!enabled);
  };

  return (
    <div className="space-y-6">
      <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
        <div className="flex items-center gap-3 mb-4">
          <Bell size={20} style={{ color: 'var(--accentWarning)' }} />
          <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Notifications</h2>
        </div>

        <div className="flex items-center justify-between mb-4 pb-4" style={{ borderBottom: '1px solid var(--borderDefault)' }}>
          <div>
            <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Enable browser notifications</p>
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
              {permission === 'denied'
                ? 'Notifications are blocked in your browser settings — enable them there first.'
                : 'Requires one-time browser permission.'}
            </p>
          </div>
          <button
            onClick={handleMasterToggle}
            disabled={permission === 'denied'}
            className="w-12 h-6 rounded-full transition-all relative disabled:opacity-50"
            style={{ backgroundColor: enabled ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}
          >
            <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: enabled ? '26px' : '2px' }} />
          </button>
        </div>

        <div className="space-y-4" style={{ opacity: enabled ? 1 : 0.5, pointerEvents: enabled ? 'auto' : 'none' }}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Users size={16} style={{ color: 'var(--textMuted)' }} />
              <div>
                <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Guest connected</p>
                <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Notify when someone joins one of your shares</p>
              </div>
            </div>
            <button onClick={() => setGuestConnected(!guestConnected)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: guestConnected ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
              <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: guestConnected ? '26px' : '2px' }} />
            </button>
          </div>

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <WifiOff size={16} style={{ color: 'var(--textMuted)' }} />
              <div>
                <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>Engine offline</p>
                <p className="text-xs" style={{ color: 'var(--textMuted)' }}>Notify if the local engine stops responding</p>
              </div>
            </div>
            <button onClick={() => setEngineOffline(!engineOffline)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: engineOffline ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
              <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: engineOffline ? '26px' : '2px' }} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
