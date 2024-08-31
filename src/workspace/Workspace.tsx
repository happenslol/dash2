import { createResizeObserver } from "@solid-primitives/resize-observer"
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { createSignal, For } from "solid-js"

const current = getCurrentWebviewWindow()

export const Workspace = () => {
  let timer: number | null = null
  const [isVisible, setIsVisible] = createSignal(false)
  const [active, setActive] = createSignal(0)
  const [workspaces, setWorkspaces] = createSignal([0, 1, 2])

  current.listen("enter", async () => {
    await invoke("request_height", { height: 80 })
    setIsVisible(true)

    if (timer != null) {
      clearTimeout(timer)
      timer = null
    }
  })

  current.listen("leave", () => {
    setIsVisible(false)
    timer = setTimeout(async () => await invoke("hide_panel"), 250)
  })

  current.listen<Array<number>>("workspace-count", ev =>
    setWorkspaces(ev.payload)
  )
  current.listen<number>("active-workspace", ev => setActive(ev.payload))

  createResizeObserver(
    () => container,
    rect => invoke("request_width", { width: Math.ceil(rect.width) + 100 * 2 })
  )

  let container!: HTMLDivElement

  return (
    <div class="left-0 right-0 top-0 fixed flex justify-center select-none cursor-default">
      <div
        ref={container}
        class={clsx(
          "fixed rounded-3xl bg-stone-700 top-3 flex px-2 transition-all duration-200",
          !isVisible() && "translate-y-[-20px] opacity-0"
        )}
      >
        <For each={workspaces()}>
          {num => (
            <Indicator
              active={active() === num}
              onClick={() => invoke("set_active_workspace", { index: num })}
            />
          )}
        </For>
      </div>
    </div>
  )
}

const Indicator = (props: { active: boolean; onClick: () => void }) => (
  <div
    onClick={props.onClick}
    class="flex items-center justify-center h-[36px] w-[24px] group cursor-pointer"
  >
    <div
      classList={{
        "bg-amber-600": props.active,
        "bg-stone-500 group-hover:bg-amber-500": !props.active,
      }}
      class="w-[12px] h-[12px] rounded-full cursor-pointer transition"
    />
  </div>
)
