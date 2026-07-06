import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { api } from '../services/api';

export interface Note {
  id: string;
  title: string;
  content: string;
  tags: string[];
  pinned: boolean;
  archived: boolean;
  color: string;
  createdAt: string;
  updatedAt: string;
  wordCount: number;
  charCount: number;
}

export type NoteFilter = 'all' | 'pinned' | 'recent' | 'archived' | 'tags';

interface NotesState {
  notes: Note[];
  searchQuery: string;
  activeFilter: NoteFilter;
  activeTag: string | null;
  selectedNoteId: string | null;
  isEditorOpen: boolean;
  selectedNoteIds: string[];
  isSelectionMode: boolean;

  addNote: (note: Partial<Note>) => Note;
  updateNote: (id: string, updates: Partial<Note>) => void;
  deleteNote: (id: string) => void;
  deleteAllNotes: () => void;
  archiveNote: (id: string) => void;
  restoreNote: (id: string) => void;
  togglePin: (id: string) => void;
  duplicateNote: (id: string) => void;
  setSearchQuery: (query: string) => void;
  setActiveFilter: (filter: NoteFilter) => void;
  setActiveTag: (tag: string | null) => void;
  selectNote: (id: string | null) => void;
  setEditorOpen: (open: boolean) => void;
  toggleSelection: (id: string) => void;
  selectAll: () => void;
  deselectAll: () => void;
  setSelectionMode: (mode: boolean) => void;
  deleteSelected: () => void;
  archiveSelected: () => void;
  pinSelected: () => void;
  exportNote: (id: string) => string;
  exportAllNotes: () => string;
  exportSelected: () => string;
  importNotes: (json: string) => void;
  getFilteredNotes: () => Note[];
  getAllTags: () => string[];
  getNoteStats: () => { total: number; pinned: number; archived: number; words: number };
  getSelectedCount: () => number;
  syncToEngine: () => Promise<void>;
  loadFromEngine: () => Promise<void>;
}

const COLORS = ['#00d4aa', '#6b8aff', '#ffaa00', '#ff4444', '#a855f7', '#f472b6', '#22d3ee'];

const generateId = () => crypto.randomUUID();

const countWords = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

