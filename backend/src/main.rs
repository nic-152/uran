use anyhow::Context;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{any, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::{
    env,
    net::SocketAddr,
    path::{Path as StdPath, PathBuf},
    sync::Arc,
    time::SystemTime,
};
use tokio::{fs, sync::Mutex};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use uuid::Uuid;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Clone)]
struct AppState {
    users_file: PathBuf,
    projects_file: PathBuf,
    file_lock: Arc<Mutex<()>>,
    db: PgPool,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    name: String,
    email: String,
    password: String,
    created_at: String,
}

#[derive(Serialize, Deserialize)]
struct UsersFile {
    users: Vec<User>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProjectMember {
    user_id: String,
    role: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: String,
    name: String,
    owner_id: String,
    created_at: String,
    updated_at: String,
    members: Vec<ProjectMember>,
    session: Option<Value>,
}

#[derive(Serialize, Deserialize)]
struct ProjectsFile {
    projects: Vec<Project>,
}

#[derive(Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    user: SafeUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeUser {
    id: String,
    name: String,
    email: String,
    created_at: String,
}

#[derive(Serialize)]
struct MeResponse {
    user: SafeUser,
}

#[derive(Serialize)]
struct ProjectsResponse {
    projects: Vec<ProjectForUser>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectForUser {
    id: String,
    name: String,
    role: String,
    owner_id: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
}

#[derive(Serialize)]
struct CreateProjectResponse {
    project: ProjectForUser,
}

#[derive(Deserialize)]
struct AddMemberRequest {
    email: String,
    role: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddedMember {
    id: String,
    email: String,
    name: String,
    role: String,
}

#[derive(Serialize)]
struct AddMemberResponse {
    added: AddedMember,
    project: ProjectForUser,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectMemberView {
    user_id: String,
    role: String,
    email: String,
    name: String,
}

#[derive(Serialize)]
struct MembersResponse {
    members: Vec<ProjectMemberView>,
}

#[derive(Deserialize)]
struct UpdateMemberRoleRequest {
    role: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateMemberRoleResponse {
    member: ProjectMemberView,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoveMemberResponse {
    ok: bool,
    updated_at: String,
}

#[derive(Serialize)]
struct ProjectSessionResponse {
    project: ProjectForUser,
    session: Option<Value>,
}

#[derive(Deserialize)]
struct SaveSessionRequest {
    session: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveSessionResponse {
    ok: bool,
    updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRunRequest {
    project_id: String,
    asset_id: Option<String>,
    template_id: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddRunItemRequest {
    testcase_version_id: String,
    position: Option<i32>,
    is_required: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRunResultRequest {
    status: String,
    fail_reason_code: Option<String>,
    comment: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRunStatusRequest {
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListRunsQuery {
    project_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunView {
    id: String,
    project_id: String,
    asset_id: Option<String>,
    template_id: Option<String>,
    title: String,
    status: String,
    executed_by_user_id: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    locked_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunItemView {
    id: String,
    testcase_version_id: String,
    position: i32,
    is_required: bool,
    status: String,
    fail_reason_code: Option<String>,
    comment: String,
    updated_at: Option<String>,
}

#[derive(Serialize)]
struct CreateRunResponse {
    run: RunView,
}

#[derive(Serialize)]
struct ListRunsResponse {
    runs: Vec<RunView>,
}

#[derive(Serialize)]
struct RunDetailsResponse {
    run: RunView,
    items: Vec<RunItemView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRunResultResponse {
    ok: bool,
    updated_at: String,
}

#[derive(Serialize)]
struct UpdateRunStatusResponse {
    run: RunView,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "uran-api",
    })
}

fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn now_iso() -> String {
    chrono::DateTime::<chrono::Utc>::from(SystemTime::now()).to_rfc3339()
}

fn map_safe_user(user: &User) -> SafeUser {
    SafeUser {
        id: user.id.clone(),
        name: user.name.clone(),
        email: user.email.clone(),
        created_at: user.created_at.clone(),
    }
}

fn membership_role(project: &Project, user_id: &str) -> Option<String> {
    project
        .members
        .iter()
        .find(|m| m.user_id == user_id)
        .map(|m| m.role.clone())
}

fn map_project_for_user(project: &Project, user_id: &str) -> Option<ProjectForUser> {
    let role = membership_role(project, user_id)?;
    Some(ProjectForUser {
        id: project.id.clone(),
        name: project.name.clone(),
        role,
        owner_id: project.owner_id.clone(),
        created_at: project.created_at.clone(),
        updated_at: project.updated_at.clone(),
    })
}

fn can_write_project(role: &str) -> bool {
    role == "owner" || role == "editor"
}

fn parse_bearer_user_id(headers: &HeaderMap) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !auth.starts_with("Bearer ") {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Требуется авторизация.",
        ));
    }
    let token = auth.trim_start_matches("Bearer ").trim();
    if !token.starts_with("uran.") {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Недействительный токен.",
        ));
    }
    let user_id = token.trim_start_matches("uran.").to_string();
    if Uuid::parse_str(&user_id).is_err() {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Недействительный токен.",
        ));
    }
    Ok(user_id)
}

async fn ensure_json_file(path: &StdPath, content: &str) -> anyhow::Result<()> {
    if fs::metadata(path).await.is_ok() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, content).await?;
    Ok(())
}

async fn read_users(path: &StdPath) -> anyhow::Result<Vec<User>> {
    ensure_json_file(path, "{\n  \"users\": []\n}\n").await?;
    let raw = fs::read_to_string(path).await?;
    let parsed: UsersFile = serde_json::from_str(&raw)?;
    Ok(parsed.users)
}

async fn write_users(path: &StdPath, users: &[User]) -> anyhow::Result<()> {
    let data = UsersFile {
        users: users.to_vec(),
    };
    let raw = serde_json::to_string_pretty(&data)?;
    fs::write(path, raw).await?;
    Ok(())
}

async fn read_projects(path: &StdPath) -> anyhow::Result<Vec<Project>> {
    ensure_json_file(path, "{\n  \"projects\": []\n}\n").await?;
    let raw = fs::read_to_string(path).await?;
    let parsed: ProjectsFile = serde_json::from_str(&raw)?;
    Ok(parsed.projects)
}

async fn write_projects(path: &StdPath, projects: &[Project]) -> anyhow::Result<()> {
    let data = ProjectsFile {
        projects: projects.to_vec(),
    };
    let raw = serde_json::to_string_pretty(&data)?;
    fs::write(path, raw).await?;
    Ok(())
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    let name = payload.name.trim();
    let email = payload.email.trim().to_lowercase();
    let password = payload.password;

    if name.chars().count() < 2 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Имя должно быть не короче 2 символов.",
        ));
    }
    if !email.contains('@') {
        return Err(api_error(StatusCode::BAD_REQUEST, "Некорректный email."));
    }
    if password.chars().count() < 8 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Пароль должен быть не короче 8 символов.",
        ));
    }

    let _guard = state.file_lock.lock().await;
    let mut users = read_users(&state.users_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка регистрации."))?;

    if users.iter().any(|u| u.email == email) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Пользователь с таким email уже существует.",
        ));
    }

    let user = User {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        email,
        password,
        created_at: now_iso(),
    };
    users.push(user.clone());
    write_users(&state.users_file, &users)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка регистрации."))?;

    let token = format!("uran.{}", user.id);
    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: map_safe_user(&user),
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let email = payload.email.trim().to_lowercase();
    let password = payload.password;

    let _guard = state.file_lock.lock().await;
    let users = read_users(&state.users_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка входа."))?;

    let user = users
        .iter()
        .find(|u| u.email == email && u.password == password)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Неверный email или пароль."))?;

    let token = format!("uran.{}", user.id);
    Ok(Json(AuthResponse {
        token,
        user: map_safe_user(&user),
    }))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let users = read_users(&state.users_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка загрузки профиля."))?;
    let user = users
        .iter()
        .find(|u| u.id == user_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Пользователь не найден."))?;

    Ok(Json(MeResponse {
        user: map_safe_user(user),
    }))
}

async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProjectsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let projects = read_projects(&state.projects_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка загрузки проектов."))?;

    let visible: Vec<ProjectForUser> = projects
        .iter()
        .filter_map(|p| map_project_for_user(p, &user_id))
        .collect();

    Ok(Json(ProjectsResponse { projects: visible }))
}

async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<CreateProjectResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;
    let name = payload.name.trim();

    if name.chars().count() < 3 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Название проекта должно быть не короче 3 символов.",
        ));
    }

    let _guard = state.file_lock.lock().await;
    let mut projects = read_projects(&state.projects_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка создания проекта."))?;

    let now = now_iso();
    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        owner_id: user_id.clone(),
        created_at: now.clone(),
        updated_at: now,
        members: vec![ProjectMember {
            user_id: user_id.clone(),
            role: "owner".to_string(),
        }],
        session: None,
    };
    let mapped = map_project_for_user(&project, &user_id)
        .ok_or_else(|| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка создания проекта."))?;
    projects.push(project);
    write_projects(&state.projects_file, &projects)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка создания проекта."))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateProjectResponse { project: mapped }),
    ))
}

