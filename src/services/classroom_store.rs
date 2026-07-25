use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::classroom::{ClassEvent, ClassInfo, ClassMemberInfo, ClassTreeResponse},
};

fn internal(e: sqlx::Error) -> AppError {
    AppError::Internal(e.to_string())
}

/// 6 位邀请码(取 uuid 前 6 位十六进制并大写)。
fn gen_code() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}

fn display_name(nickname: Option<String>) -> String {
    nickname
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "小诗人".to_string())
}

/// 建群:生成唯一邀请码,建班级 + 把创建者加为 owner。
pub async fn create_class(db: &PgPool, owner_id: &str, name: &str) -> Result<ClassInfo, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("班级名不能为空".into()));
    }
    let id = Uuid::new_v4().to_string();
    let mut code = gen_code();
    for _ in 0..8 {
        let taken: Option<i32> = sqlx::query_scalar("SELECT 1 FROM classes WHERE invite_code = $1")
            .bind(&code)
            .fetch_optional(db)
            .await
            .map_err(internal)?;
        if taken.is_none() {
            break;
        }
        code = gen_code();
    }
    sqlx::query("INSERT INTO classes (id, name, invite_code, owner_id) VALUES ($1, $2, $3, $4)")
        .bind(&id)
        .bind(name)
        .bind(&code)
        .bind(owner_id)
        .execute(db)
        .await
        .map_err(internal)?;
    sqlx::query("INSERT INTO class_members (class_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(&id)
        .bind(owner_id)
        .execute(db)
        .await
        .map_err(internal)?;
    class_info(db, &id, owner_id).await
}

/// 加群:凭邀请码加入(已在群则幂等)。
pub async fn join_class(db: &PgPool, user_id: &str, code: &str) -> Result<ClassInfo, AppError> {
    let code = code.trim().to_uppercase();
    let class_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM classes WHERE invite_code = $1")
            .bind(&code)
            .fetch_optional(db)
            .await
            .map_err(internal)?;
    let class_id = class_id.ok_or_else(|| AppError::NotFound("班级不存在或邀请码有误".into()))?;
    sqlx::query(
        "INSERT INTO class_members (class_id, user_id, role) VALUES ($1, $2, 'member')
         ON CONFLICT (class_id, user_id) DO NOTHING",
    )
    .bind(&class_id)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(internal)?;
    class_info(db, &class_id, user_id).await
}

/// 退群(owner 退群同时解散整个班级)。
pub async fn leave_class(db: &PgPool, user_id: &str, class_id: &str) -> Result<(), AppError> {
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_id FROM classes WHERE id = $1")
        .bind(class_id)
        .fetch_optional(db)
        .await
        .map_err(internal)?;
    let owner = owner.ok_or_else(|| AppError::NotFound("班级不存在".into()))?;
    if owner == user_id {
        sqlx::query("DELETE FROM classes WHERE id = $1")
            .bind(class_id)
            .execute(db)
            .await
            .map_err(internal)?;
    } else {
        sqlx::query("DELETE FROM class_members WHERE class_id = $1 AND user_id = $2")
            .bind(class_id)
            .bind(user_id)
            .execute(db)
            .await
            .map_err(internal)?;
    }
    Ok(())
}

/// 我加入的班级列表。
pub async fn my_classes(db: &PgPool, user_id: &str) -> Result<Vec<ClassInfo>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
        r#"
        SELECT c.id, c.name, c.invite_code, c.owner_id,
               (SELECT COUNT(*) FROM class_members m2 WHERE m2.class_id = c.id)::BIGINT
        FROM classes c
        JOIN class_members m ON m.class_id = c.id
        WHERE m.user_id = $1
        ORDER BY m.joined_at
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(internal)?;
    Ok(rows
        .into_iter()
        .map(|(id, name, code, owner, count)| ClassInfo {
            is_owner: owner == user_id,
            id,
            name,
            invite_code: code,
            owner_id: owner,
            member_count: count,
        })
        .collect())
}

