import { use } from 'react'

import { MemoryDataContext } from '@/hooks/memory-data-context-value'

export const useSharedMemoryData = () => {
  const ctx = use(MemoryDataContext)
  if (!ctx) {
    throw new Error(
      'useSharedMemoryData must be used under MemoryDataProvider (main layout).',
    )
  }
  return ctx
}
