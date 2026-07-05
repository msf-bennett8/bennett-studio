import { useState, useEffect, useRef, useCallback } from 'react';
import {
  Pin, Archive, Trash2, Copy, Download, Clock, Type, Hash, X,
  ChevronLeft, Palette, RotateCcw, AlertTriangle
} from 'lucide-react';
import { useNotesStore } from '../../stores/notesStore';
import { useDebounceCallback } from '../../hooks/useDebounce';

const COLORS = [
  { id: '#00d4aa', name: 'Terminal' },
  { id: '#6b8aff', name: 'Ocean' },
  { id: '#ffaa00', name: 'Amber' },
  { id: '#ff4444', name: 'Coral' },
  { id: '#a855f7', name: 'Purple' },
  { id: '#f472b6', name: 'Pink' },
  { id: '#22d3ee', name: 'Cyan' },
];

export function NoteEditor() {
  const {
    notes,
    selectedNoteId,
    updateNote,
    togglePin,
    archiveNote,
    deleteNote,
    setEditorOpen,
    duplicateNote,
  } = useNotesStore();

  const note = notes.find((n) => n.id === selectedNoteId);
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState('');
  const [showColorPicker, setShowColorPicker] = useState(false);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [displayStatus, setDisplayStatus] = useState<'saved' | 'saving'>('saved');
  const saveStatusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const displayTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (note) {
      setTitle(note.title);
      setContent(note.content);
      setTags(note.tags.join(', '));
    }
  }, [note?.id]);

  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
      el.style.height = el.scrollHeight + 'px';
    }
  }, [content]);

  // Default: show "Saved"
  // After 8 seconds of continuous typing: show "Saving..." briefly
  const debouncedSave = useDebounceCallback(() => {
    if (!note) return;

    updateNote(note.id, {
      title: title.trim() || 'Note',
      content,
      tags: tags.split(',').map((t) => t.trim()).filter(Boolean),
    });

    if (saveStatusTimer.current) clearTimeout(saveStatusTimer.current);
    if (displayTimer.current) clearTimeout(displayTimer.current);

    displayTimer.current = setTimeout(() => {
      setDisplayStatus('saving');
      saveStatusTimer.current = setTimeout(() => {
        setDisplayStatus('saved');
      }, 1500);
    }, 8000);
  }, 500);

  useEffect(() => {
    debouncedSave();
  }, [title, content, tags, debouncedSave]);

  useEffect(() => {
    return () => {
      if (saveStatusTimer.current) clearTimeout(saveStatusTimer.current);
      if (displayTimer.current) clearTimeout(displayTimer.current);
    };
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      debouncedSave();
    }
  }, [debouncedSave]);

  if (!note) return null;

  const handleCopyContent = () => {
    navigator.clipboard.writeText(content);
  };

  const handleExport = () => {
    const blob = new Blob([`# ${title}\n\n${content}`], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${title.replace(/\s+/g, '_')}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleDeleteConfirm = () => {
    deleteNote(note.id);
    setShowDeleteModal(false);
  };

  const timeAgo = (date: string) => {
    const diff = Date.now() - new Date(date).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h`;
    return `${Math.floor(hrs / 24)}d`;
  };

  return (
    <div className="h-full flex flex-col" style={{ backgroundColor: 'var(--bgPrimary)' }}>
      {/* Delete Confirmation Modal */}
      {showDeleteModal && (
        <div className="fixed inset-0 z-[200] flex items-center justify-center">
          <div
            className="absolute inset-0"
            style={{ backgroundColor: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(8px)' }}
            onClick={() => setShowDeleteModal(false)}
          />
          <div
            className="relative w-full max-w-md mx-4 rounded-2xl overflow-hidden"
            style={{
              backgroundColor: 'var(--bgElevated)',
              border: '1px solid var(--borderDefault)',
              boxShadow: '0 24px 48px rgba(0,0,0,0.5)',
            }}
          >
            <div className="flex flex-col items-center pt-8 pb-4">
              <div
                className="w-16 h-16 rounded-full flex items-center justify-center mb-4"
                style={{
                  backgroundColor: 'rgba(255, 68, 68, 0.15)',
                  border: '1px solid rgba(255, 68, 68, 0.2)',
                }}
              >
                <AlertTriangle size={28} style={{ color: 'var(--accentError)' }} />
              </div>
              <h3 className="text-lg font-bold text-center" style={{ color: 'var(--textPrimary)' }}>
                Delete Note
              </h3>
            </div>

            <div className="px-6 pb-2">
              <div
                className="rounded-xl p-4 mb-4"
                style={{
                  backgroundColor: 'var(--bgTertiary)',
                  border: '1px solid var(--borderDefault)',
                }}
              >
                <p className="text-xs text-center" style={{ color: 'var(--textMuted)' }}>
                  "<span style={{ color: 'var(--textPrimary)' }}>{note.title}</span>"
                </p>
                <p className="text-xs text-center mt-2" style={{ color: 'var(--textMuted)' }}>
                  Are you sure? This cannot be undone.
                </p>
              </div>
            </div>

            <div className="px-6 pb-6 flex gap-3">
              <button
                onClick={() => setShowDeleteModal(false)}
                className="flex-1 py-3 rounded-xl text-xs font-medium transition-all"
                style={{
                  backgroundColor: 'var(--surfaceDefault)',
                  border: '1px solid var(--borderDefault)',
                  color: 'var(--textSecondary)',
                }}
              >
                Cancel
              </button>
              <button
                onClick={handleDeleteConfirm}
                className="flex-1 py-3 rounded-xl text-xs font-bold transition-all"
                style={{
                  backgroundColor: 'rgba(255, 68, 68, 0.2)',
                  border: '1px solid rgba(255, 68, 68, 0.3)',
                  color: 'var(--accentError)',
                }}
              >
                Yes, Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Toolbar */}
      <div
        className="px-4 py-2 border-b flex items-center justify-between shrink-0"
        style={{ borderColor: 'var(--borderDefault)' }}
      >
        <div className="flex items-center gap-1">
          <button
            onClick={() => setEditorOpen(false)}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover lg:hidden"
            style={{ color: 'var(--textMuted)' }}
          >
            <ChevronLeft size={16} />
          </button>

          <button
            onClick={() => togglePin(note.id)}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: note.pinned ? 'var(--accentWarning)' : 'var(--textMuted)' }}
            title={note.pinned ? 'Pinned' : 'Pin'}
          >
            <Pin size={16} className={note.pinned ? 'fill-current' : ''} />
          </button>

          <button
            onClick={() => archiveNote(note.id)}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
            title="Archive"
          >
            <Archive size={16} />
          </button>

          <div className="relative">
            <button
              onClick={() => setShowColorPicker(!showColorPicker)}
              className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
              style={{ color: note.color }}
              title="Color"
            >
              <Palette size={16} />
            </button>
            {showColorPicker && (
              <div
                className="absolute top-full left-0 mt-1 p-2 rounded-lg border z-50 flex gap-1"
                style={{
                  backgroundColor: 'var(--bgElevated)',
                  borderColor: 'var(--borderDefault)',
                }}
              >
                {COLORS.map((c) => (
                  <button
                    key={c.id}
                    onClick={() => {
                      updateNote(note.id, { color: c.id });
                      setShowColorPicker(false);
                    }}
                    className="w-6 h-6 rounded-full transition-transform hover:scale-110"
                    style={{
                      backgroundColor: c.id,
                      border: note.color === c.id ? '2px solid var(--textPrimary)' : '2px solid transparent',
                    }}
                    title={c.name}
                  />
                ))}
              </div>
            )}
          </div>

          <div className="w-px h-4 mx-1" style={{ backgroundColor: 'var(--borderDefault)' }} />

          <button
            onClick={handleCopyContent}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
            title="Copy"
          >
            <Copy size={16} />
          </button>

          <button
            onClick={handleExport}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
            title="Download"
          >
            <Download size={16} />
          </button>

          <button
            onClick={() => duplicateNote(note.id)}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
            title="Duplicate"
          >
            <RotateCcw size={16} />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-[10px] font-mono flex items-center gap-1.5" style={{ 
            color: displayStatus === 'saving' ? 'var(--accentWarning)' : 'var(--accentPrimary)',
          }}>
            <span 
              className="w-1.5 h-1.5 rounded-full"
              style={{ 
                backgroundColor: displayStatus === 'saving' ? 'var(--accentWarning)' : 'var(--accentPrimary)',
                animation: displayStatus === 'saved' ? 'blink-dot 3s steps(1) infinite' : 'none',
              }}
            />
            {displayStatus === 'saving' ? 'Saving...' : 'Saved'}
          </span>

          <button
            onClick={() => setShowDeleteModal(true)}
            className="p-1.5 rounded-lg transition-colors hover:bg-accent-error/10"
            style={{ color: 'var(--accentError)' }}
            title="Delete"
          >
            <Trash2 size={16} />
          </button>

          <button
            onClick={() => setEditorOpen(false)}
            className="p-1.5 rounded-lg transition-colors hover:bg-surface-hover hidden lg:block"
            style={{ color: 'var(--textMuted)' }}
          >
            <X size={16} />
          </button>
        </div>
      </div>

      {/* Editor */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl px-6 py-6">
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Title..."
            className="w-full bg-transparent text-2xl font-bold outline-none mb-4"
            style={{ color: 'var(--textPrimary)' }}
          />

          <div className="flex items-center gap-4 mb-4 text-[10px] font-mono" style={{ color: 'var(--textMuted)' }}>
            <span className="flex items-center gap-1">
              <Clock size={12} />
              {timeAgo(note.updatedAt)}
            </span>
            <span className="flex items-center gap-1">
              <Type size={12} />
              {note.wordCount} words
            </span>
            <span>{note.charCount} chars</span>
          </div>

          <div className="flex items-center gap-2 mb-4">
            <Hash size={12} style={{ color: 'var(--textMuted)' }} />
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="tag1, tag2..."
              className="flex-1 bg-transparent text-xs outline-none"
              style={{ color: 'var(--textSecondary)' }}
            />
          </div>

          <div
            className="h-0.5 w-full rounded-full mb-4"
            style={{ backgroundColor: note.color, opacity: 0.5 }}
          />

          <textarea
            ref={textareaRef}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Start scribbling here...

# Markdown support
- Lists
- **Bold**
- *Italic*
- `code`"
            className="w-full min-h-[60vh] bg-transparent font-mono text-sm leading-relaxed outline-none resize-none"
            style={{ color: 'var(--textSecondary)' }}
            spellCheck={false}
          />
        </div>
      </div>
    </div>
  );
}
