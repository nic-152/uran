import Chip from '@mui/material/Chip'

export type StatusKind = 'run' | 'result' | 'level'

interface StatusChipProps {
  kind: StatusKind
  value: string
  size?: 'small' | 'medium'
}

function normalize(value: string): string {
  return value.trim().toLowerCase()
}

export default function StatusChip({ kind, value, size = 'small' }: StatusChipProps) {
  const normalized = normalize(value)

  if (kind === 'run') {
    if (normalized === 'in_progress') return <Chip size={size} label="in_progress" color="info" variant="filled" />
    if (normalized === 'done') return <Chip size={size} label="done" color="success" variant="filled" />
    if (normalized === 'locked') return <Chip size={size} label="locked" color="warning" variant="filled" />
    return <Chip size={size} label="draft" color="default" variant="outlined" />
  }

  if (kind === 'result') {
    if (normalized === 'ok') return <Chip size={size} label="OK" color="success" variant="filled" />
    if (normalized === 'fail') return <Chip size={size} label="FAIL" color="error" variant="filled" />
    return <Chip size={size} label="NA" color="default" variant="outlined" />
  }

  if (normalized === 'l0') return <Chip size={size} label="L0" color="error" variant="filled" />
  if (normalized === 'l1') return <Chip size={size} label="L1" color="warning" variant="filled" />
  return <Chip size={size} label="L2" color="default" variant="outlined" />
}
