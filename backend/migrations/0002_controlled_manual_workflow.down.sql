BEGIN;

DROP TRIGGER IF EXISTS trg_fail_reasons_set_updated_at ON fail_reasons;
DROP TRIGGER IF EXISTS trg_runs_set_updated_at ON runs;
DROP TRIGGER IF EXISTS trg_run_templates_set_updated_at ON run_templates;
DROP TRIGGER IF EXISTS trg_assets_set_updated_at ON assets;
DROP TRIGGER IF EXISTS trg_testcases_set_updated_at ON testcases;
DROP TRIGGER IF EXISTS trg_test_suites_set_updated_at ON test_suites;
DROP TRIGGER IF EXISTS trg_user_roles_set_updated_at ON user_roles;

DROP TABLE IF EXISTS audit_log;
DROP TABLE IF EXISTS attachments;
DROP TABLE IF EXISTS run_results;
DROP TABLE IF EXISTS fail_reasons;
DROP TABLE IF EXISTS run_items;
DROP TABLE IF EXISTS runs;
DROP TABLE IF EXISTS run_template_items;
DROP TABLE IF EXISTS run_templates;
DROP TABLE IF EXISTS assets;
DROP TABLE IF EXISTS testcase_tags;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS testcase_versions;
DROP TABLE IF EXISTS testcases;
DROP TABLE IF EXISTS test_suites;
DROP TABLE IF EXISTS user_roles;

DROP TYPE IF EXISTS audit_action;
DROP TYPE IF EXISTS result_status;
DROP TYPE IF EXISTS run_status;
DROP TYPE IF EXISTS user_role;

COMMIT;