async fn add_member(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<AddMemberResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;
    let email = payload.email.trim().to_lowercase();
    let role = payload.role.trim().to_lowercase();

    if role != "editor" && role != "viewer" {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Роль должна быть editor или viewer.",
        ));
    }
    if !email.contains('@') {
        return Err(api_error(StatusCode::BAD_REQUEST, "Некорректный email."));
    }

    let _guard = state.file_lock.lock().await;
    let users = read_users(&state.users_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка выдачи доступа."))?;
    let invitee = users
        .iter()
        .find(|u| u.email == email)
        .cloned()
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "Пользователь с таким email не найден.",
            )
        })?;

    let mut projects = read_projects(&state.projects_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка выдачи доступа."))?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    let actor_role = membership_role(project, &actor_id)
        .ok_or_else(|| api_error(StatusCode::FORBIDDEN, "Только владелец может управлять доступом."))?;
    if actor_role != "owner" {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Только владелец может управлять доступом.",
        ));
    }

    if let Some(existing) = project.members.iter_mut().find(|m| m.user_id == invitee.id) {
        if invitee.id == project.owner_id {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "Нельзя изменить роль владельца.",
            ));
        }
        existing.role = role.clone();
    } else {
        project.members.push(ProjectMember {
            user_id: invitee.id.clone(),
            role: role.clone(),
        });
    }

    project.updated_at = now_iso();
    let mapped_project = map_project_for_user(project, &actor_id)
        .ok_or_else(|| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка выдачи доступа."))?;
    let updated_at = project.updated_at.clone();
    write_projects(&state.projects_file, &projects)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка выдачи доступа."))?;

    Ok(Json(AddMemberResponse {
        added: AddedMember {
            id: invitee.id,
            email: invitee.email,
            name: invitee.name,
            role,
        },
        project: ProjectForUser {
            updated_at,
            ..mapped_project
        },
    }))
}

