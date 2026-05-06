import { type PropsWithChildren } from 'react'

import { TrafficDataContext } from '@/hooks/traffic-data-context-value'
import { useTrafficData } from '@/hooks/use-traffic-data'
import { useVerge } from '@/hooks/use-verge'
import { useVisibility } from '@/hooks/use-visibility'

/**
 * 侧栏与首页都需流量时，只保留一条 `connect_traffic` WebSocket + 一条节流链路，
 * 避免两处各调 `useTrafficData` 时重复建连、重复把样本喂进 `useTrafficMonitorEnhanced`。
 */
export const TrafficDataProvider = ({ children }: PropsWithChildren) => {
  const { verge } = useVerge()
  const pageVisible = useVisibility()
  const showCharts = verge?.traffic_graph ?? true
  const value = useTrafficData({
    enabled: showCharts && pageVisible,
    /* 关曲线时仍要 WS 给数字，但可显著拉长节流降 React 与侧栏重绘 */
    throttleMs: showCharts ? 400 : 3000,
  })

  return <TrafficDataContext value={value}>{children}</TrafficDataContext>
}
