import { invoke } from '@tauri-apps/api/core'
import { asyncRetry } from 'foxts/async-retry'
import { extractErrorMessage } from 'foxts/extract-error-message'
import { once } from 'foxts/once'

import { debugLog } from '@/utils/debug'

type PanelHttpInvokeResponse = { status: number; body: string }

/**
 * IP 检测：与 iGame+ 一致，走 Rust `panel_http_request` + rustls 直连（不经 mixed，避免核心未起或规则异常时全失败）。
 */
async function ipDetectionHttpGet(
  url: string,
  userAgent: string,
  timeoutMs: number,
): Promise<{
  ok: boolean
  status: number
  json: () => Promise<unknown>
}> {
  const c = Math.min(8_000, Math.max(1_000, timeoutMs))
  const r = Math.min(10_000, Math.max(1_500, timeoutMs + 400))
  const res = await invoke<PanelHttpInvokeResponse>('panel_http_request', {
    method: 'GET',
    url,
    body: null,
    headers: { 'User-Agent': userAgent, Accept: 'application/json' },
    connect_timeout_ms: c,
    request_timeout_ms: r,
  })
  return {
    ok: res.status >= 200 && res.status < 300,
    status: res.status,
    async json() {
      return JSON.parse(res.body) as unknown
    },
  }
}

/** 与 iGame+ 相同短 UA，减少部分 GeoIP 源对长客户端串的限流/拒绝 */
const getUserAgentPromise = once(async () => 'IGame')
// Get current IP and geolocation information （refactored IP detection with service-specific mappings）
interface IpInfo {
  ip: string
  country_code: string
  country: string
  region: string
  city: string
  organization: string
  asn: number
  asn_organization: string
  longitude: number
  latitude: number
  timezone: string
}

let _regionNameEn: Intl.DisplayNames | null | undefined
function getRegionNameEn(): Intl.DisplayNames | null {
  if (typeof Intl === 'undefined' || !('DisplayNames' in Intl)) {
    return null
  }
  if (_regionNameEn === null) {
    return null
  }
  if (_regionNameEn === undefined) {
    try {
      _regionNameEn = new Intl.DisplayNames(['en'], { type: 'region' })
    } catch {
      _regionNameEn = null
      return null
    }
  }
  return _regionNameEn
}

/** 统一 2 字母国家码与英文国名，减少各 GeoIP 接口混用时的 CN / China / 大小写 不一致 */
export function normalizeIpInfoGeo<T extends IpInfo>(m: T): T {
  const o = { ...m } as T
  const raw = String(o.country_code || '').trim()
  if (raw.length === 2) {
    o.country_code = raw.toUpperCase() as T['country_code']
  }
  const name = String(o.country || '').trim()
  if (name.length === 2) {
    const code = name.toUpperCase()
    if (!o.country_code) o.country_code = code as T['country_code']
    const d = getRegionNameEn()
    if (d) {
      try {
        const n = d.of(code)
        if (n) o.country = n as T['country']
        else o.country = code as T['country']
      } catch {
        o.country = code as T['country']
      }
    } else {
      o.country = code as T['country']
    }
  } else if (!o.country && o.country_code) {
    const d = getRegionNameEn()
    if (d) {
      try {
        const n = d.of(String(o.country_code))
        if (n) o.country = n as T['country']
        else o.country = o.country_code as T['country']
      } catch {
        o.country = o.country_code as T['country']
      }
    } else {
      o.country = o.country_code as T['country']
    }
  }
  return o
}

/**
 * 首页 IP 卡片主标题：优先用两位国码展开为英文国名（如 China），避免出现「CN」与「China」随机混用。
 */
export function formatCountryTitleForDisplay(
  ip: Pick<IpInfo, 'country' | 'country_code'> | null | undefined,
): string {
  if (!ip) return ''
  const codeRaw = String(ip.country_code || '').trim()
  if (codeRaw.length === 2) {
    const d = getRegionNameEn()
    if (d) {
      try {
        const n = d.of(codeRaw.toUpperCase())
        if (n) return n
      } catch {
        /* fall through */
      }
    }
  }
  const name = String(ip.country || '').trim()
  if (name.length === 2 && name === name.toUpperCase()) {
    const d = getRegionNameEn()
    if (d) {
      try {
        const n = d.of(name)
        if (n) return n
      } catch {
        /* fall through */
      }
    }
    return name
  }
  if (name.length > 2) {
    return name
  }
  return (codeRaw.length === 2 ? codeRaw.toUpperCase() : '') || name
}

// IP检测服务配置
interface ServiceConfig {
  url: string
  mapping: (data: any) => IpInfo
  timeout?: number // 保留timeout字段（如有需要）
}

