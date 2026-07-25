-- 每首诗的"诗句框"归一化位置（x,y ∈ [0,1]，屏幕宽高百分比），管理员编排、全员共用。
CREATE TABLE IF NOT EXISTS poem_panel_positions (
    poem_id    INTEGER PRIMARY KEY,
    x          DOUBLE PRECISION NOT NULL,
    y          DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