async fn list_members(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<MembersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let users = read_users(&state.users_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка загрузки участников."))?;
    let projects = read_projects(&state.projects_file)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка загрузки участников."))?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    if membership_role(project, &user_id).is_none() {
        return Err(api_error(StatusCode::FORBIDDEN, "Нет доступа к проекту."));
    }

    let members = project
        .members
        .iter()
        .map(|m| {
            let user = users.iter().find(|u| u.id == m.user_id);
            ProjectMemberView {
                user_id: m.user_id.clone(),
                role: m.role.clone(),
                email: user.map(|u| u.email.clone()).unwrap_or_default(),
                name: user.map(|u| u.name.clone()).unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(MembersResponse { members }))
}

async fn update_member(
    State(state): State<AppState>,
    Path((project_id, target_user_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<UpdateMemberRoleRequest>,
) -> Result<Json<UpdateMemberRoleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;
    let role = payload.role.trim().to_lowercase();
    if role != "editor" && role != "viewer" {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Роль должна быть editor или viewer.",
        ));
    }

    let _guard = state.file_lock.lock().await;
    let users = read_users(&state.users_file).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка обновления роли участника.",
        )
    })?;
    let mut projects = read_projects(&state.projects_file).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка обновления роли участника.",
        )
    })?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    let actor_role = membership_role(project, &actor_id)
        .ok_or_else(|| api_error(StatusCode::FORBIDDEN, "Только владелец может управлять доступом."))?;
    if actor_role != "owner" {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Только владелец может управлять доступом.",
        ));
    }
    if target_user_id == project.owner_id {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Нельзя изменить роль владельца.",
        ));
    }

    let member = project
        .members
        .iter_mut()
        .find(|m| m.user_id == target_user_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Участник не найден."))?;
    member.role = role;
    let member_snapshot = member.clone();
    project.updated_at = now_iso();
    let updated_at = project.updated_at.clone();

    write_projects(&state.projects_file, &projects).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка обновления роли участника.",
        )
    })?;

    let user = users.iter().find(|u| u.id == member_snapshot.user_id);
    Ok(Json(UpdateMemberRoleResponse {
        member: ProjectMemberView {
            user_id: member_snapshot.user_id,
            role: member_snapshot.role,
            email: user.map(|u| u.email.clone()).unwrap_or_default(),
            name: user.map(|u| u.name.clone()).unwrap_or_default(),
        },
        updated_at,
    }))
}

