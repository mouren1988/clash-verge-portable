import { useIsFetching, useQuery } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import { useVerge } from '@/hooks/use-verge'
import {
  useClashConfigData,
  useCoreDataStatus,
  useSystemData,
} from '@/providers/app-data-context'
import { getAutotemProxy } from '@/services/cmds'
import { queryClient } from '@/services/query-client'

/** 与后端 sysopt 一致取 mixed 端口；未设 verge 时依赖 Clash */
function expectedMixedPort(
  verge_mixed: number | undefined,
  mixedFromClash: number | undefined,
): number | null {
  if (verge_mixed != null) return Number(verge_mixed)
  if (mixedFromClash != null) return Number(mixedFromClash)
  return null
}

const normalizeHost = (h: string) => {
  const t = h.trim()
  if (t === 'localhost') return '127.0.0.1'
  return t
}

function systemServerMatches(
  systemServer: string | undefined,
  expectHost: string,
  expectPort: number,
): boolean {
  if (!systemServer) return false
  const last = systemServer.lastIndexOf(':')
  if (last <= 0) return false
  const p = Number.parseInt(systemServer.slice(last + 1), 10)
  if (Number.isNaN(p)) return false
  const h = systemServer.slice(0, last)
  return normalizeHost(h) === normalizeHost(expectHost) && p === expectPort
}

export const useSystemProxyState = () => {
  const { verge, mutateVerge, patchVerge } = useVerge()
  const { sysproxy } = useSystemData()
  const { clashConfig } = useClashConfigData()
  const { isCoreDataPending } = useCoreDataStatus()
  const sysProxyReadFetching =
    useIsFetching({ queryKey: ['getSystemProxy'] }) > 0
  const autoProxyReadFetching =
    useIsFetching({ queryKey: ['getAutotemProxy'] }) > 0
  const { data: autoproxy } = useQuery({
    queryKey: ['getAutotemProxy'],
    queryFn: getAutotemProxy,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
  })

  const {
    enable_system_proxy,
    proxy_auto_config,
    proxy_host,
    verge_mixed_port,
  } = verge ?? {}

  const vergeRef = useRef(verge)
  useEffect(() => {
    vergeRef.current = verge
  }, [verge])

  const indicator = (() => {
    const host = proxy_host || '127.0.0.1'
    if (proxy_auto_config) {
      if (!autoproxy?.enable) {
        if (
          enable_system_proxy &&
          (autoProxyReadFetching || sysProxyReadFetching)
        ) {
          return true
        }
        return false
      }
      const pacPort = import.meta.env.DEV ? 11233 : 33331
      return autoproxy.url === `http://${host}:${pacPort}/commands/pac`
    } else {
      if (!sysproxy?.enable) {
        if (
          enable_system_proxy &&
          (sysProxyReadFetching || autoProxyReadFetching)
        ) {
          return true
        }
        return false
      }
      const expectPort = expectedMixedPort(
        verge_mixed_port,
        clashConfig?.mixedPort,
      )
      if (expectPort == null) {
        if (systemServerMatches(sysproxy.server, host, 7897)) return true
        if (sysproxy?.enable && sysproxy.server) {
          const last = sysproxy.server.lastIndexOf(':')
          if (last > 0) {
            const h = sysproxy.server.slice(0, last)
            if (normalizeHost(h) === normalizeHost(host)) return true
          }
        }
        if (isCoreDataPending && enable_system_proxy) return true
        return false
      }
      return systemServerMatches(sysproxy.server, host, expectPort)
    }
  })()

  const pendingRef = useRef<boolean | null>(null)
  const busyRef = useRef(false)

  const toggleSystemProxy = async (enabled: boolean) => {
    mutateVerge(
      (prev) => (prev ? { ...prev, enable_system_proxy: enabled } : prev),
      false,
    )
    pendingRef.current = enabled

    if (busyRef.current) return
    busyRef.current = true

    try {
      while (pendingRef.current !== null) {
        const target = pendingRef.current
        pendingRef.current = null
        if (!target && vergeRef.current?.auto_close_connection) {
          await closeAllConnections().catch(() => {})
        }
        await patchVerge({ enable_system_proxy: target })
      }
    } finally {
      busyRef.current = false
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['getSystemProxy'] }),
        queryClient.invalidateQueries({ queryKey: ['getAutotemProxy'] }),
      ])
    }
  }

  const invalidateProxyState = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ['getSystemProxy'] }),
      queryClient.invalidateQueries({ queryKey: ['getAutotemProxy'] }),
    ])

  return {
    indicator,
    configState: enable_system_proxy ?? false,
    toggleSystemProxy,
    invalidateProxyState,
  }
}
