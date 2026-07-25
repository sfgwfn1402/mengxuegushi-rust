use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateClassRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinClassRequest {
    pub invite_code: String,
}

#[derive(Debug, Serialize)]
pub struct ClassInfo {
    pub id: String,
    pub name: String,
    pub invite_code: String,
    pub owner_id: String,
    pub member_count: i64,
    pub is_owner: bool,
}

#[derive(Debug, Serialize)]
pub struct ClassMemberInfo {
    pub user_id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub learned_count: i64,
    pub is_owner: bool,
}

#[derive(Debug, Serialize)]
pub struct ClassEvent {
    pub user_name: String,
    pub avatar: Option<String>,
    pub poem_id: i32,
    pub poem_title: String,
    pub at: String,
}

/// 班级诗词树:全班聚合状态 + 成员 + 近期动态。
#[derive(Debug, Serialize)]
pub struct ClassTreeResponse {
    pub id: String,
    pub name: String,
    pub invite_code: String,
    pub is_owner: bool,
    pub member_count: i64,
    pub total_learned: i64,   // 全班累计学诗次数(每人每首各算一次,给树浇一次水)
    pub weekly_learned: i64,  // 近 7 天新学
    pub members: Vec<ClassMemberInfo>,
    pub events: Vec<ClassEvent>,
}
