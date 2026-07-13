import { useState, useEffect, useRef, useCallback } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { ShareLandingPage } from './pages/ShareLandingPage';
import { MarketingShareLandingPage } from './pages/MarketingShareLandingPage';
import { Database, ArrowUp } from 'lucide-react';
import './index.css';

function HomePage() {
  const [scrollY, setScrollY] = useState(0);
  const [showFloating, setShowFloating] = useState(false);
  const [shareCode, setShareCode] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const handleScroll = () => {
      const y = window.scrollY;
      setScrollY(y);
      // Show floating button after scrolling past 50% of viewport
      setShowFloating(y > window.innerHeight * 0.5);
    };

    window.addEventListener('scroll', handleScroll, { passive: true });
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const scrollToTop = useCallback(() => {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }, []);

  const scrollToLanding = useCallback(() => {
    window.scrollTo({ top: window.innerHeight * 0.8, behavior: 'smooth' });
  }, []);

  const handleNavigate = useCallback(() => {
    const code = shareCode.trim().toUpperCase();
    if (code) navigate(`/db/${code}`);
  }, [shareCode, navigate]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') handleNavigate();
  }, [handleNavigate]);

  // Opacity calculations
  const heroOpacity = Math.max(0, 1 - scrollY / (window.innerHeight * 0.6));
  const landingOpacity = Math.min(1, Math.max(0, (scrollY - window.innerHeight * 0.3) / (window.innerHeight * 0.5)));

  return (
    <div ref={containerRef} className="relative">
      {/* Hero Section — Fixed, fades out on scroll */}
      <div
        className="fixed inset-0 flex flex-col items-center justify-center px-6 z-10"
        style={{
          backgroundColor: 'var(--bgPrimary, #0f172a)',
          opacity: heroOpacity,
          pointerEvents: heroOpacity > 0.1 ? 'auto' : 'none',
          transform: `translateY(${-scrollY * 0.3}px)`,
          transition: 'opacity 0.1s ease-out',
        }}
      >
        <div className="text-center max-w-md w-full p-8 rounded-2xl" style={{ backgroundColor: 'var(--surfaceDefault, #1e293b)', border: '1px solid var(--borderDefault, #334155)' }}>
          <h1 className="text-2xl font-bold mb-2" style={{ color: 'var(--textPrimary, #f8fafc)' }}>
            Bennett Studio
          </h1>
          <p className="text-sm mb-6" style={{ color: 'var(--textSecondary, #94a3b8)' }}>
            Enter a share code to access a shared database
          </p>
          <div className="flex items-center gap-2">
            <input
              type="text"
              placeholder="e.g., ACQPFDAQ7P"
              value={shareCode}
              onChange={(e) => setShareCode(e.target.value)}
              onKeyDown={handleKeyDown}
              className="flex-1 px-4 py-3 rounded-xl text-sm outline-none transition-all"
              style={{
                backgroundColor: 'var(--bgSecondary, #0f172a)',
                color: 'var(--textPrimary, #f8fafc)',
                border: '1px solid var(--borderDefault, #334155)'
              }}
            />
            <button
              onClick={handleNavigate}
              className="w-9 h-9 rounded-xl flex items-center justify-center transition-all hover:scale-105 active:scale-95"
              style={{
                backgroundColor: 'var(--accentPrimary, #00d4aa)',
                color: 'var(--textInverse, #0a0a0a)',
              }}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="22" y1="2" x2="11" y2="13" />
                <polygon points="22 2 15 22 11 13 2 9 22 2" />
              </svg>
            </button>
          </div>
          <p className="text-xs mt-4" style={{ color: 'var(--textMuted, #64748b)' }}>
            Or use a full share link with token
          </p>
        </div>
      </div>

      {/* Spacer to allow scrolling */}
      <div style={{ height: '200vh' }} />

      {/* Bottom bar — Fixed at bottom, fades with hero */}
      <div
        className="fixed bottom-0 left-0 right-0 px-6 pb-6 z-20"
        style={{
          opacity: heroOpacity,
          pointerEvents: heroOpacity > 0.1 ? 'auto' : 'none',
          transition: 'opacity 0.1s ease-out',
        }}
      >
        <button
          onClick={scrollToLanding}
          className="max-w-md mx-auto w-full flex items-center justify-between px-5 py-3.5 rounded-2xl transition-all hover:opacity-80"
          style={{
            backgroundColor: 'var(--surfaceDefault, #1e293b)',
            border: '1px solid var(--borderDefault, #334155)',
          }}
        >
          <div className="flex items-center gap-3">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: 'var(--textMuted, #64748b)' }}>
              <path d="M9 18h6" />
              <path d="M10 22h4" />
              <path d="M12 2v2" />
              <path d="M12 8a4 4 0 0 0-4 4c0 2.2 1.8 4 4 4s4-1.8 4-4a4 4 0 0 0-4-4z" />
            </svg>
            <span className="text-sm" style={{ color: 'var(--textSecondary, #94a3b8)' }}>
              Explore developments
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs" style={{ color: 'var(--textMuted, #64748b)' }}>
              Scroll to explore
            </span>
            <div className="flex flex-col items-center">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor" style={{ color: 'var(--textMuted, #64748b)' }}>
                <path d="M12 4l-8 8h16z" />
              </svg>
              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor" style={{ color: 'var(--textMuted, #64748b)', marginTop: '-3px' }}>
                <path d="M12 4l-8 8h16z" />
              </svg>
              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor" style={{ color: 'var(--textMuted, #64748b)', marginTop: '-3px' }}>
                <path d="M12 4l-8 8h16z" />
              </svg>
            </div>
          </div>
        </button>
      </div>

      {/* ShareLandingPage — fades in on scroll, fades out when scrolled up */}
      <div
        className="fixed inset-0 z-30 overflow-y-auto"
        style={{
          opacity: landingOpacity,
          pointerEvents: landingOpacity > 0.5 ? 'auto' : 'none',
          transition: 'opacity 0.15s ease-out',
        }}
      >
        <div className="min-h-screen">
          <MarketingShareLandingPage />
        </div>
      </div>

      {/* Floating button — Back to top */}
      {showFloating && (
        <button
          onClick={scrollToTop}
          className="fixed bottom-24 right-6 z-50 flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium transition-all hover:scale-105"
          style={{
            backgroundColor: 'var(--accentPrimary, #00d4aa)',
            color: 'var(--textInverse, #0a0a0a)',
            boxShadow: '0 4px 20px rgba(0,212,170,0.3)',
          }}
        >
          <Database size={14} />
          Connect to Database
          <ArrowUp size={14} />
        </button>
      )}
    </div>
  );
}

function ShareApp() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/db/:code" element={<ShareLandingPage />} />
        <Route path="/" element={<HomePage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default ShareApp;
