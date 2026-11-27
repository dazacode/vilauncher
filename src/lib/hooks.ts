import { useEffect, useRef, useState } from "react"

/**
 * Text the player edits, kept locally while they type and sent `delay` ms
 * after the last key. A new value from the launcher replaces it only while
 * nobody is typing.
 */
export function useDraft(value: string, commit: (v: string) => void, delay = 400) {
  const [draft, setDraft] = useState(value)
  const typing = useRef(false)
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const latest = useRef(commit)
  latest.current = commit
  useEffect(() => {
    if (!typing.current) setDraft(value)
  }, [value])
  useEffect(() => () => clearTimeout(timer.current), [])
  const change = (v: string) => {
    typing.current = true
    setDraft(v)
    clearTimeout(timer.current)
    timer.current = setTimeout(() => {
      typing.current = false
      latest.current(v)
    }, delay)
  }
  return [draft, change] as const
}
