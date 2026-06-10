import { useEffect } from 'react';
import { HashRouter, Routes, Route } from 'react-router-dom';
import { useThemeStore } from './stores/themeStore';
import { Layout } from './components/Layout';
import { HomePage } from './pages/HomePage';
import { DatabasePage } from './pages/DatabasePage';
import { QueryPage } from './pages/QueryPage';
import { SchemaPage } from './pages/SchemaPage';
import { SharePage } from './pages/SharePage';
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
    <HashRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/databases" element={<DatabasePage />} />
          <Route path="/query" element={<QueryPage />} />
          <Route path="/schema" element={<SchemaPage />} />
          <Route path="/share" element={<SharePage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}

export default App;

