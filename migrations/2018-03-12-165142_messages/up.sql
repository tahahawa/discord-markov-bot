-- Your SQL goes here 
CREATE TABLE IF NOT EXISTS messages (
                  id Int8 UNIQUE PRIMARY KEY,
                  channel_id Int8 NOT NULL,
                  author Int8 NOT NULL,
                  content TEXT NOT NULL,
                  timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now() NOT NULL)
