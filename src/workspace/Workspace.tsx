import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { createSignal, For } from "solid-js"

const current = getCurrentWebviewWindow()

export const Workspace = () => {
  let timer: number | null = null
  const [isVisible, setIsVisible] = createSignal(false)
  const [active, setActive] = createSignal(0)

  current.listen("enter", () => {
    setIsVisible(true)

    if (timer != null) {
      clearTimeout(timer)
      timer = null
    }
  })

  current.listen("leave", () => {
    setIsVisible(false)
    timer = setTimeout(async () => await invoke("hide_panel"), 500)
  })

  return (
    <div
      class={clsx(
        "w-full h-screen flex items-center justify-center cursor-default select-none transition-all duration-200",
        !isVisible() && "translate-y-[-20px] opacity-0"
      )}
    >
      <div class="fixed rounded-3xl bg-stone-700 top-3 flex px-2">
        <For each={[...Array(3)]}>
          {(_, index) => (
            <Indicator
              active={active() === index()}
              onClick={() => setActive(index())}
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
