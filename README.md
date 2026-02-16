# Uran

Новый репозиторий с нуля для полноценной версии системы тестирования IP-камер.

## Стек (v2)

- Frontend: React + TypeScript + Vite
- Backend: Rust + Axum
- DB: PostgreSQL

## Структура

- `frontend/` — клиентское приложение
- `backend/` — API-сервис (Rust)
- `docker-compose.yml` — локальный PostgreSQL

## Быстрый старт

### 1) Поднять PostgreSQL

```bash
docker compose up -d postgres
```

### 2) Запустить frontend

```bash
cd frontend
npm install
npm run dev
```

### 3) Запустить backend

Rust должен быть установлен (rustup/cargo).

```bash
cd backend
cp .env.example .env
cargo run
```

API health endpoint:

```text
GET http://localhost:8080/health
```

### 4) Применить SQL миграции

Вариант A, если `psql` установлен локально:

```bash
export DATABASE_URL=postgres://uran:uran_dev_password@localhost:5432/uran
psql "$DATABASE_URL" -f backend/migrations/0001_init.up.sql
```

Вариант B, через Docker (без локального `psql`):

```bash
cat backend/migrations/0001_init.up.sql | docker compose exec -T postgres psql -U uran -d uran
```

Откат миграции:

```bash
cat backend/migrations/0001_init.down.sql | docker compose exec -T postgres psql -U uran -d uran
```

## Что в схеме БД (v1)

- `users`, `auth_refresh_tokens`
- `projects`, `project_members` (RBAC: `owner/editor/viewer`)
- `test_sections`, `test_cases`
- `test_runs`, `run_test_results`, `run_test_screenshots`
- enum типы: `project_role`, `test_status`

## Следующие шаги

1. Подключить `sqlx` в backend и слой репозиториев.
2. JWT auth и RBAC (`owner/editor/viewer`).
3. API для категорий/тестов/результатов на новой схеме.
4. Загрузка скриншотов в object storage (S3/MinIO).
