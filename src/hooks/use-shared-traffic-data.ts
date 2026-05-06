import { use } from 'react'

import { TrafficDataContext } from '@/hooks/traffic-data-context-value'

export const useSharedTrafficData = () => {
  const ctx = use(TrafficDataContext)
  if (!ctx) {
    throw new Error(
      'useSharedTrafficData must be used under TrafficDataProvider (main layout).',
    )
  }
  return ctx
}
