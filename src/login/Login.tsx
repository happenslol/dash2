import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { format } from "date-fns"
import { createSignal, onCleanup, Show } from "solid-js"

const current = getCurrentWebviewWindow()

export const Login = () => {
  const [isLoading, setIsLoading] = createSignal(false)
  const [hasBattery, setHasBattery] = createSignal(false)
  const [psuConnected, setPsuConnected] = createSignal(false)
  const [batteryPercentage, setBatteryPercentage] = createSignal(0)

  const [authError, setAuthError] = createSignal<string | null>(null)

  current.listen("has-battery", () => setHasBattery(true))
  current.listen<number>("battery-percentage", ev =>
    setBatteryPercentage(ev.payload)
  )
  current.listen<boolean>("psu-connected", ev => setPsuConnected(ev.payload))

  current.listen<string>("auth-error", ev => {
    setAuthError(ev.payload)
    setIsLoading(false)
  })

  current.emit("ready")

  const submit = async (value: string) => {
    setIsLoading(true)
    await invoke("submit_password", { value })
  }

  let passwordField!: HTMLInputElement

  return (
    <div class="w-full h-screen flex items-center justify-center">
      <div class="flex flex-col items-center justify-center gap-4">
        <img src="/profile.webp" class="rounded-full h-[100px]" />

        <h1 class="text-stone-200 text-lg font-bold">happens</h1>

        <div class="flex gap-2 items-center relative">
          <input
            ref={passwordField}
            onKeyDown={ev => {
              if (ev.key === "Enter") submit(passwordField.value)
            }}
            autofocus
            disabled={isLoading()}
            type="password"
            class="focus:outline-none transition w-[200px] rounded-full px-4 py-1 text-stone-200 bg-stone-700 hover:bg-stone-600 focus:bg-stone-600 border border-stone-700 focus:border-stone-500 disabled:opacity-50 disabled:pointer-events-none"
          />

          <button
            onClick={() => submit(passwordField.value)}
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

        <Show when={authError != null}>
          <div class="text-red-600">{authError()}</div>
        </Show>

        <Show when={hasBattery()}>
          <div class="text-stone-400 flex items-center gap-2">
            <span class="text-sm font-bold">{batteryPercentage()}%</span>
            <i class="text-sm icon-[fa--battery]"></i>

            <Show when={psuConnected()}>
              <span>psu connected</span>
            </Show>
          </div>
        </Show>
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
    <button class="bg-stone-700 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-stone-900 transition cursor-pointer">
      <i class="text-stone-200 icon-[material-symbols--sleep-rounded] w-[20px]"></i>
    </button>

    <button
      onClick={() => invoke("quit")}
      class="bg-stone-700 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-stone-900 transition cursor-pointer"
    >
      <i class="text-stone-200 icon-[mingcute--power-fill] w-[20px]"></i>
    </button>
  </div>
)

const getTime = () => format(new Date(), "p")

const Clock = () => {
  const [time, setTime] = createSignal(getTime())
  const interval = setInterval(() => setTime(getTime()), 1000)
  onCleanup(() => clearInterval(interval))

  return (
    <div class="text-stone-200 text-3xl text-center font-bold select-none">
      {time()}
    </div>
  )
}
