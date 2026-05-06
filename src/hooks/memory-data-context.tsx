import { type PropsWithChildren } from 'react'

import { MemoryDataContext } from '@/hooks/memory-data-context-value'
import { useMemoryData } from '@/hooks/use-memory-data'

/**
 * 侧栏与首页「流量统计」都需内存占用时，只保留一条 `connect_memory` WebSocket。
 */
export const MemoryDataProvider = ({ children }: PropsWithChildren) => {
  const value = useMemoryData()
  return <MemoryDataContext value={value}>{children}</MemoryDataContext>
}
