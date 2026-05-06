import { emit } from '@tauri-apps/api/event'

/** 与 IP 信息卡片 `IP_REFRESH_SECONDS` 一致 */
export const IP_INFO_AUTO_REFRESH_INTERVAL_SEC = 300

/** 与 IP 信息卡片 useQuery 的 queryKey 一致 */
export const IP_INFO_QUERY_KEY = 'cv_ip_info_cache'

export const VERGE_TEST_ALL_WINDOW_EVENT = 'verge-test-all'

export async function broadcastTestAll() {
  try {
    await emit('verge://test-all')
  } catch (e) {
    console.warn('[home] emit test-all failed', e)
  }
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(VERGE_TEST_ALL_WINDOW_EVENT))
  }
}
