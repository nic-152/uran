BEGIN;

DROP TRIGGER IF EXISTS trg_test_runs_set_updated_at ON test_runs;
DROP TRIGGER IF EXISTS trg_test_cases_set_updated_at ON test_cases;
DROP TRIGGER IF EXISTS trg_test_sections_set_updated_at ON test_sections;
DROP TRIGGER IF EXISTS trg_project_members_set_updated_at ON project_members;
DROP TRIGGER IF EXISTS trg_projects_set_updated_at ON projects;
DROP TRIGGER IF EXISTS trg_users_set_updated_at ON users;

DROP TABLE IF EXISTS run_test_screenshots;
DROP TABLE IF EXISTS run_test_results;
DROP TABLE IF EXISTS test_runs;
DROP TABLE IF EXISTS test_cases;
DROP TABLE IF EXISTS test_sections;
DROP TABLE IF EXISTS project_members;
DROP TABLE IF EXISTS projects;
DROP TABLE IF EXISTS auth_refresh_tokens;
DROP TABLE IF EXISTS users;

DROP FUNCTION IF EXISTS set_updated_at();

DROP TYPE IF EXISTS test_status;
DROP TYPE IF EXISTS project_role;

COMMIT;
