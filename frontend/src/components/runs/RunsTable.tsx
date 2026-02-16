import { useMemo, useState } from 'react'
import { DataGrid, type GridColDef } from '@mui/x-data-grid'
import Box from '@mui/material/Box'
import TextField from '@mui/material/TextField'
import Paper from '@mui/material/Paper'
import Stack from '@mui/material/Stack'
import Typography from '@mui/material/Typography'
import StatusChip from '../common/StatusChip'

export interface RunRow {
  id: string
  project: string
  asset: string
  engineer: string
  status: string
  progress: number
  started: string
  finished?: string
  locked?: string
}

interface RunsTableProps {
  rows: RunRow[]
  onSelectRun?: (runId: string) => void
}

export default function RunsTable({ rows, onSelectRun }: RunsTableProps) {
  const [query, setQuery] = useState('')

  const filteredRows = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return rows
    return rows.filter((row) => {
      return (
        row.id.toLowerCase().includes(q) ||
        row.project.toLowerCase().includes(q) ||
        row.asset.toLowerCase().includes(q) ||
        row.engineer.toLowerCase().includes(q) ||
        row.status.toLowerCase().includes(q)
      )
    })
  }, [rows, query])

  const columns = useMemo<GridColDef<RunRow>[]>(
    () => [
      { field: 'id', headerName: 'ID', flex: 1.2, minWidth: 140 },
      { field: 'project', headerName: 'Project', flex: 1.1, minWidth: 140 },
      { field: 'asset', headerName: 'Asset', flex: 1.2, minWidth: 150 },
      { field: 'engineer', headerName: 'Engineer', flex: 1, minWidth: 130 },
      {
        field: 'status',
        headerName: 'Status',
        flex: 0.9,
        minWidth: 130,
        renderCell: (params) => <StatusChip kind="run" value={String(params.value ?? 'draft')} />,
      },
      {
        field: 'progress',
        headerName: 'Progress',
        type: 'number',
        flex: 0.8,
        minWidth: 110,
        valueFormatter: (value) => `${value}%`,
      },
      { field: 'started', headerName: 'Started', flex: 1, minWidth: 150 },
      { field: 'finished', headerName: 'Finished', flex: 1, minWidth: 150 },
      { field: 'locked', headerName: 'Locked', flex: 1, minWidth: 150 },
    ],
    [],
  )

  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={2}>
        <Typography variant="h6" fontWeight={700}>
          Runs
        </Typography>
        <TextField
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          label="Поиск по runs"
          placeholder="ID / Project / Asset / Engineer / Status"
          size="small"
        />
        <Box sx={{ width: '100%' }}>
          <DataGrid
            rows={filteredRows}
            columns={columns}
            autoHeight
            disableRowSelectionOnClick
            pageSizeOptions={[10, 25, 50]}
            initialState={{
              pagination: { paginationModel: { pageSize: 10, page: 0 } },
            }}
            onRowClick={(params) => {
              if (onSelectRun) onSelectRun(String(params.id))
            }}
          />
        </Box>
      </Stack>
    </Paper>
  )
}
