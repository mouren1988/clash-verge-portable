const SKIP_GLOBAL_OUTLETS = new Set([
  'DIRECT',
  'REJECT',
  'REJECT-DROP',
  'PASS',
  'COMPATIBLE',
])

/**
 * 进入全局模式时，优先选「自动选择 / URLTest」类策略，避免默认落在 DIRECT。
 */
export function pickGlobalAutoOutlet(names: string[]): string | null {
  const candidates = names.filter((n) => n && !SKIP_GLOBAL_OUTLETS.has(n))
  if (candidates.length === 0) return null
  const preferred = candidates.find((n) =>
    /自动选择|自动|自動選擇|自動|auto[-_\s]?select|^auto$|select|url[-_\s]?test|fallback/i.test(
      n,
    ),
  )
  return preferred ?? candidates[0]
}
