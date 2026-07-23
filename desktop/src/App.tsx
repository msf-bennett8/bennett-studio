import { useEffect } from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useThemeStore } from './stores/themeStore';
import { useUIPreferencesStore } from './stores/uiPreferencesStore';
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
import { GeneralSettings } from './pages/settings/GeneralSettings';
import { AppearanceSettings } from './pages/settings/AppearanceSettings';
import { NotificationSettings } from './pages/settings/NotificationSettings';
import { PrivacySettings } from './pages/settings/PrivacySettings';
import { ApiKeySettings } from './pages/settings/ApiKeySettings';
import { GuestsSettings } from './pages/settings/GuestsSettings';
import { ActivitySettings } from './pages/settings/ActivitySettings';
import { ResourcesSettings } from './pages/settings/ResourcesSettings';
import { NotesPage } from './pages/NotesPage';
import { ShareLandingPage } from './pages/ShareLandingPage';
import './index.css';

function App() {
  const { theme, colors, _hasHydrated } = useThemeStore();
  const { fontScale, reduceMotion } = useUIPreferencesStore();

  useEffect(() => {
    if (!_hasHydrated) return; // Don't apply until persisted state is ready

    const root = document.documentElement;
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    root.setAttribute('data-theme', theme);
  }, [theme, colors, _hasHydrated]);

  useEffect(() => {
    document.documentElement.style.fontSize = `${fontScale}%`;
  }, [fontScale]);

  useEffect(() => {
    const styleId = 'reduce-motion-style';
    let styleEl = document.getElementById(styleId) as HTMLStyleElement | null;
    if (reduceMotion) {
      if (!styleEl) {
        styleEl = document.createElement('style');
        styleEl.id = styleId;
        styleEl.textContent = '*, *::before, *::after { animation-duration: 0.001ms !important; animation-iteration-count: 1 !important; transition-duration: 0.001ms !important; }';
        document.head.appendChild(styleEl);
      }
    } else if (styleEl) {
      styleEl.remove();
    }
  }, [reduceMotion]);

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
    <HashRouter>
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
          <Route path="/settings" element={<SettingsPage />}>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<GeneralSettings />} />
            <Route path="appearance" element={<AppearanceSettings />} />
            <Route path="notifications" element={<NotificationSettings />} />
            <Route path="privacy" element={<PrivacySettings />} />
            <Route path="api-keys" element={<ApiKeySettings />} />
            <Route path="guests" element={<GuestsSettings />} />
            <Route path="activity" element={<ActivitySettings />} />
            <Route path="resources" element={<ResourcesSettings />} />
            {/* Engine Info merged into General — old links keep working */}
            <Route path="engine" element={<Navigate to="/settings/general" replace />} />
          </Route>
          <Route path="/db/:code" element={<ShareLandingPage />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}

export default App;

