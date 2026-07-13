import { useEffect, useState } from 'react';
import { Database, Zap, Share2, Lock, Terminal, Download, Github, ExternalLink } from 'lucide-react';

const features = [
  {
    icon: <Terminal size={20} />,
    title: 'SQL Editor',
    desc: 'Syntax-highlighted query editor with autocomplete, query history, and result export.'
  },
  {
    icon: <Share2 size={20} />,
    title: 'Instant Sharing',
    desc: 'Generate share links in one click. Recipients query your data without setting up anything.'
  },
  {
    icon: <Zap size={20} />,
    title: 'P2P + Relay',
    desc: 'Direct peer-to-peer when possible. Relay fallback when behind NAT. Always fast.'
  },
  {
    icon: <Lock size={20} />,
    title: 'JWT-Secured',
    desc: 'Every share is a signed JWT with table-level permissions and automatic expiry.'
  },
  {
    icon: <Database size={20} />,
    title: 'Schema Explorer',
    desc: 'Browse tables, columns, relationships, and indexes with a clean tree view.'
  },
  {
    icon: <Terminal size={20} />,
    title: 'SDK + CLI',
    desc: 'TypeScript SDK and CLI for programmatic access. Works in Node, Deno, and Bun.'
  }
];

const codeExample = `import { BennettClient } from '@bennettstudio/sdk';

const db = await BennettClient.fromShareUrl(
  'https://share.bennett.studio/db/ABC123?t=eyJ...'
);

const users = await db.query(
  'SELECT * FROM users WHERE active = true LIMIT 10'
);

console.log(users.rows);
// → [{ id: 1, name: 'Alice', ... }, ...]`;

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
        <div className="hidden md:flex items-center gap-6">
          <a href="#features" className="text-sm text-gray-400 hover:text-white transition-colors">Features</a>
            <a href="https://share-bennett-studio.vercel.app" className="text-sm text-gray-400 hover:text-white transition-colors">Share Viewer</a>
            <a href="https://app-bennett-studio.vercel.app" className="text-sm text-gray-400 hover:text-white transition-colors">Open in Browser</a>
            <a href="/download" className="text-sm text-gray-400 hover:text-white transition-colors">Download</a>
          <a href="https://github.com" className="text-sm text-gray-400 hover:text-white transition-colors flex items-center gap-1">
            <Github size={14} /> GitHub
          </a>
        </div>
        <div className="flex items-center gap-3">
          <a
            href="/download"
            className="hidden sm:flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium bg-[#1a1a1a] text-white border border-[#2a2a2a] hover:border-[#3a3a3a] hover:bg-[#252525] transition-all"
          >
            <Download size={14} />
            Download
          </a>
        </div>
      </div>
    </nav>
  );
}

