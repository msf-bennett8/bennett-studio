use sqlx::{sqlite::SqlitePool, Row};
use crate::models::note::{Note, CreateNoteRequest, UpdateNoteRequest};
use chrono::{DateTime, Utc};

pub struct NotesStore {
    pool: SqlitePool,
}

impl NotesStore {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(db_path).await?;
        Self::init_tables(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn from_pool(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        Self::init_tables(&pool).await?;
        Ok(Self { pool })
    }

    async fn init_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New Note',
                content TEXT NOT NULL DEFAULT '',
                tags TEXT NOT NULL DEFAULT '[]',
                pinned INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                color TEXT NOT NULL DEFAULT '#00d4aa',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                word_count INTEGER NOT NULL DEFAULT 0,
                char_count INTEGER NOT NULL DEFAULT 0,
                device_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notes_pinned ON notes(pinned) WHERE pinned = 1;
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn list_notes(&self, include_archived: bool) -> Result<Vec<Note>, sqlx::Error> {
        let query = if include_archived {
            r#"SELECT id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id 
               FROM notes ORDER BY pinned DESC, updated_at DESC"#
        } else {
            r#"SELECT id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id 
               FROM notes WHERE archived = 0 ORDER BY pinned DESC, updated_at DESC"#
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_note(r)).collect())
    }

    pub async fn get_note(&self, id: &str) -> Result<Option<Note>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id 
               FROM notes WHERE id = ?"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.row_to_note(r)))
    }

    pub async fn create_note(&self, req: CreateNoteRequest, device_id: Option<String>) -> Result<Note, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let word_count = Note::word_count(&req.content);
        let char_count = Note::char_count(&req.content);
        let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"INSERT INTO notes (id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&id)
        .bind(&req.title)
        .bind(&req.content)
        .bind(&tags_json)
        .bind(req.pinned as i32)
        .bind(0i32)
        .bind(&req.color)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(word_count)
        .bind(char_count)
        .bind(device_id.as_ref())
        .execute(&self.pool)
        .await?;

        Ok(Note {
            id,
            title: req.title,
            content: req.content,
            tags: req.tags,
            pinned: req.pinned,
            archived: false,
            color: req.color,
            created_at: now,
            updated_at: now,
            word_count,
            char_count,
            device_id,
        })
    }

    pub async fn update_note(&self, id: &str, req: UpdateNoteRequest) -> Result<Option<Note>, sqlx::Error> {
        let existing = self.get_note(id).await?;
        let Some(mut note) = existing else { return Ok(None); };

        if let Some(title) = req.title { note.title = title; }
        if let Some(content) = req.content { 
            note.content = content.clone();
            note.word_count = Note::word_count(&content);
            note.char_count = Note::char_count(&content);
        }
        if let Some(tags) = req.tags { note.tags = tags; }
        if let Some(color) = req.color { note.color = color; }
        if let Some(pinned) = req.pinned { note.pinned = pinned; }
        if let Some(archived) = req.archived { note.archived = archived; }
        
        note.updated_at = Utc::now();
        let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"UPDATE notes SET title = ?, content = ?, tags = ?, pinned = ?, archived = ?, color = ?, updated_at = ?, word_count = ?, char_count = ?
               WHERE id = ?"#
        )
        .bind(&note.title)
        .bind(&note.content)
        .bind(&tags_json)
        .bind(note.pinned as i32)
        .bind(note.archived as i32)
        .bind(&note.color)
        .bind(note.updated_at.to_rfc3339())
        .bind(note.word_count)
        .bind(note.char_count)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(Some(note))
    }

    pub async fn delete_note(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected() > 0)
    }

    pub async fn search_notes(&self, query: &str) -> Result<Vec<Note>, sqlx::Error> {
        let search_pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"SELECT id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id 
               FROM notes WHERE archived = 0 AND (title LIKE ? OR content LIKE ?)
               ORDER BY pinned DESC, updated_at DESC"#
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_note(r)).collect())
    }

    pub async fn sync_notes(&self, notes: Vec<Note>, device_id: &str) -> Result<Vec<Note>, sqlx::Error> {
        for note in notes {
            // Upsert: update if exists (and newer), insert if not
            let existing = self.get_note(&note.id).await?;
            
            match existing {
                Some(existing) if note.updated_at > existing.updated_at => {
                    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
                    sqlx::query(
                        r#"UPDATE notes SET title = ?, content = ?, tags = ?, pinned = ?, archived = ?, color = ?, updated_at = ?, word_count = ?, char_count = ?, device_id = ?
                           WHERE id = ?"#
                    )
                    .bind(&note.title)
                    .bind(&note.content)
                    .bind(&tags_json)
                    .bind(note.pinned as i32)
                    .bind(note.archived as i32)
                    .bind(&note.color)
                    .bind(note.updated_at.to_rfc3339())
                    .bind(note.word_count)
                    .bind(note.char_count)
                    .bind(device_id)
                    .bind(&note.id)
                    .execute(&self.pool)
                    .await?;
                }
                None => {
                    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
                    sqlx::query(
                        r#"INSERT INTO notes (id, title, content, tags, pinned, archived, color, created_at, updated_at, word_count, char_count, device_id)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
                    )
                    .bind(&note.id)
                    .bind(&note.title)
                    .bind(&note.content)
                    .bind(&tags_json)
                    .bind(note.pinned as i32)
                    .bind(note.archived as i32)
                    .bind(&note.color)
                    .bind(note.created_at.to_rfc3339())
                    .bind(note.updated_at.to_rfc3339())
                    .bind(note.word_count)
                    .bind(note.char_count)
                    .bind(device_id)
                    .execute(&self.pool)
                    .await?;
                }
                _ => {} // Existing is newer, skip
            }
        }

        self.list_notes(false).await
    }

    fn row_to_note(&self, r: sqlx::sqlite::SqliteRow) -> Note {
        let tags_json: String = r.get("tags");
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Note {
            id: r.get("id"),
            title: r.get("title"),
            content: r.get("content"),
            tags,
            pinned: r.get::<i32, _>("pinned") != 0,
            archived: r.get::<i32, _>("archived") != 0,
            color: r.get("color"),
            created_at: DateTime::parse_from_rfc3339(&r.get::<String, _>("created_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&r.get::<String, _>("updated_at"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            word_count: r.get("word_count"),
            char_count: r.get("char_count"),
            device_id: r.get("device_id"),
        }
    }
}