async fn remove_member(
    State(state): State<AppState>,
    Path((project_id, target_user_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<RemoveMemberResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let mut projects = read_projects(&state.projects_file).await.map_err(|_| {
        api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка удаления участника.")
    })?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    let actor_role = membership_role(project, &actor_id)
        .ok_or_else(|| api_error(StatusCode::FORBIDDEN, "Только владелец может управлять доступом."))?;
    if actor_role != "owner" {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Только владелец может управлять доступом.",
        ));
    }
    if target_user_id == project.owner_id {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Нельзя удалить владельца из проекта.",
        ));
    }
    let before = project.members.len();
    project.members.retain(|m| m.user_id != target_user_id);
    if project.members.len() == before {
        return Err(api_error(StatusCode::NOT_FOUND, "Участник не найден."));
    }

    project.updated_at = now_iso();
    let updated_at = project.updated_at.clone();
    write_projects(&state.projects_file, &projects)
        .await
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка удаления участника."))?;
    Ok(Json(RemoveMemberResponse {
        ok: true,
        updated_at,
    }))
}

async fn get_session(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ProjectSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let projects = read_projects(&state.projects_file).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка загрузки сессии проекта.",
        )
    })?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    let mapped = map_project_for_user(project, &user_id)
        .ok_or_else(|| api_error(StatusCode::FORBIDDEN, "Нет доступа к проекту."))?;
    Ok(Json(ProjectSessionResponse {
        project: mapped,
        session: project.session.clone(),
    }))
}

async fn save_session(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<SaveSessionRequest>,
) -> Result<Json<SaveSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = parse_bearer_user_id(&headers)?;

    let _guard = state.file_lock.lock().await;
    let mut projects = read_projects(&state.projects_file).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка сохранения сессии проекта.",
        )
    })?;
    let project = projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Проект не найден."))?;

    let role = membership_role(project, &user_id)
        .ok_or_else(|| api_error(StatusCode::FORBIDDEN, "Нет доступа к проекту."))?;
    if !can_write_project(&role) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "У вас только режим просмотра.",
        ));
    }

    project.session = Some(payload.session);
    project.updated_at = now_iso();
    let updated_at = project.updated_at.clone();
    write_projects(&state.projects_file, &projects).await.map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка сохранения сессии проекта.",
        )
    })?;

    Ok(Json(SaveSessionResponse {
        ok: true,
        updated_at,
    }))
}

