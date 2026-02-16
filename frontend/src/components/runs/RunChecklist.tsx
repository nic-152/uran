import AttachFile from '@mui/icons-material/AttachFile'
import IconButton from '@mui/material/IconButton'
import List from '@mui/material/List'
import ListItem from '@mui/material/ListItem'
import ListItemText from '@mui/material/ListItemText'
import Paper from '@mui/material/Paper'
import Stack from '@mui/material/Stack'
import TextField from '@mui/material/TextField'
import ToggleButton from '@mui/material/ToggleButton'
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup'
import Typography from '@mui/material/Typography'
import StatusChip from '../common/StatusChip'
import FailReasonSelect from '../common/FailReasonSelect'

export type ChecklistStatus = 'ok' | 'fail' | 'na'

export interface ChecklistItem {
  id: string
  name: string
  level: 'L0' | 'L1' | 'L2'
  status: ChecklistStatus
  comment: string
  failReasonCode: string
}

interface RunChecklistProps {
  items: ChecklistItem[]
  token?: string
  onStatusChange: (itemId: string, status: ChecklistStatus) => void
  onCommentChange: (itemId: string, comment: string) => void
  onFailReasonChange: (itemId: string, reasonCode: string) => void
  onAttach: (itemId: string, file: File) => void
}

export default function RunChecklist({
  items,
  token,
  onStatusChange,
  onCommentChange,
  onFailReasonChange,
  onAttach,
}: RunChecklistProps) {
  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Stack spacing={1.5}>
        <Typography variant="h6" fontWeight={700}>
          Run Checklist
        </Typography>
        <List disablePadding>
          {items.map((item) => (
            <ListItem
              key={item.id}
              disableGutters
              sx={{
                py: 1.5,
                display: 'grid',
                gridTemplateColumns: { xs: '1fr', md: '1.2fr 2fr' },
                gap: 2,
                borderBottom: '1px solid',
                borderColor: 'divider',
              }}
            >
              <ListItemText
                primary={
                  <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
                    <Typography fontWeight={600}>{item.name}</Typography>
                    <StatusChip kind="level" value={item.level} />
                    <StatusChip kind="result" value={item.status} />
                  </Stack>
                }
                secondary={`ID: ${item.id}`}
              />

              <Stack spacing={1}>
                <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
                  <ToggleButtonGroup
                    exclusive
                    size="small"
                    value={item.status}
                    onChange={(_, value: ChecklistStatus | null) => {
                      if (value) onStatusChange(item.id, value)
                    }}
                  >
                    <ToggleButton value="ok">OK</ToggleButton>
                    <ToggleButton value="fail">FAIL</ToggleButton>
                    <ToggleButton value="na">NA</ToggleButton>
                  </ToggleButtonGroup>

                  <IconButton component="label" size="small" color="primary">
                    <AttachFile fontSize="small" />
                    <input
                      hidden
                      type="file"
                      onChange={(event) => {
                        const file = event.target.files?.[0]
                        if (file) onAttach(item.id, file)
                        event.target.value = ''
                      }}
                    />
                  </IconButton>
                </Stack>

                {item.status === 'fail' ? (
                  <FailReasonSelect
                    value={item.failReasonCode}
                    token={token}
                    onChange={(next) => onFailReasonChange(item.id, next)}
                  />
                ) : null}

                <TextField
                  size="small"
                  value={item.comment}
                  onChange={(event) => onCommentChange(item.id, event.target.value)}
                  label="Комментарий"
                  placeholder="Краткий комментарий по шагу"
                  multiline
                  minRows={2}
                />
              </Stack>
            </ListItem>
          ))}
        </List>
      </Stack>
    </Paper>
  )
}
