/**
 * 从当前选中的策略名解析到实际出口节点（如 Selector 选中「自动选择」→ URLTest 的 now → 真实节点）。
 */
export function resolveLeafProxyRecord(
  startName: string,
  records: Record<string, any> | undefined,
  maxHops = 24,
): any | null {
  if (!startName || !records) return null

  let current = startName.trim()
  const seen = new Set<string>()

  for (let i = 0; i < maxHops; i++) {
    if (!current) return null
    if (seen.has(current)) {
      return records[current] ?? null
    }
    seen.add(current)

    const rec = records[current]
    if (!rec) return null

    const all = Array.isArray(rec.all) ? rec.all : []
    const nowRaw =
      typeof rec.now === 'string' && rec.now.trim().length > 0
        ? rec.now.trim()
        : ''

    const hasNestedMembers = all.length > 0
    if (!hasNestedMembers || !nowRaw) {
      return rec
    }

    current = nowRaw
  }

  return records[current] ?? null
}