async fn class_info(db: &PgPool, class_id: &str, user_id: &str) -> Result<ClassInfo, AppError> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64)>(
        r#"
        SELECT c.id, c.name, c.invite_code, c.owner_id,
               (SELECT COUNT(*) FROM class_members m2 WHERE m2.class_id = c.id)::BIGINT
        FROM classes c WHERE c.id = $1
        "#,
    )
    .bind(class_id)
    .fetch_optional(db)
    .await
    .map_err(internal)?
    .ok_or_else(|| AppError::NotFound("班级不存在".into()))?;
    Ok(ClassInfo {
        is_owner: row.3 == user_id,
        id: row.0,
        name: row.1,
        invite_code: row.2,
        owner_id: row.3,
        member_count: row.4,
    })
}

/// 班级诗词树:聚合全班学习进度 + 成员 + 近期动态。要求本人是成员。
pub async fn class_tree(
    db: &PgPool,
    class_id: &str,
    user_id: &str,
) -> Result<ClassTreeResponse, AppError> {
    let info = class_info(db, class_id, user_id).await?;
    let is_member: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM class_members WHERE class_id = $1 AND user_id = $2")
            .bind(class_id)
            .bind(user_id)
            .fetch_optional(db)
            .await
            .map_err(internal)?;
    if is_member.is_none() {
        return Err(AppError::Forbidden("你不在这个班级里".into()));
    }

    // 成员(各自**入班后**新点亮的诗数 —— 只算一起走的路,不含入班前的老底子)
    let member_rows = sqlx::query_as::<_, (String, Option<String>, Option<String>, String, i64)>(
        r#"
        SELECT u.id, u.nickname, u.avatar_url, m.role,
               (SELECT COUNT(*) FROM user_poem_progress p
                WHERE p.user_id = u.id AND p.learned
                  AND p.last_learned_at >= m.joined_at)::BIGINT
        FROM class_members m
        JOIN users u ON u.id = m.user_id
        WHERE m.class_id = $1
        ORDER BY 5 DESC, m.joined_at
        "#,
    )
    .bind(class_id)
    .fetch_all(db)
    .await
    .map_err(internal)?;

    let members: Vec<ClassMemberInfo> = member_rows
        .into_iter()
        .map(|(uid, nick, avatar, role, learned)| ClassMemberInfo {
            user_id: uid,
            name: display_name(nick),
            avatar,
            learned_count: learned,
            is_owner: role == "owner",
        })
        .collect();

    let total_learned: i64 = members.iter().map(|m| m.learned_count).sum();

    let weekly_learned: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM user_poem_progress p
        JOIN class_members m ON m.user_id = p.user_id AND m.class_id = $1
        WHERE p.learned
          AND p.last_learned_at >= GREATEST(m.joined_at, NOW() - INTERVAL '7 days')
        "#,
    )
    .bind(class_id)
    .fetch_one(db)
    .await
    .map_err(internal)?;

    // 近期动态:谁学了哪首诗
    let event_rows = sqlx::query_as::<_, (Option<String>, Option<String>, i32, String, String)>(
        r#"
        SELECT u.nickname, u.avatar_url, p.poem_id, po.title,
               to_char(p.last_learned_at, 'YYYY-MM-DD"T"HH24:MI:SS')
        FROM user_poem_progress p
        JOIN class_members m ON m.user_id = p.user_id AND m.class_id = $1
        JOIN users u ON u.id = p.user_id
        JOIN poems po ON po.id = p.poem_id
        WHERE p.learned AND p.last_learned_at IS NOT NULL
          AND p.last_learned_at >= m.joined_at
        ORDER BY p.last_learned_at DESC
        LIMIT 20
        "#,
    )
    .bind(class_id)
    .fetch_all(db)
    .await
    .map_err(internal)?;

    let events: Vec<ClassEvent> = event_rows
        .into_iter()
        .map(|(nick, avatar, pid, title, at)| ClassEvent {
            user_name: display_name(nick),
            avatar,
            poem_id: pid,
            poem_title: title,
            at,
        })
        .collect();

    Ok(ClassTreeResponse {
        id: info.id,
        name: info.name,
        invite_code: info.invite_code,
        is_owner: info.is_owner,
        member_count: info.member_count,
        total_learned,
        weekly_learned,
        members,
        events,
    })
}
