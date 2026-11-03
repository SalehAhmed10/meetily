-- Action items are read from each meeting's summary; this only remembers which ones were
-- ticked off. A row means done. item_key is the task text, lowercased with spaces collapsed.
CREATE TABLE IF NOT EXISTS action_item_status (
    meeting_id TEXT NOT NULL REFERENCES meetings (id) ON DELETE CASCADE,
    item_key TEXT NOT NULL,
    done_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, item_key)
);
