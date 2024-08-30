import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import { createSignal } from "solid-js"

const current = getCurrentWebviewWindow()

export const Bars = () => {
  let timer: number | null = null
  const [_isVisible, setIsVisible] = createSignal(false)

  current.listen("enter", () => {
    setIsVisible(true)

    if (timer != null) {
      clearTimeout(timer)
      timer = null
    }
  })

  current.listen("leave", () => {
    setIsVisible(false)
    timer = setTimeout(async () => {
      await invoke("hide_panel")
    }, 500)
  })

  return <div />
}