fn parse_uuid(input: &str, err_message: &str) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    Uuid::parse_str(input).map_err(|_| api_error(StatusCode::BAD_REQUEST, err_message))
}

fn parse_run_status(input: &str) -> Result<&'static str, (StatusCode, Json<ErrorResponse>)> {
    match input {
        "draft" => Ok("draft"),
        "in_progress" => Ok("in_progress"),
        "done" => Ok("done"),
        "locked" => Ok("locked"),
        _ => Err(api_error(
            StatusCode::BAD_REQUEST,
            "Некорректный статус run. Ожидается draft|in_progress|done|locked.",
        )),
    }
}

fn parse_result_status(input: &str) -> Result<&'static str, (StatusCode, Json<ErrorResponse>)> {
    match input {
        "ok" => Ok("ok"),
        "fail" => Ok("fail"),
        "na" => Ok("na"),
        _ => Err(api_error(
            StatusCode::BAD_REQUEST,
            "Некорректный статус результата. Ожидается ok|fail|na.",
        )),
    }
}

async fn ensure_db_user_exists(
    state: &AppState,
    user_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let user_uuid = parse_uuid(user_id, "Некорректный идентификатор пользователя.")?;
    let fallback_email = format!("{}@local.invalid", user_uuid);
    let fallback_name = format!("User-{}", &user_id[..8.min(user_id.len())]);

    sqlx::query(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, is_active)
        VALUES ($1, $2, $3, 'external-auth', TRUE)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(user_uuid)
    .bind(fallback_email)
    .bind(fallback_name)
    .execute(&state.db)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Не удалось синхронизировать пользователя в БД.",
        )
    })?;

    Ok(())
}

