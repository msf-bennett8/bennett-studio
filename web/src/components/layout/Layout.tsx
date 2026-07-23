import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { useNotificationWatcher } from '../../hooks/useNotificationWatcher';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  useNotificationWatcher();
  return (
    <div className="flex h-screen" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      <Sidebar />
      <main className="flex-1 overflow-auto">
        {children}
      </main>
    </div>
  );
}

