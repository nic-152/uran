BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS citext;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'project_role') THEN
    CREATE TYPE project_role AS ENUM ('owner', 'editor', 'viewer');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'test_status') THEN
    CREATE TYPE test_status AS ENUM ('pending', 'passed', 'failed', 'maybe');
  END IF;
END$$;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$;

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email CITEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL CHECK (length(trim(display_name)) BETWEEN 2 AND 120),
  password_hash TEXT NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_user_id ON auth_refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_expires_at ON auth_refresh_tokens(expires_at);

CREATE TABLE IF NOT EXISTS projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 3 AND 160),
  description TEXT NOT NULL DEFAULT '',
  owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  archived_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_projects_owner_user_id ON projects(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_projects_archived_at ON projects(archived_at);

CREATE TABLE IF NOT EXISTS project_members (
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role project_role NOT NULL,
  added_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members(user_id);
CREATE INDEX IF NOT EXISTS idx_project_members_role ON project_members(role);

CREATE TABLE IF NOT EXISTS test_sections (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 2 AND 200),
  icon TEXT NOT NULL DEFAULT '🧩',
  position INTEGER NOT NULL DEFAULT 0,
  is_archived BOOLEAN NOT NULL DEFAULT FALSE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_test_sections_project_id ON test_sections(project_id);
CREATE INDEX IF NOT EXISTS idx_test_sections_project_position ON test_sections(project_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS uq_test_sections_project_name_active
  ON test_sections(project_id, lower(name))
  WHERE is_archived = FALSE;

CREATE TABLE IF NOT EXISTS test_cases (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  section_id UUID NOT NULL REFERENCES test_sections(id) ON DELETE CASCADE,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 2 AND 240),
  details TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0,
  is_archived BOOLEAN NOT NULL DEFAULT FALSE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_test_cases_section_id ON test_cases(section_id);
CREATE INDEX IF NOT EXISTS idx_test_cases_section_position ON test_cases(section_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS uq_test_cases_section_name_active
  ON test_cases(section_id, lower(name))
  WHERE is_archived = FALSE;

CREATE TABLE IF NOT EXISTS test_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT NOT NULL DEFAULT 'New test run',
  camera_model TEXT,
  firmware_version TEXT,
  tester_name TEXT,
  test_date DATE NOT NULL DEFAULT CURRENT_DATE,
  created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_test_runs_project_id ON test_runs(project_id);
CREATE INDEX IF NOT EXISTS idx_test_runs_test_date ON test_runs(test_date);

CREATE TABLE IF NOT EXISTS run_test_results (
  run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
  test_case_id UUID NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
  status test_status NOT NULL DEFAULT 'pending',
  note TEXT NOT NULL DEFAULT '',
  updated_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (run_id, test_case_id)
);

CREATE INDEX IF NOT EXISTS idx_run_test_results_status ON run_test_results(status);

CREATE TABLE IF NOT EXISTS run_test_screenshots (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID NOT NULL,
  test_case_id UUID NOT NULL,
  storage_provider TEXT NOT NULL DEFAULT 'local',
  storage_key TEXT NOT NULL,
  file_name TEXT,
  mime_type TEXT NOT NULL,
  size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
  uploaded_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT fk_run_test_screenshots_result
    FOREIGN KEY (run_id, test_case_id)
    REFERENCES run_test_results(run_id, test_case_id)
    ON DELETE CASCADE,
  CONSTRAINT chk_run_test_screenshots_mime
    CHECK (mime_type LIKE 'image/%')
);

CREATE INDEX IF NOT EXISTS idx_run_test_screenshots_run_case ON run_test_screenshots(run_id, test_case_id);

DROP TRIGGER IF EXISTS trg_users_set_updated_at ON users;
CREATE TRIGGER trg_users_set_updated_at
BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_projects_set_updated_at ON projects;
CREATE TRIGGER trg_projects_set_updated_at
BEFORE UPDATE ON projects
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_project_members_set_updated_at ON project_members;
CREATE TRIGGER trg_project_members_set_updated_at
BEFORE UPDATE ON project_members
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_test_sections_set_updated_at ON test_sections;
CREATE TRIGGER trg_test_sections_set_updated_at
BEFORE UPDATE ON test_sections
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_test_cases_set_updated_at ON test_cases;
CREATE TRIGGER trg_test_cases_set_updated_at
BEFORE UPDATE ON test_cases
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS trg_test_runs_set_updated_at ON test_runs;
CREATE TRIGGER trg_test_runs_set_updated_at
BEFORE UPDATE ON test_runs
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

COMMIT;
