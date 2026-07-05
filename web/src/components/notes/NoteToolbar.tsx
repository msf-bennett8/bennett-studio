import { useRef, useState } from 'react';
import { Download, Upload, Plus } from 'lucide-react';
import { useNotesStore } from '../../stores/notesStore';

export function NoteToolbar() {
  const { exportAllNotes, importNotes, addNote, getNoteStats } = useNotesStore();
  const [showImport, setShowImport] = useState(false);
  const [importText, setImportText] = useState('');
  const fileRef = useRef<HTMLInputElement>(null);

  const stats = getNoteStats();

  const handleImport = () => {
    if (importText.trim()) {
      importNotes(importText);
      setImportText('');
      setShowImport(false);
    }
  };

  const handleFileImport = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      if (text) importNotes(text);
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  return (
    <div className="px-3 py-2 border-b space-y-2" style={{ borderColor: 'var(--borderDefault)' }}>
      <div className="grid grid-cols-3 gap-1">
        <div className="text-center px-2 py-1.5 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <p className="text-xs font-bold" style={{ color: 'var(--accentPrimary)' }}>{stats.total}</p>
          <p className="text-[9px]" style={{ color: 'var(--textMuted)' }}>Notes</p>
        </div>
        <div className="text-center px-2 py-1.5 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <p className="text-xs font-bold" style={{ color: 'var(--accentWarning)' }}>{stats.pinned}</p>
          <p className="text-[9px]" style={{ color: 'var(--textMuted)' }}>Pinned</p>
        </div>
        <div className="text-center px-2 py-1.5 rounded-lg" style={{ backgroundColor: 'var(--bgTertiary)' }}>
          <p className="text-xs font-bold" style={{ color: 'var(--accentSecondary)' }}>{stats.words}</p>
          <p className="text-[9px]" style={{ color: 'var(--textMuted)' }}>Words</p>
        </div>
      </div>

      <div className="flex gap-1">
        <button
          onClick={() => addNote({ title: 'New Note', content: '' })}
          className="flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-lg text-[10px] font-medium transition-all"
          style={{
            backgroundColor: 'var(--accentPrimary)',
            color: 'var(--textInverse)',
          }}
        >
          <Plus size={12} />
          New
        </button>

        <button
          onClick={() => {
            const data = exportAllNotes();
            const blob = new Blob([data], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `bennett-notes-${new Date().toISOString().split('T')[0]}.json`;
            a.click();
            URL.revokeObjectURL(url);
          }}
          className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
          style={{ color: 'var(--textMuted)' }}
          title="Export"
        >
          <Download size={14} />
        </button>

        <button
          onClick={() => fileRef.current?.click()}
          className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
          style={{ color: 'var(--textMuted)' }}
          title="Import"
        >
          <Upload size={14} />
        </button>

        <input
          ref={fileRef}
          type="file"
          accept=".json,.md,.txt"
          onChange={handleFileImport}
          className="hidden"
        />
      </div>

      {showImport && (
        <div className="space-y-1">
          <textarea
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
            placeholder="Paste JSON..."
            className="w-full h-20 bg-transparent border rounded-lg p-2 text-[10px] outline-none resize-none"
            style={{
              borderColor: 'var(--borderDefault)',
              color: 'var(--textSecondary)',
            }}
          />
          <div className="flex gap-1">
            <button
              onClick={handleImport}
              className="flex-1 py-1 rounded text-[10px] font-medium"
              style={{ backgroundColor: 'var(--accentPrimary)', color: 'var(--textInverse)' }}
            >
              Import
            </button>
            <button
              onClick={() => setShowImport(false)}
              className="flex-1 py-1 rounded text-[10px] font-medium"
              style={{ border: '1px solid var(--borderDefault)', color: 'var(--textMuted)' }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {!showImport && (
        <button
          onClick={() => setShowImport(true)}
          className="w-full text-center text-[9px] py-1 transition-colors"
          style={{ color: 'var(--textMuted)' }}
        >
          JSON Import
        </button>
      )}
    </div>
  );
}
