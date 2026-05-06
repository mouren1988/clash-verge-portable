import {
  getBaseConfig,
  healthcheckProxyProvider,
} from 'tauri-plugin-mihomo-api'

import { calcuProxies, getVergeConfig } from '@/services/cmds'
import delayManager from '@/services/delay'
import { IP_INFO_QUERY_KEY } from '@/services/home-connectivity-refresh'
import { showNotice } from '@/services/notice-service'
import { markUserDroveProxyToggles } from '@/services/proxy-notice-coordination'
import { queryClient } from '@/services/query-client'
import { resolveCurrentProxySelection } from '@/utils/resolve-current-proxy-selection'

/** TUN 起隧道路由略晚，过短会先到本地直连出口（CN）再变代理；单一延迟 + 一次失效，与 IP 区段对齐 */
const DELAY_MS_TUN_ON = 3000
const DELAY_MS_TUN_OFF = 400
const DELAY_MS_SYSTEM_PROXY_ON = 420
const DELAY_MS_SYSTEM_PROXY_OFF = 280

const PRESET_NO_LATENCY = new Set([
  'DIRECT',
  'REJECT',
  'REJECT-DROP',
  'PASS',
  'COMPATIBLE',
])

function isDelayUnavailable(delay: number, effectiveTimeout: number): boolean {
  if (!Number.isFinite(delay)) return true
  if (delay > 1e5) return true
  if (delay === 0) return true
  if (delay >= effectiveTimeout && delay <= 1e5) return true
  return false
}

/**
 * 开启 TUN/系统代理后，按当前选中的出口探测是否可用；与「组合刷新」无关，仅服务通知与失败判定。
 */
async function isProxyOpenUnhealthy(): Promise<boolean> {
  let proxies: Awaited<ReturnType<typeof calcuProxies>>
  let clashConfig: Awaited<ReturnType<typeof getBaseConfig>>
  let effectiveTimeout = 10_000
  try {
    const verge = await queryClient.fetchQuery({
      queryKey: ['getVergeConfig'],
      queryFn: getVergeConfig,
      staleTime: 30_000,
    })
    if (
      typeof verge?.default_latency_timeout === 'number' &&
      verge.default_latency_timeout > 0
    ) {
      effectiveTimeout = verge.default_latency_timeout
    }

    proxies = await queryClient.fetchQuery({
      queryKey: ['getProxies'],
      queryFn: calcuProxies,
      staleTime: 1500,
    })
    clashConfig = await queryClient.fetchQuery({
      queryKey: ['getClashConfig'],
      queryFn: getBaseConfig,
      staleTime: 1500,
    })
  } catch (e) {
    console.warn('[proxy-notice] probe: failed to load proxies/config', e)
    return true
  }

  const mode = clashConfig?.mode?.toLowerCase() || 'rule'
  if (mode === 'direct') return false

  const { currentProxy, primaryGroupName } = resolveCurrentProxySelection(
    proxies,
    mode,
  )
  if (!currentProxy?.name || !primaryGroupName) return true

  const name = currentProxy.name

  if (mode === 'global' && name === 'DIRECT') return true

  if (PRESET_NO_LATENCY.has(name)) return false

  const record = proxies.records[name] ?? currentProxy

  const hist = record.history
  const lastDelay =
    hist && hist.length > 0 ? hist[hist.length - 1]?.delay : undefined
  if (
    typeof lastDelay === 'number' &&
    isDelayUnavailable(lastDelay, effectiveTimeout)
  ) {
    return true
  }

  try {
    if (record.provider) {
      await healthcheckProxyProvider(record.provider)
      await queryClient.invalidateQueries({ queryKey: ['getProxies'] })
      proxies = await queryClient.fetchQuery({
        queryKey: ['getProxies'],
        queryFn: calcuProxies,
        staleTime: 1500,
      })
    }

    const { delay } = await delayManager.checkDelay(
      name,
      primaryGroupName,
      effectiveTimeout,
    )
    return isDelayUnavailable(delay, effectiveTimeout)
  } catch (e) {
    console.warn('[proxy-notice] probe: delay check failed', e)
    return true
  }
}

let noticeTimer: ReturnType<typeof window.setTimeout> | null = null
let noticeGen = 0

/**
 * 在 TUN/系统代理开关后：仅刷新 IP 信息缓存 + 成功/失败/关闭 通知；不触发网站测速、当前节点额外检测。
 */
export function scheduleProxyToggleNotices(
  proxyEnabled: boolean,
  options?: { linkKind?: 'tun' | 'system' },
) {
  if (typeof window === 'undefined') return
  markUserDroveProxyToggles()
  if (noticeTimer !== null) {
    clearTimeout(noticeTimer)
    noticeTimer = null
  }
  noticeGen += 1
  const myGen = noticeGen
  const linkKind = options?.linkKind ?? 'tun'
  const delay = proxyEnabled
    ? linkKind === 'system'
      ? DELAY_MS_SYSTEM_PROXY_ON
      : DELAY_MS_TUN_ON
    : linkKind === 'system'
      ? DELAY_MS_SYSTEM_PROXY_OFF
      : DELAY_MS_TUN_OFF

  noticeTimer = window.setTimeout(() => {
    noticeTimer = null
    if (myGen !== noticeGen) return
    void (async () => {
      const runInvalidate = () =>
        queryClient.invalidateQueries({ queryKey: [IP_INFO_QUERY_KEY] })
      if (proxyEnabled && linkKind === 'tun') {
        // 等隧道就绪后再查出口（外层 delay 已拉长）；直接 refetch 拉齐缓存与真实出口
        await queryClient.refetchQueries({ queryKey: [IP_INFO_QUERY_KEY] })
      } else {
        await runInvalidate()
        if (proxyEnabled) {
          await new Promise((r) => setTimeout(r, 900))
          if (myGen !== noticeGen) return
          await runInvalidate()
        }
      }
      if (myGen !== noticeGen) return
      if (!proxyEnabled) {
        showNotice.success(
          'home.components.ipInfo.proxyStateToast.successClose',
          2800,
        )
        return
      }
      const unhealthy = await isProxyOpenUnhealthy()
      if (myGen !== noticeGen) return
      if (unhealthy) {
        showNotice.error(
          'home.components.ipInfo.proxyStateToast.failOpen',
          2800,
        )
      } else {
        showNotice.success(
          'home.components.ipInfo.proxyStateToast.successOpen',
          2800,
        )
      }
    })()
  }, delay)
}
