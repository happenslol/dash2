import { createResizeObserver } from "@solid-primitives/resize-observer"
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { createSignal } from "solid-js"
import { makeEventListener } from "@solid-primitives/event-listener"

const current = getCurrentWebviewWindow()

export const Bars = () => {
  let timer: number | null = null
  const [isVisible, setIsVisible] = createSignal(false)

  current.listen("enter", async () => {
    await invoke("request_height", { height: 150 })
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

  let container!: HTMLDivElement
  createResizeObserver(
    () => container,
    rect => invoke("request_width", { width: Math.ceil(rect.width) + 150 })
  )

  return (
    <div
      ref={container}
      class={clsx(
        "bottom-4 right-4 fixed bg-stone-700 rounded-2xl px-4 py-2 transition-all duration-200 flex flex-col select-none cursor-default",
        !isVisible() && "translate-y-[40px] opacity-0"
      )}
    >
      <Bar
        icon="icon-[ph--sun-dim-fill] text-2xl translate-x-[-2px]"
        color="bg-yellow-400"
      />

      <Bar icon="icon-[ph--speaker-high-fill] text-xl" color="bg-amber-600" />
    </div>
  )
}

const clamp = (value: number) => Math.min(Math.max(value, 0), 1)

type BarProps = {
  icon: string
  color: string
}

const Bar = (props: BarProps) => {
  const [percent, setPercent] = createSignal(0)

  let barRef!: HTMLDivElement

  return (
    <div class="flex items-center relative pl-8">
      <i
        class={clsx(
          "text-stone-400 absolute left-0 cursor-pointer",
          props.icon
        )}
      />

      <div
        class="cursor-pointer h-7 flex flex-col justify-center"
        onMouseDown={ev => {
          const barRect = barRef.getBoundingClientRect()
          const clickAtPercent = clamp(
            (ev.clientX - barRect.left) / barRect.width
          )
          setPercent(clickAtPercent)

          const clearMouseMove = makeEventListener(window, "mousemove", ev =>
            setPercent(clamp((ev.clientX - barRect.left) / barRect.width))
          )

          const clearMouseUp = makeEventListener(window, "mouseup", () => {
            clearMouseMove?.()
            clearMouseUp?.()
          })
        }}
      >
        <div
          ref={barRef}
          class="relative bg-stone-900 rounded-full w-80 h-2 min-w-0"
        >
          <div
            style={{ width: `${percent() * 100}%` }}
            class={clsx(
              "absolute top-0 bottom-0 left-0 rounded-full cursor-pointer absolute",
              props.color
            )}
          />
        </div>
      </div>
    </div>
  )
}