// 首页 IP/归属地：与 clash-verge-rev 一致，将下列列表随机打乱后依次尝试（列表顺序/长度以本仓库 `api.ts` 为准）；
// 若源返回公网 IP + 地理信息，优先采用 **IPv4**；若当前轮询中始终只有 v6 则使用首次成功的 v6 结果（见 getIpInfoImpl）。
const IP_CHECK_SERVICES: ServiceConfig[] = [
  {
    url: 'https://api.ip.sb/geoip',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.organization || data.isp || '',
      asn: data.asn || 0,
      asn_organization: data.asn_organization || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://ipapi.co/json',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country_name || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.org || '',
      asn: data.asn ? parseInt(data.asn.replace('AS', '')) : 0,
      asn_organization: data.org || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://api.ipapi.is/',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.location?.country_code || '',
      country: data.location?.country || '',
      region: data.location?.state || '',
      city: data.location?.city || '',
      organization: data.asn?.org || data.company?.name || '',
      asn: data.asn?.asn || 0,
      asn_organization: data.asn?.org || '',
      longitude: data.location?.longitude || 0,
      latitude: data.location?.latitude || 0,
      timezone: data.location?.timezone || '',
    }),
  },
  {
    url: 'https://ipwho.is/',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.connection?.org || data.connection?.isp || '',
      asn: data.connection?.asn || 0,
      asn_organization: data.connection?.isp || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone?.id || '',
    }),
  },
  {
    url: 'https://ip.api.skk.moe/cf-geoip',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.asOrg || '',
      asn: data.asn || 0,
      asn_organization: data.asOrg || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://get.geojs.io/v1/ip/geo.json',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.organization_name || '',
      asn: data.asn || 0,
      asn_organization: data.organization_name || '',
      longitude: Number(data.longitude) || 0,
      latitude: Number(data.latitude) || 0,
      timezone: data.timezone || '',
    }),
  },
]

/** 双栈时优先走 IPv4；仅当拿不到公网 IPv4 时再由通用 GeoIP 回退（可接受 IPv6） */
function isIpv4Address(ip: string): boolean {
  const s = ip.trim()
  if (!s || s.includes(':')) return false
  if (!/^\d{1,3}(?:\.\d{1,3}){3}$/.test(s)) return false
  const parts = s.split('.').map((x) => Number(x))
  return (
    parts.length === 4 &&
    parts.every((n) => Number.isInteger(n) && n >= 0 && n <= 255)
  )
}

function hasUsableIp(ip: string): boolean {
  return Boolean(ip && String(ip).trim().length > 0)
}

/** 整段 IP 检测（随机多源、优先 IPv4）的硬上界，避免首页长时间无结果 */
const GET_IP_INFO_TOTAL_MS = 15_000

// 获取当前IP和地理位置信息
async function getIpInfoImpl(): Promise<IpInfo & { lastFetchTs: number }> {
  // 单服务超时与重试：偏短，失败时尽快红字，不重试以换下一个 Geo 源
  const maxRetries = 0
  const serviceTimeout = 2_500

  const userAgent = await getUserAgentPromise()
  console.debug('User-Agent for IP detection:', userAgent)

  const shuffledServices = IP_CHECK_SERVICES.toSorted(() => Math.random() - 0.5)
  let lastError: unknown | null = null
  let ipv6OnlyCandidate: (IpInfo & { lastFetchTs: number }) | null = null

  for (const service of shuffledServices) {
    debugLog(`尝试IP检测服务: ${service.url}`)

    try {
      const resolved = await asyncRetry(
        async (bail) => {
          console.debug('Fetching IP information:', service.url)

          const response = await ipDetectionHttpGet(
            service.url,
            userAgent,
            service.timeout || serviceTimeout,
          )

          if (!response.ok) {
            return bail(
              new Error(
                `IP 检测服务出错，状态码: ${response.status} from ${service.url}`,
              ),
            )
          }

          let data: any
          try {
            data = await response.json()
          } catch {
            return bail(new Error(`无法解析 JSON 响应 from ${service.url}`))
          }

          if (data && data.ip) {
            const mapped = service.mapping(data)
            if (!hasUsableIp(mapped.ip)) {
              return bail(new Error(`无效 IP from ${service.url}`))
            }
            debugLog(`IP检测成功，使用服务: ${service.url}`)
            return normalizeIpInfoGeo(
              Object.assign(mapped, {
                lastFetchTs: Date.now(),
              }) as IpInfo & { lastFetchTs: number },
            )
          } else {
            return bail(new Error(`无效的响应格式 from ${service.url}`))
          }
        },
        {
          retries: maxRetries,
          minTimeout: 300,
          maxTimeout: 800,
          randomize: true,
        },
      )
      if (isIpv4Address(resolved.ip)) {
        return resolved
      }
      if (!ipv6OnlyCandidate) {
        ipv6OnlyCandidate = resolved
      }
    } catch (error) {
      debugLog(`IP检测服务失败: ${service.url}`, error)
      lastError = error
    }
  }

  if (ipv6OnlyCandidate) {
    debugLog('多源轮询中仅出现 IPv6 结果，使用该结果')
    return ipv6OnlyCandidate
  }

  if (lastError) {
    throw new Error(
      `所有IP检测服务都失败: ${extractErrorMessage(lastError) || '未知错误'}`,
    )
  } else {
    throw new Error('没有可用的IP检测服务')
  }
}

export const getIpInfo = async (): Promise<
  IpInfo & { lastFetchTs: number }
> => {
  let timeoutId: ReturnType<typeof setTimeout> | undefined
  const timeoutPromise = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(() => {
      reject(
        new Error(
          `IP 检测超时（${GET_IP_INFO_TOTAL_MS / 1000} 秒内未完成，请检查网络或稍后重试）`,
        ),
      )
    }, GET_IP_INFO_TOTAL_MS)
  })
  try {
    return await Promise.race([getIpInfoImpl(), timeoutPromise])
  } finally {
    if (timeoutId != null) clearTimeout(timeoutId)
  }
}
