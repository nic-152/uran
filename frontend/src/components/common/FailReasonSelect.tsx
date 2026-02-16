import { useEffect, useMemo, useState } from 'react'
import FormControl from '@mui/material/FormControl'
import InputLabel from '@mui/material/InputLabel'
import MenuItem from '@mui/material/MenuItem'
import Select from '@mui/material/Select'

export interface FailReasonOption {
  code: string
  title: string
  description?: string
}

interface FailReasonSelectProps {
  value: string
  onChange: (value: string) => void
  token?: string
  disabled?: boolean
}

export default function FailReasonSelect({ value, onChange, token, disabled = false }: FailReasonSelectProps) {
  const [options, setOptions] = useState<FailReasonOption[]>([])

  useEffect(() => {
    let cancelled = false

    async function loadFailReasons() {
      try {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' }
        if (token) headers.Authorization = `Bearer ${token}`

        const response = await fetch('/api/fail-reasons', { headers })
        if (!response.ok) return

        const payload = (await response.json()) as { reasons?: FailReasonOption[] }
        if (!cancelled) setOptions(payload.reasons ?? [])
      } catch {
        if (!cancelled) setOptions([])
      }
    }

    void loadFailReasons()
    return () => {
      cancelled = true
    }
  }, [token])

  const mergedOptions = useMemo(() => {
    if (!value) return options
    const exists = options.some((opt) => opt.code === value)
    return exists ? options : [...options, { code: value, title: value }]
  }, [options, value])

  return (
    <FormControl fullWidth size="small" disabled={disabled}>
      <InputLabel id="fail-reason-label">Причина FAIL</InputLabel>
      <Select
        labelId="fail-reason-label"
        value={value}
        label="Причина FAIL"
        onChange={(event) => onChange(event.target.value)}
      >
        <MenuItem value="">
          <em>Не выбрано</em>
        </MenuItem>
        {mergedOptions.map((opt) => (
          <MenuItem key={opt.code} value={opt.code}>
            {opt.title}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  )
}