async fn fetch_run_view(
    db: &PgPool,
    run_id: Uuid,
) -> Result<Option<RunView>, (StatusCode, Json<ErrorResponse>)> {
    let row = sqlx::query(
        r#"
        SELECT
          id::text AS id,
          project_id::text AS project_id,
          asset_id::text AS asset_id,
          template_id::text AS template_id,
          title,
          status::text AS status,
          executed_by_user_id::text AS executed_by_user_id,
          started_at::text AS started_at,
          finished_at::text AS finished_at,
          locked_at::text AS locked_at,
          created_at::text AS created_at,
          updated_at::text AS updated_at
        FROM runs
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения run из БД."))?;

    Ok(row.map(|r| RunView {
        id: r.get::<String, _>("id"),
        project_id: r.get::<String, _>("project_id"),
        asset_id: r.get::<Option<String>, _>("asset_id"),
        template_id: r.get::<Option<String>, _>("template_id"),
        title: r.get::<String, _>("title"),
        status: r.get::<String, _>("status"),
        executed_by_user_id: r.get::<String, _>("executed_by_user_id"),
        started_at: r.get::<Option<String>, _>("started_at"),
        finished_at: r.get::<Option<String>, _>("finished_at"),
        locked_at: r.get::<Option<String>, _>("locked_at"),
        created_at: r.get::<String, _>("created_at"),
        updated_at: r.get::<String, _>("updated_at"),
    }))
}

async fn create_run_v2(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRunRequest>,
) -> Result<(StatusCode, Json<CreateRunResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;
    ensure_db_user_exists(&state, &actor_id).await?;

    let project_id = parse_uuid(&payload.project_id, "Некорректный project_id.")?;
    let asset_id = match payload.asset_id.as_deref() {
        Some(v) if !v.trim().is_empty() => Some(parse_uuid(v, "Некорректный asset_id.")?),
        _ => None,
    };
    let template_id = match payload.template_id.as_deref() {
        Some(v) if !v.trim().is_empty() => Some(parse_uuid(v, "Некорректный template_id.")?),
        _ => None,
    };
    let actor_uuid = parse_uuid(&actor_id, "Некорректный идентификатор пользователя.")?;
    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("New run")
        .to_string();

    let run_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO runs (
          project_id, asset_id, template_id, title, status, executed_by_user_id
        )
        VALUES ($1, $2, $3, $4, 'draft', $5)
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(asset_id)
    .bind(template_id)
    .bind(title)
    .bind(actor_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Не удалось создать run. Проверь проект/asset/template."))?;

    let run = fetch_run_view(&state.db, run_id)
        .await?
        .ok_or_else(|| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Run создан, но не найден."))?;

    Ok((StatusCode::CREATED, Json(CreateRunResponse { run })))
}

async fn list_runs_v2(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListRunsQuery>,
) -> Result<Json<ListRunsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _actor_id = parse_bearer_user_id(&headers)?;
    let project_id = match query.project_id.as_deref() {
        Some(v) if !v.trim().is_empty() => Some(parse_uuid(v, "Некорректный project_id.")?),
        _ => None,
    };
    let status = match query.status.as_deref() {
        Some(v) => Some(parse_run_status(v)?.to_string()),
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query(
        r#"
        SELECT
          id::text AS id,
          project_id::text AS project_id,
          asset_id::text AS asset_id,
          template_id::text AS template_id,
          title,
          status::text AS status,
          executed_by_user_id::text AS executed_by_user_id,
          started_at::text AS started_at,
          finished_at::text AS finished_at,
          locked_at::text AS locked_at,
          created_at::text AS created_at,
          updated_at::text AS updated_at
        FROM runs
        WHERE ($1::uuid IS NULL OR project_id = $1)
          AND ($2::run_status IS NULL OR status = $2)
        ORDER BY created_at DESC
        LIMIT $3
        "#,
    )
    .bind(project_id)
    .bind(status)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения списка runs."))?;

    let runs = rows
        .into_iter()
        .map(|r| RunView {
            id: r.get::<String, _>("id"),
            project_id: r.get::<String, _>("project_id"),
            asset_id: r.get::<Option<String>, _>("asset_id"),
            template_id: r.get::<Option<String>, _>("template_id"),
            title: r.get::<String, _>("title"),
            status: r.get::<String, _>("status"),
            executed_by_user_id: r.get::<String, _>("executed_by_user_id"),
            started_at: r.get::<Option<String>, _>("started_at"),
            finished_at: r.get::<Option<String>, _>("finished_at"),
            locked_at: r.get::<Option<String>, _>("locked_at"),
            created_at: r.get::<String, _>("created_at"),
            updated_at: r.get::<String, _>("updated_at"),
        })
        .collect();

    Ok(Json(ListRunsResponse { runs }))
}

async fn get_run_details_v2(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RunDetailsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _actor_id = parse_bearer_user_id(&headers)?;
    let run_uuid = parse_uuid(&run_id, "Некорректный run_id.")?;

    let run = fetch_run_view(&state.db, run_uuid)
        .await?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Run не найден."))?;

    let rows = sqlx::query(
        r#"
        SELECT
          ri.id::text AS id,
          ri.testcase_version_id::text AS testcase_version_id,
          ri.position AS position,
          ri.is_required AS is_required,
          COALESCE(rr.status::text, 'na') AS status,
          rr.fail_reason_code AS fail_reason_code,
          COALESCE(rr.comment, '') AS comment,
          rr.updated_at::text AS updated_at
        FROM run_items ri
        LEFT JOIN run_results rr ON rr.run_item_id = ri.id
        WHERE ri.run_id = $1
        ORDER BY ri.position ASC, ri.created_at ASC
        "#,
    )
    .bind(run_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения run items."))?;

    let items = rows
        .into_iter()
        .map(|r| RunItemView {
            id: r.get::<String, _>("id"),
            testcase_version_id: r.get::<String, _>("testcase_version_id"),
            position: r.get::<i32, _>("position"),
            is_required: r.get::<bool, _>("is_required"),
            status: r.get::<String, _>("status"),
            fail_reason_code: r.get::<Option<String>, _>("fail_reason_code"),
            comment: r.get::<String, _>("comment"),
            updated_at: r.get::<Option<String>, _>("updated_at"),
        })
        .collect();

    Ok(Json(RunDetailsResponse { run, items }))
}

async fn add_run_item_v2(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<AddRunItemRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;
    ensure_db_user_exists(&state, &actor_id).await?;
    let run_uuid = parse_uuid(&run_id, "Некорректный run_id.")?;
    let testcase_version_id = parse_uuid(
        &payload.testcase_version_id,
        "Некорректный testcase_version_id.",
    )?;
    let actor_uuid = parse_uuid(&actor_id, "Некорректный идентификатор пользователя.")?;
    let position = payload.position.unwrap_or(0);
    let is_required = payload.is_required.unwrap_or(true);

    let run_status: Option<String> = sqlx::query_scalar(
        r#"SELECT status::text FROM runs WHERE id = $1"#,
    )
    .bind(run_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения run."))?;
    let run_status = run_status.ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Run не найден."))?;
    if run_status == "locked" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Run в статусе locked, состав менять нельзя.",
        ));
    }

    let run_item_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO run_items (run_id, testcase_version_id, position, is_required)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(run_uuid)
    .bind(testcase_version_id)
    .bind(position)
    .bind(is_required)
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            "Не удалось добавить пункт в run (проверь testcase_version или дубликат).",
        )
    })?;

    sqlx::query(
        r#"
        INSERT INTO run_results (run_item_id, status, comment, updated_by_user_id)
        VALUES ($1, 'na', '', $2)
        ON CONFLICT (run_item_id) DO NOTHING
        "#,
    )
    .bind(run_item_id)
    .bind(actor_uuid)
    .execute(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось создать run_result."))?;

    Ok(StatusCode::CREATED)
}

