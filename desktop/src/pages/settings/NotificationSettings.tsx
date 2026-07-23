import { useState } from 'react';
import { Bell } from 'lucide-react';

export function NotificationSettings() {
  const [notifications, setNotifications] = useState(true);
  const [autoUpdate, setAutoUpdate] = useState(true);

  return (
    <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
      <div className="flex items-center gap-3 mb-4">
        <Bell size={20} style={{ color: 'var(--accentWarning)' }} />
        <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Notifications</h2>
      </div>
      <div className="space-y-4">
        {[
          { label: 'Enable Notifications', desc: 'Get alerts for query completions and share requests', state: notifications, setState: setNotifications },
          { label: 'Auto-update Check', desc: 'Automatically check for new versions', state: autoUpdate, setState: setAutoUpdate },
        ].map((item, i) => (
          <div key={i} className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{item.label}</p>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>{item.desc}</p>
            </div>
            <button onClick={() => item.setState(!item.state)} className="w-12 h-6 rounded-full transition-all relative" style={{ backgroundColor: item.state ? 'var(--accentPrimary)' : 'var(--bgTertiary)' }}>
              <div className="w-5 h-5 rounded-full absolute top-0.5 transition-all" style={{ backgroundColor: 'var(--textInverse)', left: item.state ? '26px' : '2px' }} />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
