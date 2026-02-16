BEGIN;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'user_role') THEN
    CREATE TYPE user_role AS ENUM ('admin', 'lead', 'engineer', 'viewer');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'run_status') THEN
    CREATE TYPE run_status AS ENUM ('draft', 'in_progress', 'done', 'locked');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'result_status') THEN
    CREATE TYPE result_status AS ENUM ('ok', 'fail', 'na');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'audit_action') THEN
    CREATE TYPE audit_action AS ENUM (
      'create',
      'update',
      'delete',
      'lock',
      'unlock',
      'status_change',
      'assign_role',
      'revoke_role',
      'attach',
      'detach'
    );
  END IF;
END$$;

CREATE TABLE IF NOT EXISTS user_roles (
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role user_role NOT NULL,
  assigned_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, role)
);

CREATE INDEX IF NOT EXISTS idx_user_roles_role ON user_roles(role);

CREATE TABLE IF NOT EXISTS test_suites (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 2 AND 200),
  description TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0,
  is_archived BOOLEAN NOT NULL DEFAULT FALSE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (project_id, key)
);

CREATE INDEX IF NOT EXISTS idx_test_suites_project_position ON test_suites(project_id, position);

CREATE TABLE IF NOT EXISTS testcases (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  suite_id UUID NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 2 AND 240),
  is_required BOOLEAN NOT NULL DEFAULT TRUE,
  estimated_minutes INTEGER CHECK (estimated_minutes IS NULL OR estimated_minutes > 0),
  complexity SMALLINT CHECK (complexity BETWEEN 1 AND 5),
  is_archived BOOLEAN NOT NULL DEFAULT FALSE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (suite_id, key)
);

CREATE INDEX IF NOT EXISTS idx_testcases_suite_id ON testcases(suite_id);