async fn update_run_result_v2(
    State(state): State<AppState>,
    Path((run_id, run_item_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<UpdateRunResultRequest>,
) -> Result<Json<UpdateRunResultResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor_id = parse_bearer_user_id(&headers)?;
    ensure_db_user_exists(&state, &actor_id).await?;
    let run_uuid = parse_uuid(&run_id, "Некорректный run_id.")?;
    let run_item_uuid = parse_uuid(&run_item_id, "Некорректный run_item_id.")?;
    let actor_uuid = parse_uuid(&actor_id, "Некорректный идентификатор пользователя.")?;
    let status = parse_result_status(payload.status.trim())?;
    let comment = payload.comment.unwrap_or_default();
    let fail_reason_code = if status == "fail" {
        payload.fail_reason_code
    } else {
        None
    };

    let run_status: Option<String> = sqlx::query_scalar(
        r#"
        SELECT r.status::text
        FROM runs r
        JOIN run_items ri ON ri.run_id = r.id
        WHERE r.id = $1 AND ri.id = $2
        "#,
    )
    .bind(run_uuid)
    .bind(run_item_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения run status."))?;

    let run_status = run_status.ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            "Run или run_item не найден для обновления результата.",
        )
    })?;
    if run_status == "locked" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Run в статусе locked, результаты менять нельзя.",
        ));
    }

    let updated_at: String = sqlx::query_scalar(
        r#"
        INSERT INTO run_results (run_item_id, status, fail_reason_code, comment, updated_by_user_id, updated_at)
        VALUES ($1, $2::result_status, $3, $4, $5, NOW())
        ON CONFLICT (run_item_id)
        DO UPDATE SET
          status = EXCLUDED.status,
          fail_reason_code = EXCLUDED.fail_reason_code,
          comment = EXCLUDED.comment,
          updated_by_user_id = EXCLUDED.updated_by_user_id,
          updated_at = NOW()
        RETURNING updated_at::text
        "#,
    )
    .bind(run_item_uuid)
    .bind(status)
    .bind(fail_reason_code)
    .bind(comment)
    .bind(actor_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Не удалось обновить run_result."))?;

    Ok(Json(UpdateRunResultResponse {
        ok: true,
        updated_at,
    }))
}

