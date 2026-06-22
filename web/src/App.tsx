import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { useThemeStore } from './stores/themeStore';
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
import './index.css';

function App() {
  const { theme, colors } = useThemeStore();

  useEffect(() => {
    const root = document.documentElement;
    Object.entries(colors).forEach(([key, value]) => {
      root.style.setProperty(`--${key}`, value);
    });
    root.setAttribute('data-theme', theme);
  }, [theme, colors]);

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
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;