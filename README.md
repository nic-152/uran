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

## Следующие шаги

1. Миграции БД (пользователи, проекты, роли, категории, тесты, результаты).
2. JWT auth и RBAC (`owner/editor/viewer`).
3. UI-дизайн системы (дневная/ночная темы, рабочий дашборд).
4. Загрузка скриншотов в object storage (S3/MinIO).
