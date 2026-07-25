-- 会员订阅验单记账：客户端购买/续期/恢复后上报 StoreKit 2 签名交易(JWS)，
-- 服务端验签落库，作为「谁是会员」的服务器真相源（管理端可见、后续额度下发依据）。
CREATE TABLE IF NOT EXISTS user_subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_transaction_id TEXT NOT NULL UNIQUE,   -- 同一订阅周期家族的唯一键，续期覆盖更新
    transaction_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    bundle_id TEXT NOT NULL,
    purchase_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,                          -- 退款/撤销时间，非空即失效
    environment TEXT NOT NULL DEFAULT 'Production',  -- Xcode | Sandbox | Production
    signed_transaction TEXT NOT NULL,                -- 原始 JWS 留存复核
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_subscriptions_user ON user_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_subscriptions_expires ON user_subscriptions(expires_at);
