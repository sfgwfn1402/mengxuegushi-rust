CREATE TABLE IF NOT EXISTS user_idiom_progress (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    idiom_id INTEGER NOT NULL,
    learned BOOLEAN NOT NULL DEFAULT FALSE,
    read_count INTEGER NOT NULL DEFAULT 0,
    quiz_correct_count INTEGER NOT NULL DEFAULT 0,
    quiz_wrong_count INTEGER NOT NULL DEFAULT 0,
    last_learned_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, idiom_id),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_checkins (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    checkin_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, checkin_date),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_daily_tasks (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    task_date DATE NOT NULL,
    task_id TEXT NOT NULL,
    stars INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, task_date, task_id),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_stats (
    user_id TEXT PRIMARY KEY,
    stars INTEGER NOT NULL DEFAULT 0,
    total_days INTEGER NOT NULL DEFAULT 0,
    streak INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_idiom_progress_user_id ON user_idiom_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_checkins_user_date ON user_checkins(user_id, checkin_date);
CREATE INDEX IF NOT EXISTS idx_daily_tasks_user_date ON user_daily_tasks(user_id, task_date);
