import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { LandingPage } from './pages/LandingPage';
import { DownloadPage } from './pages/DownloadPage';

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<LandingPage />} />
        <Route path="/download" element={<DownloadPage />} />
        <Route path="/app" element={
          <div className="min-h-screen flex items-center justify-center" style={{ backgroundColor: '#0a0a0a' }}>
            <div className="text-center max-w-md px-6">
              <div className="w-16 h-16 rounded-2xl flex items-center justify-center mx-auto mb-5"
                style={{ backgroundColor: 'rgba(0,212,170,0.1)', border: '1px solid rgba(0,212,170,0.2)' }}>
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: '#00d4aa' }}>
                  <ellipse cx="12" cy="5" rx="9" ry="3"/>
                  <path d="M3 5V19A9 3 0 0 0 21 19V5"/>
                  <path d="M3 12A9 3 0 0 0 21 12"/>
                </svg>
              </div>
              <h1 className="text-2xl font-bold text-white mb-3">Web IDE</h1>
              <p className="text-gray-400 mb-6 text-sm">
                The browser-based IDE is launching. Connect to your databases, run queries, and share — all without installing anything.
              </p>
              <div className="space-y-3">
                <a
                  href="/download"
                  className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-medium bg-[#00d4aa] text-[#0a0a0a] hover:bg-[#00e6b8] transition-all"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                  </svg>
                  Download Desktop App
                </a>
                <p className="text-xs text-gray-600">
                  Or use the <a href="https://share-bennett-studio.vercel.app" className="text-[#00d4aa] hover:underline">Share Viewer</a> to query shared databases
                </p>
              </div>
            </div>
          </div>
        } />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
