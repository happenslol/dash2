import { createSignal, onCleanup, onMount, Show } from "solid-js"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import { invoke } from "@tauri-apps/api/core"
import { UnlistenFn } from "@tauri-apps/api/event"

const current = getCurrentWebviewWindow()

export const Desktop = () => {
  const [isPrimary, setIsPrimary] = createSignal(false)
  const unlisten: Array<UnlistenFn> = []

  onMount(async () => {
    unlisten.push(
      await current.listen<[number, number]>("enter", async ev => {
        const [x, y] = ev.payload
        console.log("enter at", x, y)
      })
    )

    unlisten.push(
      await current.listen<[number, number]>("leave", async ev => {
        const [x, y] = ev.payload
        console.log("leave at", x, y)
      })
    )

    unlisten.push(
      await current.listen<boolean>("is-primary", async ev => {
        setIsPrimary(ev.payload)

        if (ev.payload) {
          const rect = container.getBoundingClientRect()

          const bottomEdge = window.innerHeight - 10
          const width = window.innerWidth

          const regions = [
            { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
            { x: 0, y: bottomEdge, width, height: 10 },
          ]
          console.log("requesting regions", regions)

          await invoke("request_input_regions", { regions })
        } else {
          await invoke("request_input_regions", { regions: [] })
        }
      })
    )

    await invoke("window_ready")
  })

  onCleanup(() => {
    unlisten.forEach(unlisten => unlisten())
  })

  let container!: HTMLDivElement

  return (
    <Show when={isPrimary()}>
      <div class="fixed flex items-center justify-center bottom-0 top-0 left-0 right-0">
        <div
          ref={container}
          class="absolute w-[300px] h-[300px] bottom-0 bg-stone-500/30"
        >
          hello world
        </div>
      </div>
    </Show>
  )
}
