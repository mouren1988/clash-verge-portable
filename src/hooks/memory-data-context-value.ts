import { createContext } from 'react'

import { useMemoryData } from '@/hooks/use-memory-data'

type MemoryDataValue = ReturnType<typeof useMemoryData>

export const MemoryDataContext = createContext<MemoryDataValue | null>(null)