async fn update_run_status_v2(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateRunStatusRequest>,
) -> Result<Json<UpdateRunStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _actor_id = parse_bearer_user_id(&headers)?;
    let run_uuid = parse_uuid(&run_id, "Некорректный run_id.")?;
    let next = parse_run_status(payload.status.trim())?;

    let current: Option<String> =
        sqlx::query_scalar(r#"SELECT status::text FROM runs WHERE id = $1"#)
            .bind(run_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Ошибка чтения run status."))?;

    let current = current.ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Run не найден."))?;
    let allowed = matches!(
        (current.as_str(), next),
        ("draft", "draft")
            | ("draft", "in_progress")
            | ("in_progress", "in_progress")
            | ("in_progress", "done")
            | ("done", "done")
            | ("done", "locked")
            | ("locked", "locked")
    );
    if !allowed {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Недопустимый переход статуса run.",
        ));
    }

    match next {
        "draft" => {
            sqlx::query(r#"UPDATE runs SET status = 'draft', updated_at = NOW() WHERE id = $1"#)
                .bind(run_uuid)
                .execute(&state.db)
                .await
                .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось обновить статус run."))?;
        }
        "in_progress" => {
            sqlx::query(
                r#"
                UPDATE runs
                SET status = 'in_progress',
                    started_at = COALESCE(started_at, NOW()),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(run_uuid)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось обновить статус run."))?;
        }
        "done" => {
            sqlx::query(
                r#"
                UPDATE runs
                SET status = 'done',
                    started_at = COALESCE(started_at, NOW()),
                    finished_at = COALESCE(finished_at, NOW()),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(run_uuid)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось обновить статус run."))?;
        }
        "locked" => {
            sqlx::query(
                r#"
                UPDATE runs
                SET status = 'locked',
                    started_at = COALESCE(started_at, NOW()),
                    finished_at = COALESCE(finished_at, NOW()),
                    locked_at = NOW(),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(run_uuid)
            .execute(&state.db)
            .await
            .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Не удалось обновить статус run."))?;
        }
        _ => {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "Некорректный статус run.",
            ))
        }
    }

    let run = fetch_run_view(&state.db, run_uuid)
        .await?
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Run не найден после обновления."))?;
    Ok(Json(UpdateRunStatusResponse { run }))
}

async fn api_not_found() -> (StatusCode, Json<ErrorResponse>) {
    api_error(StatusCode::NOT_FOUND, "API endpoint не найден.")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("API_PORT").unwrap_or_else(|_| "8181".to_string());
    let repo_root = env::var("REPO_ROOT").unwrap_or_else(|_| "..".to_string());
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("failed to parse API_HOST/API_PORT")?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    let data_dir = PathBuf::from(&repo_root).join("backend").join("data");
    let state = AppState {
        users_file: data_dir.join("users.json"),
        projects_file: data_dir.join("projects.json"),
        file_lock: Arc::new(Mutex::new(())),
        db,
    };

    let frontend_dist = PathBuf::from(repo_root).join("frontend").join("dist");
    let frontend_index = frontend_dist.join("index.html");
    let static_service = ServeDir::new(frontend_dist).fallback(ServeFile::new(frontend_index));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/me", get(me))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{project_id}/members", post(add_member).get(list_members))
        .route(
            "/api/projects/{project_id}/members/{user_id}",
            patch(update_member).delete(remove_member),
        )
        .route(
            "/api/projects/{project_id}/session",
            get(get_session).put(save_session),
        )
        .route("/api/v2/runs", post(create_run_v2).get(list_runs_v2))
        .route("/api/v2/runs/{run_id}", get(get_run_details_v2))
        .route("/api/v2/runs/{run_id}/status", patch(update_run_status_v2))
        .route("/api/v2/runs/{run_id}/items", post(add_run_item_v2))
        .route(
            "/api/v2/runs/{run_id}/items/{run_item_id}/result",
            patch(update_run_result_v2),
        )
        .route("/api/{*path}", any(api_not_found))
        .fallback_service(static_service)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("uran-api listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
