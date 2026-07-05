import { useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { StickyNote, Plus, ArrowLeft } from 'lucide-react';
import { useNotesStore } from '../stores/notesStore';
import { NoteEditor } from '../components/notes/NoteEditor';
import { NoteList } from '../components/notes/NoteList';
import { NoteToolbar } from '../components/notes/NoteToolbar';

export function NotesPage() {
  const navigate = useNavigate();
  const { notes, selectedNoteId, isEditorOpen, setEditorOpen, selectNote, addNote } = useNotesStore();

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) {
      if (e.key === 'n') {
        e.preventDefault();
        addNote({ title: 'New Note', content: '' });
      }
      if (e.key === 'e' && selectedNoteId) {
        e.preventDefault();
        setEditorOpen(true);
      }
    }
    if (e.key === 'Escape' && isEditorOpen) {
      setEditorOpen(false);
    }
  }, [addNote, selectedNoteId, isEditorOpen, setEditorOpen]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  const handleCreateNote = () => {
    addNote({ title: 'New Note', content: '' });
  };

  return (
    <div className="h-full flex flex-col" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      {/* Header */}
      <div
        className="sticky top-0 z-30 px-6 py-4 border-b flex items-center justify-between shrink-0"
        style={{
          backgroundColor: 'var(--bgSecondary)',
          borderColor: 'var(--borderDefault)',
        }}
      >
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/')}
            className="p-2 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
          >
            <ArrowLeft size={18} />
          </button>
          <div className="flex items-center gap-3">
            <StickyNote size={20} style={{ color: 'var(--accentPrimary)' }} />
            <div>
              <h1 className="text-lg font-bold" style={{ color: 'var(--textPrimary)' }}>
                Notes
              </h1>
              <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
                {notes.filter(n => !n.archived).length} notes • {notes.reduce((s, n) => s + n.wordCount, 0)} words
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => useNotesStore.getState().syncToEngine()}
            className="flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition-all border"
            style={{
              borderColor: 'var(--borderDefault)',
              color: 'var(--textMuted)',
            }}
            title="Sync with engine"
          >
            <span 
              className="w-1.5 h-1.5 rounded-full"
              style={{ 
                backgroundColor: 'var(--accentPrimary)',
                animation: 'blink-dot 3s steps(1) infinite',
              }}
            />
            Sync
          </button>
          <button
            onClick={handleCreateNote}
            className="flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all hover:opacity-90"
            style={{
              backgroundColor: 'var(--accentPrimary)',
              color: 'var(--textInverse)',
            }}
          >
            <Plus size={16} />
            <span>New Note</span>
          </button>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar List */}
        <div
          className="w-80 border-r flex flex-col shrink-0 overflow-hidden"
          style={{
            borderColor: 'var(--borderDefault)',
            backgroundColor: 'var(--bgSecondary)',
          }}
        >
          <NoteToolbar />
          <div className="flex-1 overflow-y-auto">
            <NoteList />
          </div>
        </div>

        {/* Editor Area */}
        <div className="flex-1 overflow-hidden relative">
          {isEditorOpen && selectedNoteId ? (
            <NoteEditor />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <StickyNote size={48} style={{ color: 'var(--textMuted)', opacity: 0.3 }} />
              <p className="text-sm" style={{ color: 'var(--textMuted)' }}>
                Select a note or create a new one
              </p>
              <button
                onClick={handleCreateNote}
                className="mt-2 flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-all border"
                style={{
                  borderColor: 'var(--borderDefault)',
                  color: 'var(--textSecondary)',
                }}
              >
                <Plus size={16} />
                Create Note
              </button>
              <p className="text-xs mt-4" style={{ color: 'var(--textMuted)', opacity: 0.4 }}>
                Ctrl+N — new note • Ctrl+S — save
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
