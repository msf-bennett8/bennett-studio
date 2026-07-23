import { BookOpen, ExternalLink, FileText, Cookie, ShieldCheck } from 'lucide-react';

const links = [
  { label: 'Bennett Studio Website', href: 'https://bennett-studio.vercel.app', external: true, available: true },
  { label: 'Documentation', href: '#', external: false, available: false },
  { label: 'Terms of Service', href: '#', external: false, available: false },
  { label: 'Privacy Policy', href: '#', external: false, available: false },
  { label: 'Cookie Policy', href: '#', external: false, available: false },
];

const icons = [BookOpen, FileText, ShieldCheck, ShieldCheck, Cookie];

export function ResourcesSettings() {
  return (
    <div className="card p-6 rounded-xl" style={{ backgroundColor: 'var(--surfaceDefault)', border: '1px solid var(--borderDefault)' }}>
      <div className="flex items-center gap-3 mb-4">
        <BookOpen size={20} style={{ color: 'var(--accentPrimary)' }} />
        <h2 className="text-lg font-semibold" style={{ color: 'var(--textPrimary)' }}>Resources</h2>
      </div>
      <div className="space-y-2">
        {links.map((link, i) => {
          const Icon = icons[i];
                   return (
            <a
              key={link.label}
              href={link.href}
              target={link.external ? '_blank' : undefined}
              rel={link.external ? 'noopener noreferrer' : undefined}
              className="flex items-center justify-between p-3 rounded-lg transition-all"
              style={{ backgroundColor: 'var(--bgSecondary)' }}
            >
              <div className="flex items-center gap-3">
                <Icon size={16} style={{ color: 'var(--textMuted)' }} />
                <span className="text-sm font-medium" style={{ color: 'var(--textPrimary)' }}>{link.label}</span>
                {!link.available && (
                  <span className="text-xs px-2 py-0.5 rounded-full" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>Coming soon</span>
                )}
              </div>
              <ExternalLink size={14} style={{ color: 'var(--textMuted)' }} />
            </a>
          );
        })}
      </div>
    </div>
  );
}
