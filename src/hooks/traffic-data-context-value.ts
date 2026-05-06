import { createContext } from 'react'

import { useTrafficData } from '@/hooks/use-traffic-data'

type TrafficDataValue = ReturnType<typeof useTrafficData>

export const TrafficDataContext = createContext<TrafficDataValue | null>(null)
