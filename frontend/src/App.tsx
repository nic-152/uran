import { useCallback, useEffect, useMemo, useState } from 'react'
import Alert from '@mui/material/Alert'
import Box from '@mui/material/Box'
import CssBaseline from '@mui/material/CssBaseline'
import Paper from '@mui/material/Paper'
import Stack from '@mui/material/Stack'
import Typography from '@mui/material/Typography'
import { ThemeProvider, createTheme } from '@mui/material/styles'
import StatusChip from './components/common/StatusChip'
import Dashboard from './components/dashboard/Dashboard'
import AppLayout from './components/layout/AppLayout'
import type { AppSection } from './components/layout/Sidebar'
import RunChecklist, { type ChecklistItem, type ChecklistStatus } from './components/runs/RunChecklist'
import RunsTable, { type RunRow } from './components/runs/RunsTable'

interface ApiRun {
  id: string
  projectId: string
  assetId?: string
  title: string
  status: string
  executedByUserId: string
  startedAt?: string
  finishedAt?: string
  lockedAt?: string
}

interface ApiRunItem {
  id: string
  testcaseVersionId: string
  status: 'ok' | 'fail' | 'na'
  failReasonCode?: string
  comment: string
}

interface ApiRunDetails {
  run: ApiRun
  items: ApiRunItem[]
}

const theme = createTheme({
  palette: {
    mode: 'light',
    primary: { main: '#0ea5a4' },
    background: { default: '#f5f7fb' },
  },
  shape: { borderRadius: 12 },
})

function parseStoredToken(): string | undefined {
  try {
    const raw = localStorage.getItem('ipCameraAuth')
    if (!raw) return undefined
    const parsed = JSON.parse(raw) as { token?: string }
    return parsed.token || undefined
  } catch {
    return undefined
  }
}

function mapRunsToRows(runs: ApiRun[]): RunRow[] {
  return runs.map((run) => ({
    id: run.id,
    project: run.projectId,
    asset: run.assetId || '-',
    engineer: run.executedByUserId,
    status: run.status,
    progress: run.status === 'locked' ? 100 : run.status === 'done' ? 90 : run.status === 'in_progress' ? 50 : 0,
    started: run.startedAt || '-',
    finished: run.finishedAt || '-',
    locked: run.lockedAt || '-',
  }))
}

function fallbackChecklist(runId: string): ChecklistItem[] {
  return [
    { id: `${runId}-1`, name: 'RTSP stream health', level: 'L0', status: 'na', comment: '', failReasonCode: '' },
    { id: `${runId}-2`, name: 'ONVIF discovery', level: 'L1', status: 'na', comment: '', failReasonCode: '' },
    { id: `${runId}-3`, name: 'Night mode stability', level: 'L2', status: 'na', comment: '', failReasonCode: '' },
  ]
}

function formatSectionTitle(section: AppSection): { title: string; subtitle: string } {
  if (section === 'dashboard') return { title: 'Dashboard', subtitle: 'Оперативные метрики ручного тестирования' }
  if (section === 'runs') return { title: 'Runs', subtitle: 'Управление прогонами и чеклистом' }
  if (section === 'test_library') return { title: 'Test Library', subtitle: 'Библиотека тестов и версий' }
  if (section === 'templates') return { title: 'Templates', subtitle: 'Шаблоны наборов тестов' }
  if (section === 'analytics') return { title: 'Analytics', subtitle: 'Срезы по качеству и нагрузке' }
  return { title: 'Admin', subtitle: 'Администрирование и контроль' }
}

