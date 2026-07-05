import { useState, useRef, useCallback } from 'react';
import { Pin, Archive, Clock, Hash, Search, StickyNote, X, Check, Trash2, Download } from 'lucide-react';
import { useNotesStore } from '../../stores/notesStore';

export function NoteList() {
  const {
    getFilteredNotes,
    selectedNoteId,
    selectNote,
    setEditorOpen,
    searchQuery,
    setSearchQuery,
    activeFilter,
    setActiveFilter,
    getAllTags,
    activeTag,
    setActiveTag,
    getNoteStats,
    selectedNoteIds,
    isSelectionMode,
    toggleSelection,
    selectAll,
    deselectAll,
    setSelectionMode,
    deleteSelected,
    archiveSelected,
    pinSelected,
    exportSelected,
  } = useNotesStore();

  const notes = getFilteredNotes();
  const tags = getAllTags();
  const stats = getNoteStats();
  const selectedCount = selectedNoteIds.length;

  const [longPressTimer, setLongPressTimer] = useState<ReturnType<typeof setTimeout> | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; noteId: string } | null>(null);

  const filters = [
    { id: 'all' as const, label: 'All', count: stats.total },
    { id: 'pinned' as const, label: 'Pinned', count: stats.pinned },
    { id: 'recent' as const, label: 'Recent', count: stats.total },
    { id: 'archived' as const, label: 'Archived', count: stats.archived },
  ];

  const timeAgo = (date: string) => {
    const diff = Date.now() - new Date(date).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'now';
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h`;
    return `${Math.floor(hrs / 24)}d`;
  };

  const handleTouchStart = (noteId: string) => {
    const timer = setTimeout(() => {
      setSelectionMode(true);
      toggleSelection(noteId);
    }, 600);
    setLongPressTimer(timer);
  };

  const handleTouchEnd = () => {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      setLongPressTimer(null);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, noteId: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, noteId });
  };

  const handleNoteClick = (noteId: string) => {
    if (isSelectionMode) {
      toggleSelection(noteId);
    } else {
      selectNote(noteId);
      setEditorOpen(true);
    }
  };

  const isSelected = (id: string) => selectedNoteIds.includes(id);

  return (
    <div className="flex flex-col h-full relative">
      {/* Context Menu */}
      {contextMenu && (
        <>
          <div
            className="fixed inset-0 z-[150]"
            onClick={() => setContextMenu(null)}
          />
          <div
            className="fixed z-[160] py-1 rounded-xl overflow-hidden"
            style={{
              left: Math.min(contextMenu.x, window.innerWidth - 200),
              top: Math.min(contextMenu.y, window.innerHeight - 150),
              backgroundColor: 'var(--bgElevated)',
              border: '1px solid var(--borderDefault)',
              boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
              minWidth: '180px',
            }}
          >
            <button
              onClick={() => {
                setSelectionMode(true);
                toggleSelection(contextMenu.noteId);
                setContextMenu(null);
              }}
              className="w-full px-4 py-2 text-left text-xs flex items-center gap-2 transition-colors hover:bg-surface-hover"
              style={{ color: 'var(--textPrimary)' }}
            >
              <Check size={14} /> Select
            </button>
            <button
              onClick={() => {
                selectNote(contextMenu.noteId);
                setEditorOpen(true);
                setContextMenu(null);
              }}
              className="w-full px-4 py-2 text-left text-xs flex items-center gap-2 transition-colors hover:bg-surface-hover"
              style={{ color: 'var(--textSecondary)' }}
            >
              <StickyNote size={14} /> Open
            </button>
            <div className="h-px mx-3 my-1" style={{ backgroundColor: 'var(--borderDefault)' }} />
            <button
              onClick={() => {
                toggleSelection(contextMenu.noteId);
                setContextMenu(null);
              }}
              className="w-full px-4 py-2 text-left text-xs flex items-center gap-2 transition-colors hover:bg-surface-hover"
              style={{ color: 'var(--accentError)' }}
            >
              <Trash2 size={14} /> Delete
            </button>
          </div>
        </>
      )}

      {/* Search */}
      <div className="px-3 py-2 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg border" style={{ borderColor: 'var(--borderDefault)', backgroundColor: 'var(--bgTertiary)' }}>
          <Search size={14} style={{ color: 'var(--textMuted)' }} />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search..."
            className="flex-1 bg-transparent text-xs outline-none"
            style={{ color: 'var(--textPrimary)' }}
          />
          {searchQuery && (
            <button onClick={() => setSearchQuery('')} style={{ color: 'var(--textMuted)' }}>
              <X size={12} />
            </button>
          )}
        </div>
      </div>

      {/* Selection Mode Bar */}
      {isSelectionMode && (
        <div
          className="px-3 py-2 border-b flex items-center justify-between"
          style={{
            borderColor: 'var(--borderDefault)',
            backgroundColor: 'rgba(0, 212, 170, 0.05)',
          }}
        >
          <div className="flex items-center gap-2">
            <button
              onClick={selectAll}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-[10px] font-medium transition-all"
              style={{
                backgroundColor: 'var(--accentPrimary)',
                color: 'var(--textInverse)',
              }}
            >
              <Check size={12} />
              {selectedCount === notes.filter(n => !n.archived).length ? 'Deselect All' : 'Select All'}
            </button>
            <span className="text-[10px] font-mono" style={{ color: 'var(--textMuted)' }}>
              {selectedCount} selected
            </span>
          </div>
          <button
            onClick={deselectAll}
            className="p-1 rounded-lg transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
          >
            <X size={14} />
          </button>
        </div>
      )}

      {/* Bulk Actions Bar */}
      {isSelectionMode && selectedCount > 0 && (
        <div
          className="px-3 py-2 border-b flex items-center gap-1"
          style={{ borderColor: 'var(--borderDefault)' }}
        >
          <button
            onClick={pinSelected}
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--accentWarning)' }}
          >
            <Pin size={12} /> Pin
          </button>
          <button
            onClick={archiveSelected}
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--textMuted)' }}
          >
            <Archive size={12} /> Archive
          </button>
          <button
            onClick={() => {
              const data = exportSelected();
              const blob = new Blob([data], { type: 'application/json' });
              const url = URL.createObjectURL(blob);
              const a = document.createElement('a');
              a.href = url;
              a.download = `bennett-notes-selected-${new Date().toISOString().split('T')[0]}.json`;
              a.click();
              URL.revokeObjectURL(url);
            }}
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium transition-colors hover:bg-surface-hover"
            style={{ color: 'var(--accentSecondary)' }}
          >
            <Download size={12} /> Export
          </button>
          <button
            onClick={deleteSelected}
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium transition-colors hover:bg-accent-error/10"
            style={{ color: 'var(--accentError)' }}
          >
            <Trash2 size={12} /> Delete
          </button>
        </div>
      )}

      {/* Filters */}
      <div className="flex gap-1 px-3 py-2 border-b overflow-x-auto" style={{ borderColor: 'var(--borderDefault)' }}>
        {filters.map((f) => (
          <button
            key={f.id}
            onClick={() => setActiveFilter(f.id)}
            className="px-2.5 py-1 rounded-md text-[10px] font-medium transition-all whitespace-nowrap"
            style={{
              backgroundColor: activeFilter === f.id ? 'var(--accentPrimary)' + '20' : 'transparent',
              color: activeFilter === f.id ? 'var(--accentPrimary)' : 'var(--textMuted)',
              border: activeFilter === f.id ? '1px solid var(--accentPrimary)40' : '1px solid transparent',
            }}
          >
            {f.label} <span style={{ opacity: 0.6 }}>{f.count}</span>
          </button>
        ))}
      </div>

      {/* Tags */}
      {tags.length > 0 && (
        <div className="flex flex-wrap gap-1 px-3 py-2 border-b" style={{ borderColor: 'var(--borderDefault)' }}>
          {tags.map((tag) => (
            <button
              key={tag}
              onClick={() => setActiveTag(activeTag === tag ? null : tag)}
              className="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium transition-all"
              style={{
                backgroundColor: activeTag === tag ? 'var(--accentSecondary)' + '20' : 'var(--bgTertiary)',
                color: activeTag === tag ? 'var(--accentSecondary)' : 'var(--textMuted)',
                border: activeTag === tag ? '1px solid var(--accentSecondary)40' : '1px solid var(--borderDefault)',
              }}
            >
              <Hash size={9} />
              {tag}
            </button>
          ))}
        </div>
      )}

      {/* Notes */}
      <div className="flex-1 overflow-y-auto">
        {notes.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 gap-3">
            <StickyNote size={32} style={{ color: 'var(--textMuted)', opacity: 0.3 }} />
            <p className="text-xs" style={{ color: 'var(--textMuted)' }}>
              {searchQuery ? 'No results' : 'No notes'}
            </p>
          </div>
        ) : (
          <div className="py-1">
            {notes.map((note) => {
              const selected = isSelected(note.id);
              return (
                <button
                  key={note.id}
                  onClick={() => handleNoteClick(note.id)}
                  onContextMenu={(e) => handleContextMenu(e, note.id)}
                  onMouseDown={() => handleTouchStart(note.id)}
                  onMouseUp={handleTouchEnd}
                  onMouseLeave={handleTouchEnd}
                  onTouchStart={() => handleTouchStart(note.id)}
                  onTouchEnd={handleTouchEnd}
                  className="w-full text-left px-3 py-2.5 transition-all border-b group flex items-center gap-3"
                  style={{
                    borderColor: 'var(--borderDefault)',
                    backgroundColor: selected
                      ? 'rgba(0, 212, 170, 0.08)'
                      : selectedNoteId === note.id
                      ? 'var(--accentPrimary)' + '08'
                      : 'transparent',
                    borderLeft: selectedNoteId === note.id && !selected
                      ? `2px solid ${note.color}`
                      : selected
                      ? `2px solid var(--accentPrimary)`
                      : '2px solid transparent',
                  }}
                >
                  <div className="flex items-center gap-2 shrink-0">
                    <div
                      className="w-2 h-2 rounded-full"
                      style={{ backgroundColor: note.color }}
                    />
                    {note.pinned && (
                      <Pin size={10} style={{ color: 'var(--accentWarning)' }} />
                    )}
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs font-medium truncate" style={{ color: 'var(--textPrimary)' }}>
                        {note.title}
                      </span>
                    </div>
                    <p className="text-[10px] truncate mt-0.5" style={{ color: 'var(--textMuted)' }}>
                      {note.content.slice(0, 60) || 'Empty note'}
                    </p>
                    <div className="flex items-center gap-2 mt-1.5">
                      <span className="flex items-center gap-1 text-[9px]" style={{ color: 'var(--textMuted)' }}>
                        <Clock size={9} />
                        {timeAgo(note.updatedAt)}
                      </span>
                      <span className="text-[9px]" style={{ color: 'var(--textMuted)' }}>
                        {note.wordCount} words
                      </span>
                      {note.tags.length > 0 && (
                        <span className="flex items-center gap-0.5 text-[9px]" style={{ color: 'var(--accentSecondary)' }}>
                          <Hash size={9} />
                          {note.tags.length}
                        </span>
                      )}
                    </div>
                  </div>

                  <div
                    className="w-5 h-5 rounded-full border-2 flex items-center justify-center shrink-0 transition-all"
                    style={{
                      borderColor: selected ? 'var(--accentPrimary)' : 'rgba(255,255,255,0.15)',
                      backgroundColor: selected ? 'var(--accentPrimary)' : 'transparent',
                    }}
                  >
                    {selected && (
                      <Check size={12} style={{ color: 'var(--bgPrimary)' }} />
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