CREATE TABLE IF NOT EXISTS testcase_versions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  testcase_id UUID NOT NULL REFERENCES testcases(id) ON DELETE CASCADE,
  version_number INTEGER NOT NULL CHECK (version_number > 0),
  summary TEXT NOT NULL DEFAULT '',
  preconditions TEXT NOT NULL DEFAULT '',
  steps_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  expected_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  typical_artifacts_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  common_mistakes_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  is_mandatory BOOLEAN NOT NULL DEFAULT TRUE,
  estimated_minutes INTEGER CHECK (estimated_minutes IS NULL OR estimated_minutes > 0),
  complexity SMALLINT CHECK (complexity BETWEEN 1 AND 5),
  change_note TEXT NOT NULL DEFAULT '',
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (testcase_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_testcase_versions_testcase_id ON testcase_versions(testcase_id);

CREATE TABLE IF NOT EXISTS tags (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name CITEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS testcase_tags (
  testcase_id UUID NOT NULL REFERENCES testcases(id) ON DELETE CASCADE,
  tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (testcase_id, tag_id)
);

CREATE TABLE IF NOT EXISTS assets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  asset_type TEXT NOT NULL DEFAULT 'camera',
  model TEXT NOT NULL DEFAULT '',
  firmware_version TEXT NOT NULL DEFAULT '',
  location_name TEXT NOT NULL DEFAULT '',
  stand_name TEXT NOT NULL DEFAULT '',
  serial_number TEXT,
  metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_assets_project_id ON assets(project_id);

CREATE TABLE IF NOT EXISTS run_templates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (project_id, key)
);

CREATE TABLE IF NOT EXISTS run_template_items (
  template_id UUID NOT NULL REFERENCES run_templates(id) ON DELETE CASCADE,
  testcase_version_id UUID NOT NULL REFERENCES testcase_versions(id) ON DELETE RESTRICT,
  position INTEGER NOT NULL DEFAULT 0,
  is_required BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (template_id, testcase_version_id)
);

CREATE INDEX IF NOT EXISTS idx_run_template_items_template_position ON run_template_items(template_id, position);

CREATE TABLE IF NOT EXISTS runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  asset_id UUID REFERENCES assets(id) ON DELETE SET NULL,
  template_id UUID REFERENCES run_templates(id) ON DELETE SET NULL,
  title TEXT NOT NULL DEFAULT 'New run',
  status run_status NOT NULL DEFAULT 'draft',
  executed_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  lead_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  started_at TIMESTAMPTZ,
  finished_at TIMESTAMPTZ,
  locked_at TIMESTAMPTZ,
  locked_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  correction_of_run_id UUID REFERENCES runs(id) ON DELETE SET NULL,
  fail_reason_code TEXT,
  fail_summary TEXT NOT NULL DEFAULT '',
  report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT chk_runs_status_timestamps
    CHECK (
      (status = 'draft') OR
      (status = 'in_progress' AND started_at IS NOT NULL) OR
      (status = 'done' AND started_at IS NOT NULL AND finished_at IS NOT NULL) OR
      (status = 'locked' AND started_at IS NOT NULL AND finished_at IS NOT NULL AND locked_at IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_runs_project_status ON runs(project_id, status);
CREATE INDEX IF NOT EXISTS idx_runs_executed_by ON runs(executed_by_user_id);
CREATE INDEX IF NOT EXISTS idx_runs_started_at ON runs(started_at);

CREATE TABLE IF NOT EXISTS run_items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
  testcase_version_id UUID NOT NULL REFERENCES testcase_versions(id) ON DELETE RESTRICT,
  position INTEGER NOT NULL DEFAULT 0,
  is_required BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (run_id, testcase_version_id)
);

CREATE INDEX IF NOT EXISTS idx_run_items_run_position ON run_items(run_id, position);

CREATE TABLE IF NOT EXISTS fail_reasons (
  code TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS run_results (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_item_id UUID NOT NULL REFERENCES run_items(id) ON DELETE CASCADE,
  status result_status NOT NULL DEFAULT 'na',
  fail_reason_code TEXT REFERENCES fail_reasons(code) ON DELETE SET NULL,
  comment TEXT NOT NULL DEFAULT '',
  measured_value TEXT,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (run_item_id)
);

CREATE INDEX IF NOT EXISTS idx_run_results_status ON run_results(status);
CREATE INDEX IF NOT EXISTS idx_run_results_fail_reason ON run_results(fail_reason_code);

CREATE TABLE IF NOT EXISTS attachments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID REFERENCES runs(id) ON DELETE CASCADE,
  run_result_id UUID REFERENCES run_results(id) ON DELETE CASCADE,
  storage_provider TEXT NOT NULL DEFAULT 'local',
  storage_key TEXT NOT NULL,
  file_name TEXT NOT NULL DEFAULT '',
  mime_type TEXT NOT NULL,
  size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
  uploaded_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT chk_attachments_scope
    CHECK (run_id IS NOT NULL OR run_result_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_attachments_run_id ON attachments(run_id);
CREATE INDEX IF NOT EXISTS idx_attachments_run_result_id ON attachments(run_result_id);

CREATE TABLE IF NOT EXISTS audit_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  action audit_action NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id UUID,
  context_project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
  context_run_id UUID REFERENCES runs(id) ON DELETE SET NULL,
  before_json JSONB,
  after_json JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log(entity_type, entity_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor_created_at ON audit_log(actor_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_log_project_created_at ON audit_log(context_project_id, created_at DESC);

-- updated_at triggers for newly mutable tables
DROP TRIGGER IF EXISTS trg_user_roles_set_updated_at ON user_roles;
CREATE TRIGGER trg_user_roles_set_updated_at
BEFORE UPDATE ON user_roles
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_test_suites_set_updated_at ON test_suites;
CREATE TRIGGER trg_test_suites_set_updated_at
BEFORE UPDATE ON test_suites
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_testcases_set_updated_at ON testcases;
CREATE TRIGGER trg_testcases_set_updated_at
BEFORE UPDATE ON testcases
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_assets_set_updated_at ON assets;
CREATE TRIGGER trg_assets_set_updated_at
BEFORE UPDATE ON assets
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_run_templates_set_updated_at ON run_templates;
CREATE TRIGGER trg_run_templates_set_updated_at
BEFORE UPDATE ON run_templates
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_runs_set_updated_at ON runs;
CREATE TRIGGER trg_runs_set_updated_at
BEFORE UPDATE ON runs
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_fail_reasons_set_updated_at ON fail_reasons;
CREATE TRIGGER trg_fail_reasons_set_updated_at
BEFORE UPDATE ON fail_reasons
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- seed default fail reasons
INSERT INTO fail_reasons (code, title, description)
VALUES
  ('firmware_bug', 'Firmware bug', 'Ошибка в прошивке устройства'),
  ('hardware_issue', 'Hardware issue', 'Аппаратная проблема устройства/стенда'),
  ('environment_issue', 'Environment issue', 'Проблема окружения/сети/инфраструктуры'),
  ('test_data_issue', 'Test data issue', 'Некорректные входные данные теста'),
  ('requirements_gap', 'Requirements gap', 'Неопределённые или противоречивые требования')
ON CONFLICT (code) DO NOTHING;

COMMIT;
