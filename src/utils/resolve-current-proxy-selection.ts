/** 与首页「当前节点」、useCurrentProxy 一致的主要代理组解析逻辑 */

interface ProxyGroupLite {
  name: string
  now?: string
}

export type ProxiesForSelection = {
  global: ProxyGroupLite
  groups: ProxyGroupLite[]
  records: Record<string, IProxyItem>
}

export function resolveCurrentProxySelection(
  proxies: ProxiesForSelection,
  currentMode: string,
): { currentProxy: IProxyItem | null; primaryGroupName: string | null } {
  const { global, groups, records } = proxies

  let primaryGroupName: string | null = 'GLOBAL'
  let currentName = global?.now

  if (currentMode === 'rule' && groups.length > 0) {
    const primaryKeywords = ['auto', 'select', 'proxy', '节点选择', '自动选择']
    const primaryGroup =
      groups.find((group) =>
        primaryKeywords.some((keyword) =>
          group.name.toLowerCase().includes(keyword.toLowerCase()),
        ),
      ) || groups.filter((g) => g.name !== 'GLOBAL')[0]

    if (primaryGroup) {
      primaryGroupName = primaryGroup.name
      currentName = primaryGroup.now
    }
  }

  if (!currentName) {
    return { currentProxy: null, primaryGroupName }
  }

  const currentProxy =
    records[currentName] ||
    ({
      name: currentName,
      type: 'Unknown',
      udp: false,
      xudp: false,
      tfo: false,
      mptcp: false,
      smux: false,
      history: [],
    } as IProxyItem)

  return { currentProxy, primaryGroupName }
}