function Hero() {
  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden" style={{ backgroundColor: '#0a0a0a' }}>
      <div className="absolute inset-0 opacity-30">
        <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-[#00d4aa] rounded-full blur-[128px]" />
        <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-[#6b8aff] rounded-full blur-[128px]" />
      </div>
      <div className="relative z-10 max-w-4xl mx-auto px-6 text-center pt-20">
        <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium mb-8 border border-[#2a2a2a] bg-[#1a1a1a] text-gray-400">
          <span className="w-1.5 h-1.5 rounded-full bg-[#00d4aa] animate-pulse" />
          Now in public beta
        </div>
        <h1 className="text-5xl md:text-7xl font-bold text-white tracking-tight mb-6 leading-tight">
          Query any database.<br />
          <span className="text-[#00d4aa]">Share results instantly.</span>
        </h1>
        <p className="text-lg md:text-xl text-gray-400 max-w-2xl mx-auto mb-10 leading-relaxed">
          A modern database IDE with built-in sharing, real-time collaboration, and zero-config deployment. Connect locally, share globally.
        </p>
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mb-16">
          <a
            href="/download"
            className="flex items-center gap-2 px-6 py-3 rounded-xl text-sm font-semibold bg-[#00d4aa] text-[#0a0a0a] hover:bg-[#00e6b8] transition-all"
          >
            <Download size={16} />
            Download for macOS
          </a>
          <a
            href="https://app-bennett-studio.vercel.app"
            className="flex items-center gap-2 px-6 py-3 rounded-xl text-sm font-medium text-gray-400 hover:text-white transition-all"
          >
            <ExternalLink size={16} />
            Open in Browser
          </a>
          <a
            href="https://share-bennett-studio.vercel.app"
            className="flex items-center gap-2 px-6 py-3 rounded-xl text-sm font-medium text-gray-400 hover:text-white transition-all"
          >
            <ExternalLink size={16} />
            Open Share Viewer
          </a>
        </div>
        <div className="max-w-3xl mx-auto rounded-2xl overflow-hidden border border-[#2a2a2a] bg-[#111111] shadow-2xl">
          <div className="flex items-center gap-2 px-4 py-3 border-b border-[#2a2a2a] bg-[#1a1a1a]">
            <div className="w-3 h-3 rounded-full bg-[#ff5f57]" />
            <div className="w-3 h-3 rounded-full bg-[#febc2e]" />
            <div className="w-3 h-3 rounded-full bg-[#28c840]" />
            <span className="ml-2 text-xs text-gray-500 font-mono">query.sql</span>
          </div>
          <pre className="p-6 text-left font-mono text-sm leading-relaxed overflow-x-auto">
            <code className="text-gray-300">
              <span className="text-[#ff7b72]">SELECT</span>{' '}
              <span className="text-[#d2a8ff]">u.name</span>,{' '}
              <span className="text-[#d2a8ff]">u.email</span>,{' '}
              <span className="text-[#d2a8ff]">o.total</span>{'\n'}
              <span className="text-[#ff7b72]">FROM</span>{' '}
              <span className="text-[#ffa657]">users</span>{' '}
              <span className="text-[#ff7b72]">u</span>{'\n'}
              <span className="text-[#ff7b72]">JOIN</span>{' '}
              <span className="text-[#ffa657]">orders</span>{' '}
              <span className="text-[#ff7b72]">o</span>{' '}
              <span className="text-[#ff7b72]">ON</span>{' '}
              <span className="text-[#d2a8ff]">u.id</span>{' '}
              <span className="text-[#ff7b72]">=</span>{' '}
              <span className="text-[#d2a8ff]">o.user_id</span>{'\n'}
              <span className="text-[#ff7b72]">WHERE</span>{' '}
              <span className="text-[#d2a8ff]">u.active</span>{' '}
              <span className="text-[#ff7b72]">=</span>{' '}
              <span className="text-[#79c0ff]">true</span>{'\n'}
              <span className="text-[#ff7b72]">ORDER BY</span>{' '}
              <span className="text-[#d2a8ff]">o.total</span>{' '}
              <span className="text-[#ff7b72]">DESC</span>{'\n'}
              <span className="text-[#ff7b72]">LIMIT</span>{' '}
              <span className="text-[#79c0ff]">10</span>;
            </code>
          </pre>
        </div>
      </div>
    </section>
  );
}

function Features() {
  return (
    <section id="features" className="py-24" style={{ backgroundColor: '#111111' }}>
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Everything you need</h2>
          <p className="text-gray-400 max-w-xl mx-auto">Built for developers who want to move fast without compromising on data access control.</p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((f, i) => (
            <div key={i} className="p-6 rounded-2xl border border-[#2a2a2a] bg-[#1a1a1a] hover:border-[#3a3a3a] transition-all group">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center bg-[#00d4aa]/10 text-[#00d4aa] mb-4 group-hover:scale-110 transition-transform">
                {f.icon}
              </div>
              <h3 className="font-semibold text-white mb-2">{f.title}</h3>
              <p className="text-sm text-gray-400 leading-relaxed">{f.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function SDKSection() {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    navigator.clipboard.writeText(codeExample);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section className="py-24" style={{ backgroundColor: '#0a0a0a' }}>
      <div className="max-w-6xl mx-auto px-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
          <div>
            <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Programmatic access</h2>
            <p className="text-gray-400 mb-6 leading-relaxed">
              Use the TypeScript SDK to query shared databases from your applications. No connection strings, no VPNs — just a share URL.
            </p>
            <ul className="space-y-3 mb-8">
              {['Type-safe queries with auto-completion', 'Works in Node.js, Deno, and Bun', 'Zero dependencies, < 15KB gzipped'].map((item, i) => (
                <li key={i} className="flex items-center gap-3 text-sm text-gray-300">
                  <span className="w-5 h-5 rounded-full bg-[#00d4aa]/10 flex items-center justify-center text-[#00d4aa] text-xs">✓</span>
                  {item}
                </li>
              ))}
            </ul>
            <div className="flex items-center gap-4">
              <code className="px-4 py-2 rounded-lg bg-[#1a1a1a] border border-[#2a2a2a] text-sm font-mono text-gray-300">
                npm install @bennettstudio/sdk
              </code>
            </div>
          </div>
          <div className="rounded-2xl overflow-hidden border border-[#2a2a2a] bg-[#111111]">
            <div className="flex items-center justify-between px-4 py-3 border-b border-[#2a2a2a] bg-[#1a1a1a]">
              <span className="text-xs text-gray-500 font-mono">example.ts</span>
              <button onClick={copy} className="text-xs text-gray-500 hover:text-white transition-colors">
                {copied ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <pre className="p-6 text-sm font-mono leading-relaxed overflow-x-auto">
              <code className="text-gray-300">{codeExample}</code>
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="py-12 border-t border-[#2a2a2a]" style={{ backgroundColor: '#0a0a0a' }}>
      <div className="max-w-6xl mx-auto px-6">
        <div className="flex flex-col md:flex-row items-center justify-between gap-6">
          <div className="flex items-center gap-2">
            <Database size={18} className="text-[#00d4aa]" />
            <span className="font-semibold text-white text-sm">Bennett Studio</span>
          </div>
          <div className="flex items-center gap-6 text-sm text-gray-500">
            <a href="https://share-bennett-studio.vercel.app" className="hover:text-white transition-colors">Share Viewer</a>
            <a href="/download" className="hover:text-white transition-colors">Download</a>
            <a href="https://github.com" className="hover:text-white transition-colors">GitHub</a>
          </div>
          <p className="text-xs text-gray-600">© 2026 Bennett Studio. Open source under MIT.</p>
        </div>
      </div>
    </footer>
  );
}

export function LandingPage() {
  return (
    <div className="min-h-screen" style={{ backgroundColor: '#0a0a0a' }}>
      <Navbar />
      <Hero />
      <Features />
      <SDKSection />
      <Footer />
    </div>
  );
}