export const useNotesStore = create<NotesState>()(
  persist(
    (set, get) => ({
      notes: [],
      searchQuery: '',
      activeFilter: 'all',
      activeTag: null,
      selectedNoteId: null,
      isEditorOpen: false,
      selectedNoteIds: [],
      isSelectionMode: false,

      addNote: (noteData) => {
        const now = new Date().toISOString();
        const content = noteData.content || '';
        const newNote: Note = {
          id: generateId(),
          title: noteData.title || 'New Note',
          content,
          tags: noteData.tags || [],
          pinned: false,
          archived: false,
          color: noteData.color || COLORS[Math.floor(Math.random() * COLORS.length)],
          createdAt: now,
          updatedAt: now,
          wordCount: countWords(content),
          charCount: content.length,
        };

        set((state) => ({
          notes: [newNote, ...state.notes],
          selectedNoteId: newNote.id,
          isEditorOpen: true,
          selectedNoteIds: [],
          isSelectionMode: false,
        }));

        return newNote;
      },

      updateNote: (id, updates) => {
        set((state) => ({
          notes: state.notes.map((note) => {
            if (note.id !== id) return note;
            const content = updates.content !== undefined ? updates.content : note.content;
            return {
              ...note,
              ...updates,
              content,
              wordCount: countWords(content),
              charCount: content.length,
              updatedAt: new Date().toISOString(),
            };
          }),
        }));
      },

      deleteNote: (id) => {
        set((state) => ({
          notes: state.notes.filter((n) => n.id !== id),
          selectedNoteId: state.selectedNoteId === id ? null : state.selectedNoteId,
          selectedNoteIds: state.selectedNoteIds.filter((sid) => sid !== id),
        }));
      },

      deleteAllNotes: () => {
        set({ notes: [], selectedNoteId: null, isEditorOpen: false, selectedNoteIds: [], isSelectionMode: false });
        // Also clear from engine on next sync
      },

      // Hybrid sync: push localStorage notes to engine when online
      syncToEngine: async () => {
        const { notes } = get();
        const deviceId = navigator.userAgent + '_' + Date.now();
        try {
          const synced = await api.syncNotes(notes, deviceId);
          // Merge: engine wins on conflict (newer updated_at)
          const localIds = new Set(notes.map(n => n.id));
          const newFromEngine = synced.filter(n => !localIds.has(n.id));
          
          set((state) => ({
            notes: [...newFromEngine, ...state.notes].sort((a, b) => {
              if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
              return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
            }),
            lastSynced: new Date().toISOString(),
          }));
        } catch (e) {
          console.warn('Engine sync failed, keeping localStorage:', e);
        }
      },

      // Load from engine on startup
      loadFromEngine: async () => {
        try {
          const engineNotes = await api.listNotes();
          if (engineNotes.length > 0) {
            set((state) => {
              const localIds = new Set(state.notes.map(n => n.id));
              const merged = [
                ...engineNotes.map((n: any) => ({ ...n, createdAt: n.created_at || n.createdAt, updatedAt: n.updated_at || n.updatedAt })),
                ...state.notes.filter(n => !localIds.has(n.id)),
              ];
              return { notes: merged };
            });
          }
        } catch (e) {
          console.warn('Engine load failed, using localStorage:', e);
        }
      },

      archiveNote: (id) => {
        set((state) => ({
          notes: state.notes.map((n) => (n.id === id ? { ...n, archived: true, pinned: false } : n)),
        }));
      },

      restoreNote: (id) => {
        set((state) => ({
          notes: state.notes.map((n) => (n.id === id ? { ...n, archived: false } : n)),
        }));
      },

      togglePin: (id) => {
        set((state) => ({
          notes: state.notes.map((n) => (n.id === id ? { ...n, pinned: !n.pinned } : n)),
        }));
      },

      duplicateNote: (id) => {
        const original = get().notes.find((n) => n.id === id);
        if (!original) return;
        const now = new Date().toISOString();
        const copy: Note = {
          ...original,
          id: generateId(),
          title: `${original.title} (Copy)`,
          pinned: false,
          archived: false,
          createdAt: now,
          updatedAt: now,
        };
        set((state) => ({
          notes: [copy, ...state.notes],
          selectedNoteId: copy.id,
          isEditorOpen: true,
          selectedNoteIds: [],
          isSelectionMode: false,
        }));
      },

      setSearchQuery: (query) => set({ searchQuery: query }),
      setActiveFilter: (filter) => set({ activeFilter: filter, activeTag: null }),
      setActiveTag: (tag) => set({ activeTag: tag, activeFilter: tag ? 'tags' : 'all' }),
      selectNote: (id) => set({ selectedNoteId: id, isEditorOpen: id !== null, selectedNoteIds: [], isSelectionMode: false }),
      setEditorOpen: (open) => set({ isEditorOpen: open }),

      toggleSelection: (id) => {
        set((state) => {
          const ids = state.selectedNoteIds.includes(id)
            ? state.selectedNoteIds.filter((sid) => sid !== id)
            : [...state.selectedNoteIds, id];
          return {
            selectedNoteIds: ids,
            isSelectionMode: ids.length > 0,
          };
        });
      },

      selectAll: () => {
        set((state) => {
          const visibleIds = state.notes
            .filter((n) => !n.archived)
            .map((n) => n.id);
          const allSelected = visibleIds.length === state.selectedNoteIds.length &&
            visibleIds.every((id) => state.selectedNoteIds.includes(id));
          return {
            selectedNoteIds: allSelected ? [] : visibleIds,
            isSelectionMode: !allSelected,
          };
        });
      },

      deselectAll: () => set({ selectedNoteIds: [], isSelectionMode: false }),

      setSelectionMode: (mode) => {
        if (!mode) set({ selectedNoteIds: [], isSelectionMode: false });
        else set({ isSelectionMode: true });
      },

      deleteSelected: () => {
        set((state) => ({
          notes: state.notes.filter((n) => !state.selectedNoteIds.includes(n.id)),
          selectedNoteIds: [],
          selectedNoteId: state.selectedNoteIds.includes(state.selectedNoteId || '') ? null : state.selectedNoteId,
          isSelectionMode: false,
        }));
      },

      archiveSelected: () => {
        set((state) => ({
          notes: state.notes.map((n) =>
            state.selectedNoteIds.includes(n.id) ? { ...n, archived: true, pinned: false } : n
          ),
          selectedNoteIds: [],
          isSelectionMode: false,
        }));
      },

      pinSelected: () => {
        set((state) => ({
          notes: state.notes.map((n) =>
            state.selectedNoteIds.includes(n.id) && !n.archived
              ? { ...n, pinned: !n.pinned }
              : n
          ),
          selectedNoteIds: [],
          isSelectionMode: false,
        }));
      },

      exportNote: (id) => {
        const note = get().notes.find((n) => n.id === id);
        if (!note) return '';
        return JSON.stringify(note, null, 2);
      },

      exportAllNotes: () => {
        const data = {
          exportedAt: new Date().toISOString(),
          app: 'Bennett Studio Notes',
          version: '1.0',
          notes: get().notes,
        };
        return JSON.stringify(data, null, 2);
      },

      exportSelected: () => {
        const { notes, selectedNoteIds } = get();
        const selected = notes.filter((n) => selectedNoteIds.includes(n.id));
        return JSON.stringify({
          exportedAt: new Date().toISOString(),
          app: 'Bennett Studio Notes',
          version: '1.0',
          notes: selected,
        }, null, 2);
      },

      importNotes: (json) => {
        try {
          const data = JSON.parse(json);
          const imported = Array.isArray(data.notes) ? data.notes : Array.isArray(data) ? data : [];
          const validNotes = imported
            .filter((n: any) => n.id && n.content !== undefined)
            .map((n: any) => ({
              ...n,
              id: generateId(),
              createdAt: n.createdAt || new Date().toISOString(),
              updatedAt: n.updatedAt || new Date().toISOString(),
              wordCount: countWords(n.content || ''),
              charCount: (n.content || '').length,
            }));
          set((state) => ({
            notes: [...validNotes, ...state.notes],
          }));
        } catch (e) {
          console.error('Import failed:', e);
          alert('Import failed');
        }
      },

      getFilteredNotes: () => {
        const { notes, searchQuery, activeFilter, activeTag } = get();
        let filtered = [...notes];

        switch (activeFilter) {
          case 'pinned':
            filtered = filtered.filter((n) => n.pinned && !n.archived);
            break;
          case 'recent':
            filtered = filtered.filter((n) => !n.archived).sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime());
            break;
          case 'archived':
            filtered = filtered.filter((n) => n.archived);
            break;
          case 'tags':
            if (activeTag) {
              filtered = filtered.filter((n) => n.tags.includes(activeTag) && !n.archived);
            } else {
              filtered = filtered.filter((n) => !n.archived);
            }
            break;
          default:
            filtered = filtered.filter((n) => !n.archived);
        }

        if (searchQuery.trim()) {
          const q = searchQuery.toLowerCase();
          filtered = filtered.filter(
            (n) =>
              n.title.toLowerCase().includes(q) ||
              n.content.toLowerCase().includes(q) ||
              n.tags.some((t) => t.toLowerCase().includes(q))
          );
        }

        return filtered.sort((a, b) => {
          if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
          return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
        });
      },

      getAllTags: () => {
        const tags = new Set<string>();
        get().notes.forEach((n) => n.tags.forEach((t) => tags.add(t)));
        return Array.from(tags).sort();
      },

      getNoteStats: () => {
        const { notes } = get();
        return {
          total: notes.filter((n) => !n.archived).length,
          pinned: notes.filter((n) => n.pinned && !n.archived).length,
          archived: notes.filter((n) => n.archived).length,
          words: notes.reduce((sum, n) => sum + n.wordCount, 0),
        };
      },

      getSelectedCount: () => get().selectedNoteIds.length,
    }),
    {
      name: 'bennett-notes-storage-v1',
      onRehydrateStorage: () => (state) => {
        // After hydration from localStorage, try to sync with engine
        if (state) {
          setTimeout(() => {
            // @ts-ignore
            state.loadFromEngine?.();
          }, 1000);
        }
      },
      storage: {
        getItem: (name) => {
          const value = localStorage.getItem(name);
          return value ? JSON.parse(value) : null;
        },
        setItem: (name, value) => {
          localStorage.setItem(name, JSON.stringify(value));
        },
        removeItem: (name) => {
          localStorage.removeItem(name);
        },
      },
    }
  )
);

export default useNotesStore;
