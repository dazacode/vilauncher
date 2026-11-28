import type en from "../en/format"
import type { Translation } from "../../types"

export default {
  stage: {
    downloading: "下載中",
    building: "建置中",
    warming: "預先編譯著色器",
    installing_tools: "安裝工具中",
    uninstalling: "解除安裝中",
    moving: "搬移中",
    importing: "匯入中",
    playing: "遊戲中",
  },
  mode: {
    fast: { label: "快速", detail: "一半解析度，再放大" },
    balanced: { label: "平衡", detail: "四分之三解析度，再放大" },
    best: { label: "最佳畫質", detail: "完整解析度" },
  },
  output: {
    default: "遊戲預設",
  },
  theme: {
    dark: "深色",
    light: "淺色",
    calm: "柔和",
    paws: "貓掌",
  },
  sort: {
    personalFps: "個人平均幀率",
    listing: "VIPortLive 順序",
    name: "名稱",
    fps: "影格率",
    stars: "星數",
    commits: "近期提交",
  },
  compat: {
    gpu: "顯示卡",
    ram: "系統記憶體",
    vram: "顯示記憶體",
    madeForYours: "針對 {vendor} 顯示卡，和你的一樣",
    madeFor: "針對 {cards} 顯示卡",
    noCards: "作者沒有說明能在哪些顯示卡上執行",
    noUsage: "作者未回報用量",
    reported: "作者回報約 {amount}",
    above: "高於作者回報的 {amount}",
    close: "接近作者回報的 {amount}",
    below: "低於作者回報的 {amount}",
    headline: {
      card: "針對 {cards} 製作：在你的顯示卡上可能無法啟動，或執行緩慢。",
      memory: "在你的電腦上可能執行緩慢：它在作者的電腦上用了更多記憶體。",
      close: "和你的電腦配置相近：可能比在作者的電腦上稍慢一些。",
      good: "根據作者的回報，和你的電腦很相配。",
      unknown: "目前資訊不足，還無法和你的電腦比較。",
    },
    warning: {
      card: "針對 {cards} 顯示卡，而非 {vendor}",
      ram: "在作者的電腦上用了 {used} 記憶體；你的有 {have}",
      vram: "在作者的電腦上用了 {used} 顯示記憶體；你的有 {have}",
      close: "接近你電腦的上限：可能會稍慢一些",
    },
  },
} satisfies Translation<typeof en>
