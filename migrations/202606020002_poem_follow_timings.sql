CREATE TABLE IF NOT EXISTS poem_follow_timings (
    poem_id INTEGER PRIMARY KEY REFERENCES poems(id) ON DELETE CASCADE,
    lines_json JSONB NOT NULL,
    source TEXT NOT NULL DEFAULT 'manual',
    status TEXT NOT NULL DEFAULT 'active',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE poem_follow_timings IS '古诗跟读专用分句时间轴；与官方朗读高亮 poem_line_timings 分离';
COMMENT ON COLUMN poem_follow_timings.lines_json IS '跟读分句 JSON 数组：[{index,text,start,end,...}]';
COMMENT ON COLUMN poem_follow_timings.source IS '来源：funasr/manual/special 等';
COMMENT ON COLUMN poem_follow_timings.status IS '状态：active/manual_review 等';

INSERT INTO poem_follow_timings (poem_id, lines_json, source, status, updated_at)
VALUES (
    9,
    '[{"index":0,"text":"鹅，鹅，鹅，","start":7.55,"end":9.45,"source":"manual_follow"},{"index":1,"text":"曲项向天歌。","start":10.25,"end":13.75,"source":"manual_follow"},{"index":2,"text":"白毛浮绿水，","start":14.95,"end":17.75,"source":"manual_follow"},{"index":3,"text":"红掌拨清波。","start":18.25,"end":22.05,"source":"manual_follow"}]'::jsonb,
    'manual_follow',
    'active',
    NOW()
)
ON CONFLICT (poem_id) DO UPDATE SET
    lines_json = EXCLUDED.lines_json,
    source = EXCLUDED.source,
    status = EXCLUDED.status,
    updated_at = NOW();
