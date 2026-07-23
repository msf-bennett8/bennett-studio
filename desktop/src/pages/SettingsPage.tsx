import { Outlet } from 'react-router-dom';

export function SettingsPage() {
  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-3xl font-bold mb-8" style={{ color: 'var(--textPrimary)' }}>Settings</h1>
      <div className="space-y-6">
        <Outlet />
      </div>
    </div>
  );
}
