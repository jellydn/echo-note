-- FTS5 full-text search over meeting titles, transcripts, and summaries (issue #14).
-- The virtual table is kept in sync with the source tables via triggers.
--
-- Note: sync triggers use UPDATE-then-INSERT (rather than DELETE-then-INSERT)
-- because FTS5 raises a PRIMARY KEY constraint error when the same rowid is
-- deleted and re-inserted within one trigger firing.

CREATE VIRTUAL TABLE IF NOT EXISTS meeting_search USING fts5(
    title,
    transcript,
    summary
);

-- Backfill the index with any rows that predate this migration.
INSERT INTO meeting_search (rowid, title, transcript, summary)
SELECT
    m.id,
    m.title,
    COALESCE(t.content, ''),
    COALESCE(
        printf('%s %s %s', s.key_points, s.decisions, s.action_items),
        ''
    )
FROM meetings m
LEFT JOIN transcripts t ON t.meeting_id = m.id
LEFT JOIN summaries s ON s.meeting_id = m.id;

-- Meetings: index on insert, drop on delete, refresh title on update.
CREATE TRIGGER IF NOT EXISTS meeting_search_insert
AFTER INSERT ON meetings
BEGIN
    INSERT INTO meeting_search (rowid, title, transcript, summary)
    VALUES (new.id, new.title, '', '');
END;

CREATE TRIGGER IF NOT EXISTS meeting_search_delete
AFTER DELETE ON meetings
BEGIN
    DELETE FROM meeting_search WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS meeting_search_update
AFTER UPDATE ON meetings
BEGIN
    UPDATE meeting_search
    SET title = new.title
    WHERE rowid = new.id;
END;

-- Transcripts: refresh the affected meeting's index row.
CREATE TRIGGER IF NOT EXISTS meeting_search_transcript_insert
AFTER INSERT ON transcripts
BEGIN
    UPDATE meeting_search
    SET transcript = new.content
    WHERE rowid = new.meeting_id;
    INSERT INTO meeting_search (rowid, title, transcript, summary)
    SELECT
        m.id,
        m.title,
        COALESCE(t.content, ''),
        COALESCE(
            printf('%s %s %s', s.key_points, s.decisions, s.action_items),
            ''
        )
    FROM meetings m
    LEFT JOIN transcripts t ON t.meeting_id = m.id
    LEFT JOIN summaries s ON s.meeting_id = m.id
    WHERE m.id = new.meeting_id
      AND m.id NOT IN (SELECT rowid FROM meeting_search);
END;

CREATE TRIGGER IF NOT EXISTS meeting_search_transcript_update
AFTER UPDATE ON transcripts
BEGIN
    UPDATE meeting_search
    SET transcript = new.content
    WHERE rowid = new.meeting_id;
    INSERT INTO meeting_search (rowid, title, transcript, summary)
    SELECT
        m.id,
        m.title,
        COALESCE(t.content, ''),
        COALESCE(
            printf('%s %s %s', s.key_points, s.decisions, s.action_items),
            ''
        )
    FROM meetings m
    LEFT JOIN transcripts t ON t.meeting_id = m.id
    LEFT JOIN summaries s ON s.meeting_id = m.id
    WHERE m.id = new.meeting_id
      AND m.id NOT IN (SELECT rowid FROM meeting_search);
END;

-- Summaries: refresh the affected meeting's index row.
CREATE TRIGGER IF NOT EXISTS meeting_search_summary_insert
AFTER INSERT ON summaries
BEGIN
    UPDATE meeting_search
    SET summary = printf('%s %s %s', new.key_points, new.decisions, new.action_items)
    WHERE rowid = new.meeting_id;
    INSERT INTO meeting_search (rowid, title, transcript, summary)
    SELECT
        m.id,
        m.title,
        COALESCE(t.content, ''),
        COALESCE(
            printf('%s %s %s', s.key_points, s.decisions, s.action_items),
            ''
        )
    FROM meetings m
    LEFT JOIN transcripts t ON t.meeting_id = m.id
    LEFT JOIN summaries s ON s.meeting_id = m.id
    WHERE m.id = new.meeting_id
      AND m.id NOT IN (SELECT rowid FROM meeting_search);
END;

CREATE TRIGGER IF NOT EXISTS meeting_search_summary_update
AFTER UPDATE ON summaries
BEGIN
    UPDATE meeting_search
    SET summary = printf('%s %s %s', new.key_points, new.decisions, new.action_items)
    WHERE rowid = new.meeting_id;
    INSERT INTO meeting_search (rowid, title, transcript, summary)
    SELECT
        m.id,
        m.title,
        COALESCE(t.content, ''),
        COALESCE(
            printf('%s %s %s', s.key_points, s.decisions, s.action_items),
            ''
        )
    FROM meetings m
    LEFT JOIN transcripts t ON t.meeting_id = m.id
    LEFT JOIN summaries s ON s.meeting_id = m.id
    WHERE m.id = new.meeting_id
      AND m.id NOT IN (SELECT rowid FROM meeting_search);
END;
