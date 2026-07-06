import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { useThemeStore } from './stores/themeStore';
import { useRemoteConnectionStore } from './stores/remoteConnectionStore';
import { Layout } from './components/layout/Layout';
import { HomePage } from './pages/HomePage';
import { DatabasePage } from './pages/DatabasePage';
import { QueryPage } from './pages/QueryPage';
import { SchemaPage } from './pages/SchemaPage';
import { DataPage } from './pages/DataPage';
import { SharePage } from './pages/SharePage';
import { JoinSharePage } from './pages/JoinSharePage';
import { RemoteQueryPage } from './pages/RemoteQueryPage';
import { SettingsPage } from './pages/SettingsPage';
import { NotesPage } from './pages/NotesPage';
import { ShareLandingPage } from './pages/ShareLandingPage';
import './index.css';

function App() {
  const { theme, colors, _hasHydrated } = useThemeStore();

  useEffect(() => {
    if (!_hasHydrated) return; // Don't apply until persisted state is ready
    
    const root = document.documentElement;
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    root.setAttribute('data-theme', theme);
  }, [theme, colors, _hasHydrated]);

  // Prevent flash of wrong theme before hydration
  if (!_hasHydrated) {
    return <div className="bg-black min-h-screen" />; // Or null, or a loader
  }

  // Reconnect to persisted remote shares on app load
  useEffect(() => {
    // Use getState() to avoid dependency issues with zustand actions
    const { reconnectAll } = useRemoteConnectionStore.getState();
    reconnectAll();
  }, []);

  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/databases" element={<DatabasePage />} />
          <Route path="/query" element={<QueryPage />} />
          <Route path="/schema" element={<SchemaPage />} />
          <Route path="/data" element={<DataPage />} />
          <Route path="/share" element={<SharePage />} />
          <Route path="/join-share" element={<JoinSharePage />} />
          <Route path="/remote-query" element={<RemoteQueryPage />} />
          <Route path="/notes" element={<NotesPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/db/:code" element={<ShareLandingPage />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;