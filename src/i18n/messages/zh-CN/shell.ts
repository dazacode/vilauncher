import type en from "../en/shell"
import type { Translation } from "../../types"

export default {
  unlocked: "已解锁新主题，可以在“设置”的“外观”中找到。",
  notifications: "通知",
  nav: {
    label: "页面",
    library: "项目库",
    settings: "设置",
  },
  search: "搜索项目库",
  list: {
    label: "项目",
    loadFailed: "无法加载项目列表。",
    noMatch: "没有与“{query}”匹配的项目。",
    noneForCard: "暂时没有适用于这类显卡的项目。",
    clearSearch: "清除搜索",
    showAllCards: "显示所有显卡",
    import: "导入仓库或构建版本",
    sections: {
      custom: "你的仓库",
      installed: "已安装",
      all: "全部项目",
    },
  },
  filter: {
    button: "筛选和排序",
    buttonOn: { other: "筛选和排序（已启用 {count} 项）" },
    showTakenDown: "显示已下架的项目",
    sort: "排序",
    tier: "层级",
    tiers: {
      all: "全部",
      tracked: "跟踪中",
      watchlist: "观察名单",
    },
    cards: "显卡",
    allCards: "全部",
    advanced: "高级：按电脑配置",
    reset: "重置筛选条件",
    specHint: "将按这些数值判断构建版本是否适合你的电脑。留空则使用启动器检测到的数值。",
    ram: "系统内存（GB）",
    vram: "显存（GB）",
  },
  row: {
    takenDown: "已下架 · 在 VIPortLive 上打开",
    keptCopy: "已下架 · 已保留已安装的副本",
    update: "有可用更新",
    imported: "导入的构建版本",
    installedByLink: "已安装 · 通过链接添加",
    byLink: "通过链接添加",
    notMeasured: "尚未测量",
    verified: "已验证",
  },
  job: {
    log: "构建日志",
    dismiss: "关闭",
  },
  window: {
    minimise: "最小化",
    maximise: "最大化",
    restore: "向下还原",
  },
  update: {
    out: "VILauncher {tag} 已发布",
    outHint: "可以立即更新，或稍后从侧边栏更新。",
    notes: "更新说明",
    downloading: "正在下载 {tag}",
    ready: "{tag} 已就绪",
    restart: "立即重启",
  },
} satisfies Translation<typeof en>
