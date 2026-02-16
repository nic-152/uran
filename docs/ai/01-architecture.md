# AI Context: Architecture

## Цель документа
Единый источник правды для ИИ-ассистентов по архитектуре монорепозитория `uran`.

## Текущий стек
- Frontend: React + TypeScript + Vite (`frontend/`)
- Backend: Rust + Axum (`backend/`)
- Data: PostgreSQL 16 (схема в `backend/migrations/`)
- Dev orchestration: `bin/start.sh` + `docker compose`

## Слои приложения
1. Presentation Layer (React)
- Рендерит UI тестирования IP-камер.
- Делает HTTP-запросы только к backend (`/api/...`).
- Не должен хранить критичные бизнес-данные в localStorage.

2. API Layer (Axum)
- Маршруты API: `/api/...`
- Системный маршрут: `/health`
- Раздача frontend-статики через `fallback_service`.

3. Data Layer (PostgreSQL)
- Источник правды для пользователей, проектов, тест-кейсов, прогонов и результатов.
- Контракты определяются SQL-миграциями.

## Потоки данных
1. Авторизация
- UI -> `POST /api/auth/register|login`
- Backend -> проверка/создание пользователя -> ответ с auth-данными
- UI -> `GET /api/auth/me` для валидации сессии

2. Проекты и доступ
- UI -> `/api/projects` и `/api/projects/:id/members`
- Backend проверяет роль (`owner/editor/viewer`) и возвращает данные проекта.

3. Тест-сессии
- UI загружает проектную сессию из API (`GET /api/projects/:id/session`).
- UI сохраняет изменения через API (`PUT /api/projects/:id/session`).
- Цель: отказ от localStorage как primary storage, использовать БД как источник правды.

## Правила архитектурных изменений
- Любое изменение API, модели данных или потоков обязано сопровождаться обновлением `docs/ai/` в том же коммите.
- При расхождении кода и документации приоритет у кода, но документация должна быть приведена в соответствие сразу.

## Краткая карта репозитория
- `frontend/` — UI
- `backend/` — API и бизнес-логика
- `backend/migrations/` — схема БД
- `docs/ai/` — база знаний для ИИ
