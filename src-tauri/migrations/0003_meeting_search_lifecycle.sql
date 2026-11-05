-- Repair stale search content and index every transcript/summary for a meeting.
-- Keep 0002 unchanged so existing installations can apply this migration.
DROP TRIGGER meeting_search_transcript_insert;
DROP TRIGGER meeting_search_transcript_update;
DROP TRIGGER meeting_search_summary_insert;
DROP TRIGGER meeting_search_summary_update;

CREATE TRIGGER meeting_search_transcript_insert
AFTER INSERT ON transcripts
BEGIN
    UPDATE meeting_search
    SET transcript = COALESCE((
        SELECT group_concat(content, char(10)) FROM transcripts WHERE meeting_id = new.meeting_id
    ), '')
    WHERE rowid = new.meeting_id;
END;

CREATE TRIGGER meeting_search_transcript_update
AFTER UPDATE OF content ON transcripts
BEGIN
    UPDATE meeting_search
    SET transcript = COALESCE((
        SELECT group_concat(content, char(10)) FROM transcripts WHERE meeting_id = new.meeting_id
    ), '')
    WHERE rowid = new.meeting_id;
END;

CREATE TRIGGER meeting_search_transcript_delete
AFTER DELETE ON transcripts
BEGIN
    UPDATE meeting_search
    SET transcript = COALESCE((
        SELECT group_concat(content, char(10)) FROM transcripts WHERE meeting_id = old.meeting_id
    ), '')
    WHERE rowid = old.meeting_id;
END;

CREATE TRIGGER meeting_search_summary_insert
AFTER INSERT ON summaries
BEGIN
    UPDATE meeting_search
    SET summary = COALESCE((
        SELECT group_concat(printf('%s %s %s', key_points, decisions, action_items), char(10))
        FROM summaries WHERE meeting_id = new.meeting_id
    ), '')
    WHERE rowid = new.meeting_id;
END;

CREATE TRIGGER meeting_search_summary_update
AFTER UPDATE OF key_points, decisions, action_items ON summaries
BEGIN
    UPDATE meeting_search
    SET summary = COALESCE((
        SELECT group_concat(printf('%s %s %s', key_points, decisions, action_items), char(10))
        FROM summaries WHERE meeting_id = new.meeting_id
    ), '')
    WHERE rowid = new.meeting_id;
END;

CREATE TRIGGER meeting_search_summary_delete
AFTER DELETE ON summaries
BEGIN
    UPDATE meeting_search
    SET summary = COALESCE((
        SELECT group_concat(printf('%s %s %s', key_points, decisions, action_items), char(10))
        FROM summaries WHERE meeting_id = old.meeting_id
    ), '')
    WHERE rowid = old.meeting_id;
END;

-- Rebuild from source rows, removing already-stale terms without deleting notes.
DELETE FROM meeting_search;
INSERT INTO meeting_search (rowid, title, transcript, summary)
SELECT
    m.id,
    m.title,
    COALESCE((
        SELECT group_concat(content, char(10)) FROM transcripts WHERE meeting_id = m.id
    ), ''),
    COALESCE((
        SELECT group_concat(printf('%s %s %s', key_points, decisions, action_items), char(10))
        FROM summaries WHERE meeting_id = m.id
    ), '')
FROM meetings m;
