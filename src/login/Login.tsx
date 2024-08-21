import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import clsx from "clsx"
import { createSignal } from "solid-js"

const timeout = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms))

export const Login = () => {
  const [isLoading, setIsLoading] = createSignal(false)
  listen("test", async (arg) => console.log("got arg:", arg))

  const quit = async () => {
    setIsLoading(true)
    await timeout(3000)
    await invoke("quit")
  }

  return (
    <div class="w-full h-screen flex items-center justify-center">
      <div class="flex flex-col items-center justify-center gap-4">
        <img src="/profile.webp" class="rounded-full h-[100px]" />

        <h1 class="text-stone-200 text-lg font-bold">happens</h1>

        <div class="flex gap-2 items-center relative">
          <input
            disabled={isLoading()}
            type="password"
            class="focus:outline-none transition w-[200px] rounded-full px-4 py-1 text-stone-200 bg-stone-700 hover:bg-stone-600 focus:bg-stone-600 border border-stone-700 focus:border-stone-500"
          />

          <button
            onClick={() => quit()}
            disabled={isLoading()}
            class={clsx(
              "absolute right-[-40px] rounded-full border border-stone-400 flex items-center justify-center h-[32px] w-[32px] cursor-pointer",
              isLoading() && "opacity-50"
            )}
          >
            <i
              class={clsx(
                "text-stone-200 text-2xl",
                isLoading() && "icon-[ph--circle-notch] animate-spin",
                !isLoading() && "icon-[mdi--arrow-right]"
              )}
            />
          </button>
        </div>
        <div class="text-stone-400 flex items-center gap-2">
          <span class="text-sm font-bold">65%</span>
          <i class="text-sm icon-[fa--battery]"></i>
        </div>
      </div>

      <div class="fixed bottom-6 flex flex-col gap-4">
        <PowerControls />
        <Clock />
      </div>
    </div>
  )
}

const PowerControls = () => (
  <div class="flex items-center justify-center gap-2">
    <div class="bg-stone-700 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-stone-900 transition cursor-pointer">
      <i class="text-stone-200 icon-[material-symbols--sleep-rounded] w-[20px]"></i>
    </div>

    <div class="bg-stone-700 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-stone-900 transition cursor-pointer">
      <i class="text-stone-200 icon-[mingcute--power-fill] w-[20px]"></i>
    </div>
  </div>
)

const Clock = () => (
  <div class="text-stone-200 text-3xl text-center font-bold select-none">
    21:34
  </div>
)
