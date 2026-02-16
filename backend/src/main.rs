use anyhow::Context;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{any, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("failed to parse API_HOST/API_PORT")?;

    let data_dir = PathBuf::from(&repo_root).join("backend").join("data");
    let state = AppState {
        users_file: data_dir.join("users.json"),
        projects_file: data_dir.join("projects.json"),
        file_lock: Arc::new(Mutex::new(())),
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
