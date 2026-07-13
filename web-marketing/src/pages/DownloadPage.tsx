import { useState, useEffect } from 'react';
import { Database, Download, Apple, Monitor, Terminal, ArrowLeft, Check } from 'lucide-react';

const platforms = [
  {
    name: 'macOS',
    icon: <Apple size={20} />,
    archs: ['Apple Silicon (M1/M2/M3)', 'Intel'],
    filename: 'Bennett-Studio-0.1.0-universal.dmg',
    size: '48 MB'
  },
  {
    name: 'Windows',
    icon: <Monitor size={20} />,
    archs: ['x64', 'ARM64'],
    filename: 'Bennett-Studio-0.1.0-setup.exe',
    size: '52 MB'
  },
  {
    name: 'Linux',
    icon: <Terminal size={20} />,
    archs: ['AppImage', 'deb', 'rpm'],
    filename: 'bennett-studio-0.1.0.AppImage',
    size: '45 MB'
  }
];

function Navbar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener('scroll', onScroll);
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <nav className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
      scrolled ? 'bg-[#0a0a0a]/90 backdrop-blur-xl border-b border-[#2a2a2a]' : 'bg-transparent'
    }`}>
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
        <a href="/" className="flex items-center gap-2">
          <Database size={20} className="text-[#00d4aa]" />
          <span className="font-semibold text-white text-sm tracking-tight">Bennett Studio</span>
        </a>
        <a href="/" className="flex items-center gap-1 text-sm text-gray-400 hover:text-white transition-colors">
          <ArrowLeft size={14} /> Back to home
        </a>
      </div>
    </nav>
  );
}

export function DownloadPage() {
  const [downloading, setDownloading] = useState<string | null>(null);

  const handleDownload = (platform: string) => {
    setDownloading(platform);
    // In production, this would trigger the actual download
    setTimeout(() => setDownloading(null), 2000);
  };

  return (
    <div className="min-h-screen" style={{ backgroundColor: '#0a0a0a' }}>
      <Navbar />
      <div className="pt-32 pb-20 max-w-4xl mx-auto px-6">
        <div className="text-center mb-16">
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">Download Bennett Studio</h1>
          <p className="text-gray-400 max-w-lg mx-auto">Choose your platform. All builds include the query engine, desktop app, and sharing tools.</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-16">
          {platforms.map((p) => (
            <div key={p.name} className="p-6 rounded-2xl border border-[#2a2a2a] bg-[#1a1a1a] hover:border-[#3a3a3a] transition-all">
              <div className="w-12 h-12 rounded-xl flex items-center justify-center bg-[#00d4aa]/10 text-[#00d4aa] mb-4">
                {p.icon}
              </div>
              <h3 className="font-semibold text-white mb-1">{p.name}</h3>
              <p className="text-xs text-gray-500 mb-4">{p.archs.join(' · ')}</p>
              <button
                onClick={() => handleDownload(p.name)}
                disabled={downloading === p.name}
                className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium bg-[#00d4aa] text-[#0a0a0a] hover:bg-[#00e6b8] disabled:opacity-50 transition-all"
              >
                {downloading === p.name ? (
                  <><Check size={14} /> Starting download...</>
                ) : (
                  <><Download size={14} /> Download {p.size}</>
                )}
              </button>
              <p className="text-xs text-gray-600 mt-3 text-center font-mono">{p.filename}</p>
            </div>
          ))}
        </div>

        <div className="p-6 rounded-2xl border border-[#2a2a2a] bg-[#1a1a1a]">
          <h3 className="font-semibold text-white mb-3 flex items-center gap-2">
            <Terminal size={16} className="text-[#00d4aa]" />
            Install via CLI
          </h3>
          <pre className="p-4 rounded-lg bg-[#111111] border border-[#2a2a2a] font-mono text-sm text-gray-300 overflow-x-auto">
            <code>curl -fsSL https://bennett.studio/install.sh | sh</code>
          </pre>
          <p className="text-xs text-gray-500 mt-3">
            This installs the Bennett CLI, engine, and desktop app. Requires macOS 13+, Windows 10+, or Linux (glibc 2.31+).
          </p>
        </div>
      </div>
    </div>
  );
}
