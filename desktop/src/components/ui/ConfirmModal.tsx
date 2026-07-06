import { AlertTriangle, Trash2, ShieldAlert } from 'lucide-react';

interface ConfirmModalProps {
  open: boolean;
  type: 'revoke' | 'delete';
  title: string;
  message: string;
  code: string;
  confirmText: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmModal({ open, type, title, message, code, confirmText, onConfirm, onCancel }: ConfirmModalProps) {
  if (!open) return null;

  const isDelete = type === 'delete';
  const accentColor = isDelete ? 'var(--accentError)' : 'var(--accentWarning)';
  const bgColor = isDelete ? 'rgba(255,68,68,0.1)' : 'rgba(255,170,0,0.1)';
  const Icon = isDelete ? Trash2 : ShieldAlert;

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50 animate-in fade-in duration-200" style={{ backgroundColor: 'var(--bgOverlay)' }}>
      <div 
        className="w-full max-w-md mx-4 p-6 rounded-2xl shadow-2xl transform transition-all"
        style={{ 
          backgroundColor: 'var(--bgElevated)', 
          border: '1px solid var(--borderDefault)',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)'
        }}
      >
        {/* Header with icon */}
        <div className="flex items-center gap-3 mb-4">
          <div 
            className="w-12 h-12 rounded-xl flex items-center justify-center"
            style={{ backgroundColor: bgColor }}
          >
            <Icon size={24} style={{ color: accentColor }} />
          </div>
          <div>
            <h2 className="text-lg font-bold" style={{ color: 'var(--textPrimary)' }}>{title}</h2>
            <p className="text-xs font-mono mt-0.5 px-2 py-0.5 rounded" style={{ backgroundColor: 'var(--bgTertiary)', color: 'var(--textMuted)' }}>
              {code}
            </p>
          </div>
        </div>

        {/* Message */}
        <div className="mb-6 p-4 rounded-xl" style={{ backgroundColor: 'var(--bgSecondary)' }}>
          <p className="text-sm leading-relaxed" style={{ color: 'var(--textSecondary)' }}>
            {message}
          </p>
        </div>

        {/* Warning for delete */}
        {isDelete && (
          <div className="flex items-center gap-2 mb-6 p-3 rounded-lg" style={{ backgroundColor: 'rgba(255,68,68,0.05)', border: '1px solid rgba(255,68,68,0.2)' }}>
            <AlertTriangle size={14} style={{ color: 'var(--accentError)' }} />
            <span className="text-xs" style={{ color: 'var(--accentError)' }}>
              This action is permanent and cannot be undone.
            </span>
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3">
          <button
            onClick={onCancel}
            className="flex-1 py-2.5 rounded-xl text-sm font-medium transition-all hover:opacity-80"
            style={{ 
              backgroundColor: 'var(--bgTertiary)', 
              color: 'var(--textSecondary)',
              border: '1px solid var(--borderDefault)'
            }}
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="flex-1 py-2.5 rounded-xl text-sm font-medium text-white transition-all hover:opacity-90"
            style={{ backgroundColor: accentColor }}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
