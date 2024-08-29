import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { createSignal } from "solid-js"
import { createClockSignal } from "../clock"

const current = getCurrentWebviewWindow()

export const Control = () => {
  let timer: number | null = null
  const [isVisible, setIsVisible] = createSignal(false)

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
      await invoke("hide_control")
    }, 500)
  })

  return (
    <div
      class={clsx(
        "left-0 right-0 fixed bottom-3 flex justify-center transition-all duration-200",
        !isVisible() && "translate-y-[40px] opacity-0"
      )}
    >
      <div class="rounded-2xl bg-stone-700 min-w-0 py-4 px-6 gap-6 flex items-center text-white">
        <Clock />

        <div class="flex gap-2 text-2xl items-center justify-center">
          <TextIcon text="19. June" icon="icon-[mdi--calendar]" />
          <TextIcon text="Sacred Heart" icon="icon-[ic--round-wifi]" />
          <TextIcon text="Pixel Buds Pro 2" icon="icon-[ic--round-bluetooth]" />
          <TextIcon text="Headphones 2" icon="icon-[ic--baseline-speaker]" />

          <SimpleIcon icon="icon-[mingcute--power-fill]" />
        </div>
      </div>
    </div>
  )
}

const SimpleIcon = (props: { icon: string }) => (
  <div class="bg-stone-800 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-stone-900 transition cursor-pointer">
    <i class={`${props.icon} w-[20px]`}></i>
  </div>
)

const TextIcon = (props: { icon: string; text: string }) => (
  <div class="px-3 py-2 bg-stone-800 rounded-full flex items-center hover:bg-stone-900 transition cursor-pointer">
    <i class={`${props.icon} ml-1`}></i>
    <span class="text-sm mx-2">{props.text}</span>
  </div>
)

const Clock = () => {
  const time = createClockSignal()
  return <div class="text-4xl text-stone-200 font-bold select-none">{time()}</div>
}
