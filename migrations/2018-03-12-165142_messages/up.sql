-- Your SQL goes here 
CREATE TABLE IF NOT EXISTS messages (
                  id TEXT PRIMARY KEY,
                  channel_id TEXT NOT NULL,
                  author TEXT NOT NULL,
                  content TEXT NOT NULL,
                  timestamp       TEXT NOT NULL)
