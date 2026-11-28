import type en from "../en/format"
import type { Translation } from "../../types"

export default {
  stage: {
    downloading: "下载中",
    building: "构建中",
    warming: "预编译着色器",
    installing_tools: "安装工具中",
    uninstalling: "卸载中",
    moving: "移动中",
    importing: "导入中",
    playing: "游戏中",
  },
  mode: {
    fast: { label: "快速", detail: "一半分辨率，再放大" },
    balanced: { label: "平衡", detail: "四分之三分辨率，再放大" },
    best: { label: "最佳画质", detail: "完整分辨率" },
  },
  output: {
    default: "游戏默认",
  },
  theme: {
    dark: "深色",
    light: "浅色",
    calm: "柔和",
    paws: "猫爪",
  },
  sort: {
    personalFps: "个人平均帧率",
    listing: "VIPortLive 顺序",
    name: "名称",
    fps: "帧率",
    stars: "星标数",
    commits: "近期提交",
  },
  compat: {
    gpu: "显卡",
    ram: "系统内存",
    vram: "显存",
    commit: "提交内存上限",
    noCommitUsage: "作者未报告峰值提交内存",
    commitUnknown: "本机未读取到提交内存上限",
    commitAbove: "高于作者报告的 {amount} 峰值提交内存，余量充足",
    commitBelow: "低于作者报告的 {amount} 峰值提交内存，且余量不足",
    madeForYours: "针对 {vendor} 显卡，与你的一致",
    madeFor: "针对 {cards} 显卡",
    noCards: "作者没有说明能在哪些显卡上运行",
    noUsage: "作者未报告用量",
    reported: "作者报告约 {amount}",
    above: "高于作者报告的 {amount}",
    close: "接近作者报告的 {amount}",
    below: "低于作者报告的 {amount}",
    headline: {
      card: "针对 {cards} 制作：在你的显卡上可能无法启动，或运行缓慢。",
      memory: "在你的电脑上可能运行缓慢：它在作者的电脑上占用了更多内存。",
      close: "与你的电脑配置相近：可能比在作者的电脑上稍慢一些。",
      good: "根据作者的报告，与你的电脑很匹配。",
      unknown: "目前信息不足，还无法与你的电脑比较。",
    },
    warning: {
      card: "针对 {cards} 显卡，而非 {vendor}",
      ram: "在作者的电脑上占用 {used} 内存；你的有 {have}",
      vram: "在作者的电脑上占用 {used} 显存；你的有 {have}",
      commit: "作者电脑上的峰值提交内存为 {used}；本机的提交内存上限为 {have}",
      close: "接近你电脑的上限：可能会稍慢一些",
    },
  },
} satisfies Translation<typeof en>
