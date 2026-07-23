import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { TitleBar } from './TitleBar';
import { useNotificationWatcher } from '../../hooks/useNotificationWatcher';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  useNotificationWatcher();
  return (
    <div className="flex flex-col h-screen" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      <TitleBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-auto">
          {children}
        </main>
      </div>
    </div>
  );
}

