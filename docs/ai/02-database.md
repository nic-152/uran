# AI Context: Database

## Назначение
Текстовое описание модели данных для управляемого ручного процесса.
Источник: миграции `backend/migrations/0001_init.up.sql` и `backend/migrations/0002_controlled_manual_workflow.up.sql`.

## Что уже реализовано миграциями

### Базовые enum
- `project_role`: `owner | editor | viewer`
- `test_status`: `pending | passed | failed | maybe` (legacy results)
- `user_role`: `admin | lead | engineer | viewer`
- `run_status`: `draft | in_progress | done | locked`
- `result_status`: `ok | fail | na`
- `audit_action`: `create | update | delete | lock | unlock | status_change | assign_role | revoke_role | attach | detach`

### Legacy v1 таблицы (сохраняются для совместимости)
- `users`, `auth_refresh_tokens`
- `projects`, `project_members`
- `test_sections`, `test_cases`
- `test_runs`, `run_test_results`, `run_test_screenshots`

### Controlled workflow v2 таблицы

#### Управление доступом
- `user_roles` — глобальные роли пользователей (`admin/lead/engineer/viewer`)

#### Библиотека тестов
- `test_suites` — наборы/разделы тестов
- `testcases` — стабильная сущность кейса
- `testcase_versions` — версионированное содержимое кейса (шаги, критерии, артефакты)
- `tags`, `testcase_tags` — теги и связь m:n

#### Операционная работа
- `assets` — объект тестирования (камера/прошивка/стенд/объект)
- `run_templates`, `run_template_items` — шаблоны прогонов
- `runs` — прогон с state machine и lock-полями
- `run_items` — состав прогона, всегда со ссылкой на `testcase_version`
- `fail_reasons` — справочник причин fail
- `run_results` — результат по каждому пункту (`ok/fail/na`)
- `attachments` — файлы к прогону или к результату (без base64)

#### Аудит
- `audit_log` — actor/action/entity/before/after с контекстом проекта и прогона

## Ключевая логика связей (самое важное)
1. `run_items` ссылается на `testcase_versions`, а не на mutable `testcases`.
- Это гарантирует неизменяемость исторических прогонов.

2. `run_results` связан 1:1 с `run_items` (`UNIQUE (run_item_id)`).
- По каждому пункту прогона хранится один актуальный результат.

3. `attachments` привязывается к `run` и/или `run_result`.
- Храним метаданные файла и ключ хранения.

4. `runs.status` ограничен state machine check-constraint.
- `locked` требует заполненных `started_at`, `finished_at`, `locked_at`.

## Пример связки данных
- Есть `testcase` "RTSP reconnect".
- Для него есть версия `testcase_versions.version_number = 3`.
- При создании прогона запись в `run_items` фиксирует именно эту версию.
- `run_results` хранит `fail` + комментарий + `fail_reason_code`.
- Скриншоты/видео живут в `attachments` и ссылаются на этот результат.

## Правила для запросов и моделей
- Для отчётов строить JOIN:
  - `runs -> run_items -> testcase_versions -> testcases`
  - `run_items -> run_results`
  - при необходимости `run_results|runs -> attachments`
- Для аудита строить выборки по:
  - `entity_type/entity_id`
  - `context_project_id`
  - `context_run_id`

## Обязательные поля для управляемости
- Проект
- Asset
- Инженер
- Шаблон/набор тестов
- Статус прогона
- Итог/причина fail

## Миграционный статус по коду
- Уже реализовано: `sqlx` + PostgreSQL для v2 run workflow:
  - `POST /api/v2/runs`
  - `GET /api/v2/runs`
  - `GET /api/v2/runs/{run_id}`
  - `POST /api/v2/runs/{run_id}/items`
  - `PATCH /api/v2/runs/{run_id}/items/{run_item_id}/result`
  - `PATCH /api/v2/runs/{run_id}/status`
- Пока остаётся legacy слой (file-based) для `/api/auth/*` и `/api/projects/*` до полного перевода.
