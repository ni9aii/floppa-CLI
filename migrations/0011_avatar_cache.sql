-- Server-side avatar cache.
--
-- Telegram profile photos are served from Telegram's CDN, which is unreachable from clients in
-- Russia (and the CDN sends no CORS headers, so client-side fetch caching can't populate either).
-- The server (in a datacenter, where Telegram is reachable) downloads the photo — via the Bot API
-- (getUserProfilePhotos → getFile → download), falling back to the stored photo_url — and caches
-- the bytes here, serving them from our own origin.
--
-- Separate table (not columns on users) so the hot users table stays lean and blobs aren't pulled
-- into ordinary user queries.

CREATE TABLE IF NOT EXISTS user_avatars (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    blob BYTEA NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'image/jpeg',
    -- Strong validator for HTTP caching + change detection (hash of the bytes).
    etag TEXT NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Find stale avatars to refresh (TTL-based refetch).
CREATE INDEX IF NOT EXISTS idx_user_avatars_fetched_at ON user_avatars(fetched_at);