export default function App() {
  const [section, setSection] = useState<AppSection>('dashboard')
  const [token] = useState<string | undefined>(() => parseStoredToken())
  const [runs, setRuns] = useState<ApiRun[]>([])
  const [selectedRunId, setSelectedRunId] = useState<string>('')
  const [checklist, setChecklist] = useState<ChecklistItem[]>([])
  const [loadError, setLoadError] = useState<string>('')

  const loadRuns = useCallback(async () => {
    if (!token) {
      setLoadError('Нет токена авторизации. Войди через экран авторизации legacy для работы с API.')
      setRuns([])
      return
    }

    try {
      const response = await fetch('/api/v2/runs?limit=100', {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
      })

      const payload = (await response.json()) as { runs?: ApiRun[]; error?: string }
      if (!response.ok) throw new Error(payload.error || 'Не удалось загрузить runs.')

      const items = payload.runs ?? []
      setRuns(items)
      setSelectedRunId((prev) => (prev && items.some((r) => r.id === prev) ? prev : items[0]?.id || ''))
      setLoadError('')
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : 'Ошибка загрузки runs')
      setRuns([])
      setSelectedRunId('')
    }
  }, [token])

  const loadRunDetails = useCallback(async () => {
    if (!selectedRunId || !token) {
      if (selectedRunId) setChecklist(fallbackChecklist(selectedRunId))
      return
    }

    try {
      const response = await fetch(`/api/v2/runs/${selectedRunId}`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
      })
      const payload = (await response.json()) as ApiRunDetails & { error?: string }
      if (!response.ok) throw new Error(payload.error || 'Не удалось загрузить run details.')

      const mapped = (payload.items || []).map<ChecklistItem>((item, idx) => ({
        id: item.id,
        name: `Testcase ${idx + 1} (${item.testcaseVersionId.slice(0, 8)})`,
        level: idx % 3 === 0 ? 'L0' : idx % 3 === 1 ? 'L1' : 'L2',
        status: item.status,
        comment: item.comment || '',
        failReasonCode: item.failReasonCode || '',
      }))

      setChecklist(mapped.length ? mapped : fallbackChecklist(selectedRunId))
    } catch {
      setChecklist(fallbackChecklist(selectedRunId))
    }
  }, [selectedRunId, token])

  useEffect(() => {
    void loadRuns()
  }, [loadRuns])

  useEffect(() => {
    void loadRunDetails()
  }, [loadRunDetails])

  const rows = useMemo(() => mapRunsToRows(runs), [runs])

  async function updateRunResult(itemId: string, next: Partial<ChecklistItem>) {
    if (!token || !selectedRunId) return

    const target = checklist.find((item) => item.id === itemId)
    if (!target) return

    const status = (next.status || target.status) as ChecklistStatus
    const failReasonCode = next.failReasonCode ?? target.failReasonCode
    const comment = next.comment ?? target.comment

    try {
      await fetch(`/api/v2/runs/${selectedRunId}/items/${itemId}/result`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ status, failReasonCode, comment }),
      })
    } catch {
      // Silent network failure for now; UI still keeps local state.
    }
  }

  const summary = formatSectionTitle(section)

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <AppLayout
        activeSection={section}
        onSectionChange={setSection}
        title={summary.title}
        subtitle={summary.subtitle}
      >
        <Stack spacing={2}>
          {!token ? (
            <Alert severity="warning">Для работы с API нужна авторизация. Войди через legacy-экран и обнови страницу.</Alert>
          ) : null}

          {loadError ? <Alert severity="error">{loadError}</Alert> : null}

          {section === 'dashboard' ? (
            <Dashboard
              runsCompleted={runs.filter((r) => r.status === 'done' || r.status === 'locked').length}
              activeRuns={runs.filter((r) => r.status === 'in_progress').length}
              failRate={12.5}
              engineersActivity={`${new Set(runs.map((r) => r.executedByUserId)).size} active`}
            />
          ) : null}

          {section === 'runs' ? (
            <Stack spacing={2}>
              <RunsTable rows={rows} onSelectRun={setSelectedRunId} />
              {selectedRunId ? (
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Typography variant="body2" color="text.secondary">
                    Selected run:
                  </Typography>
                  <Typography variant="body2" fontWeight={700}>
                    {selectedRunId}
                  </Typography>
                  <StatusChip
                    kind="run"
                    value={runs.find((run) => run.id === selectedRunId)?.status || 'draft'}
                  />
                </Box>
              ) : null}
              <RunChecklist
                items={checklist}
                token={token}
                onStatusChange={(itemId, status) => {
                  setChecklist((prev) => prev.map((item) => (item.id === itemId ? { ...item, status } : item)))
                  void updateRunResult(itemId, { status })
                }}
                onCommentChange={(itemId, comment) => {
                  setChecklist((prev) => prev.map((item) => (item.id === itemId ? { ...item, comment } : item)))
                }}
                onFailReasonChange={(itemId, failReasonCode) => {
                  setChecklist((prev) =>
                    prev.map((item) => (item.id === itemId ? { ...item, failReasonCode, status: 'fail' } : item)),
                  )
                  void updateRunResult(itemId, { failReasonCode, status: 'fail' })
                }}
                onAttach={(itemId, file) => {
                  console.info('Attachment selected', { itemId, fileName: file.name })
                }}
              />
            </Stack>
          ) : null}

          {section !== 'dashboard' && section !== 'runs' ? (
            <Paper variant="outlined" sx={{ p: 3 }}>
              <Typography variant="h6" gutterBottom>
                {summary.title}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Раздел подготовлен в MUI layout. Следующим шагом сюда подключается предметная логика Uran.
              </Typography>
            </Paper>
          ) : null}
        </Stack>
      </AppLayout>
    </ThemeProvider>
  )
}
