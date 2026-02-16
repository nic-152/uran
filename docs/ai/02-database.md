# AI Context: Database

## Назначение
Текстовое описание схемы БД для корректной генерации Rust-структур, SQL-запросов и обработчиков API.
Источник схемы: `backend/migrations/0001_init.up.sql`.

## ENUM типы
- `project_role`: `owner | editor | viewer`
- `test_status`: `pending | passed | failed | maybe`

## Главные сущности и связи

### users
Пользователи системы.
- PK: `id (uuid)`
- Уникальность: `email (citext unique)`

### projects
Проекты тестирования.
- PK: `id`
- FK: `owner_user_id -> users.id`
- Содержит флаг архивирования через `archived_at`.

### project_members
RBAC в рамках проекта.
- PK: `(project_id, user_id)`
- FK: `project_id -> projects.id`
- FK: `user_id -> users.id`
- `role` использует enum `project_role`.

### test_sections
Категории тестов внутри проекта.
- FK: `project_id -> projects.id`
- Один проект -> много секций.

### test_cases
Тест-кейсы внутри секции.
- FK: `section_id -> test_sections.id`
- Одна секция -> много тест-кейсов.

### test_runs
Конкретный прогон тестирования (снимок запуска).
- FK: `project_id -> projects.id`
- FK: `created_by_user_id -> users.id`
- Один проект -> много прогонов.

### run_test_results (ключевая таблица)
Результаты кейсов в рамках конкретного прогона.
- Композитный PK: `(run_id, test_case_id)`
- FK: `run_id -> test_runs.id`
- FK: `test_case_id -> test_cases.id`
- `status` использует enum `test_status`

Это таблица-связка many-to-many между `test_runs` и `test_cases` с полезной нагрузкой (`status`, `note`, `updated_by_user_id`, `updated_at`).

### run_test_screenshots
Скриншоты, привязанные к конкретному результату теста.
- FK (композитный): `(run_id, test_case_id) -> run_test_results(run_id, test_case_id)`
- Таким образом скриншот принадлежит не просто кейсу, а кейсу в контексте конкретного прогона.

## Как читать связи run/test
Пример:
- Есть `test_case` "RTSP reconnect".
- Есть `test_run` "Firmware 2.3.1 / Camera X".
- Строка в `run_test_results` говорит, что именно в этом прогоне данный кейс имеет статус, например `failed`.
- Скриншоты и заметки должны ссылаться на эту строку результата, а не на кейс в вакууме.

## Правила для SQL и моделей
- Для выборки отчётов всегда JOIN:
  - `test_runs -> run_test_results -> test_cases`
  - при необходимости `-> run_test_screenshots`
- Не хранить бинарные изображения в БД как Base64: только метаданные и ключи хранения.
- `updated_at` поддерживается триггерами для ряда таблиц (`users`, `projects`, `project_members`, `test_sections`, `test_cases`, `test_runs`).
