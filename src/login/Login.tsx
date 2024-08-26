/* @refresh reload */
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { format } from "date-fns"
import { createMemo, createSignal, onCleanup, onMount, Show } from "solid-js"

const current = getCurrentWebviewWindow()

type BatteryState = {
  percentage: number
  psu_connected: boolean
}

export const Login = () => {
  const [isPrimary, setIsPrimary] = createSignal(false)
  const [hasBattery, setHasBattery] = createSignal(false)
  const [psuConnected, setPsuConnected] = createSignal(false)
  const [batteryPercentage, setBatteryPercentage] = createSignal(0)

  const [isCheckingPassword, setIsCheckingPassword] = createSignal(false)
  const [isCheckingFingerprint, setIsCheckingFingerprint] = createSignal(false)
  const [hasPasswordError, setHasPasswordError] = createSignal(false)
  const [hasFingerprintError, setHasFingerprintError] = createSignal(false)

  onMount(async () => {
    await invoke("window_ready")

    const state = await invoke<BatteryState | null>("get_battery_state")
    if (state != null) {
      setHasBattery(true)
      setBatteryPercentage(state.percentage)
      setPsuConnected(state.psu_connected)
    }
  })

  current.listen<boolean>("psu-connected", ev => setPsuConnected(ev.payload))
  current.listen<number>("battery-percentage", ev =>
    setBatteryPercentage(ev.payload)
  )

  current.listen<string>("password-error", () => {
    setHasPasswordError(true)
    setIsCheckingPassword(false)
    passwordField.focus()
    passwordField.select()
  })

  current.listen("scanning-fingerprint", () => {
    setIsCheckingFingerprint(true)
  })

  current.listen<string>("fingerprint-error", () => {
    setHasFingerprintError(true)
    setIsCheckingFingerprint(false)
  })

  const isLoading = createMemo(
    () => isCheckingPassword() || isCheckingFingerprint()
  )

  current.listen<boolean>("is-primary", ev => setIsPrimary(ev.payload))

  const submit = async (value: string) => {
    if (isLoading()) return
    if (passwordField.value.length === 0) return

    setIsCheckingPassword(true)
    await invoke("submit_password", { value })
  }

  let passwordField!: HTMLInputElement

  return (
    <Show when={isPrimary()}>
      <div class="w-full h-screen flex items-center justify-center cursor-default select-none">
        <div class="flex flex-col items-center justify-center gap-4">
          <img src="/profile.webp" class="rounded-full h-[100px]" />

          <h1 class="text-stone-200 text-lg font-bold">happens</h1>

          <div class="flex flex-col gap-2">
            <div class="flex gap-2 items-center relative">
              <input
                ref={passwordField}
                onKeyDown={ev => {
                  if (hasPasswordError()) setHasPasswordError(false)
                  if (hasFingerprintError()) setHasFingerprintError(false)
                  if (ev.key === "Enter") submit(passwordField.value)
                }}
                autofocus
                disabled={isLoading()}
                type="password"
                class={clsx(
                  "focus:outline-none transition w-[200px] rounded-full px-4 py-1 text-stone-200 bg-stone-700 hover:bg-stone-600 focus:bg-stone-600 border disabled:opacity-50 disabled:pointer-events-none",
                  !hasPasswordError() &&
                    "border-stone-700 focus:border-stone-500",
                  hasPasswordError() && "border-red-500 focus:border-red-400"
                )}
              />

              <button
                type="button"
                onClick={() => submit(passwordField.value)}
                disabled={isCheckingPassword()}
                class={clsx(
                  "absolute right-[-40px] rounded-full border-2 flex items-center justify-center h-[32px] w-[32px] cursor-pointer",
                  isCheckingPassword() && "opacity-50",
                  !hasFingerprintError() && "border-stone-400",
                  hasFingerprintError() && "border-red-500"
                )}
              >
                <i
                  class={clsx(
                    "text-stone-200",
                    isCheckingPassword() &&
                      "icon-[ph--circle-notch] animate-spin text-2xl",
                    isCheckingFingerprint() &&
                      "icon-[ph--fingerprint] animate-pulse text-2xl",
                    !isLoading() && "icon-[ph--arrow-right-bold] text-xl"
                  )}
                />
              </button>
            </div>
          </div>

          <div
            class={clsx(
              "text-stone-400 flex items-center gap-2 relative",
              !hasBattery() && "opacity-0"
            )}
          >
            <span class="text-sm font-bold">{batteryPercentage()}%</span>
            <i
              class={clsx(
                "text-sm icon-[fa--battery]",
                getChargeClass(batteryPercentage())
              )}
            ></i>

            <i
              class={clsx(
                "absolute -right-7",
                psuConnected() &&
                  batteryPercentage() <= 95 &&
                  "text-xl text-amber-500 icon-[ph--lightning-fill]",
                psuConnected() &&
                  batteryPercentage() > 95 &&
                  "text-lime-500 icon-[fa-solid--pepper-hot]"
              )}
            ></i>
          </div>
        </div>

        <div class="fixed bottom-6 flex flex-col gap-4">
          <PowerControls disabled={isLoading()} />
          <Clock />
        </div>
      </div>
    </Show>
  )
}

const getChargeClass = (percentage: number) => {
  if (percentage <= 5) return "icon-[fa--battery-empty] text-red-500"
  if (percentage <= 20) return "icon-[fa--battery-quarter] text-amber-600"
  if (percentage <= 50) return "icon-[fa--battery-half]"
  if (percentage <= 80) return "icon-[fa--battery-three-quarters]"
  return "icon-[fa--battery-full]"
}

type PowerIconButtonProps = {
  icon: string
  onClick?: () => void
  disabled?: boolean
}

const PowerIconButton = (props: PowerIconButtonProps) => (
  <button
    type="button"
    onClick={props.onClick}
    disabled={props.disabled}
    class="bg-stone-700 rounded-full w-12 h-12 flex items-center justify-center hover:bg-stone-900 transition cursor-pointer disabled:pointer-events-none disabled:opacity-50"
  >
    <i class={clsx("text-stone-200 text-2xl", props.icon)}></i>
  </button>
)

type PowerControlsProps = {
  disabled?: boolean
}

const PowerControls = (props: PowerControlsProps) => (
  <div class="flex items-center justify-center gap-4">
    <PowerIconButton
      icon="icon-[ph--moon-stars-bold]"
      onClick={() => invoke("suspend")}
      disabled={props.disabled}
    />

    <PowerIconButton
      icon="icon-[ph--arrow-clockwise-bold]"
      onClick={() => invoke("suspend")}
      disabled={props.disabled}
    />

    <PowerIconButton
      icon="icon-[ph--power-bold]"
      onClick={() => invoke("poweroff")}
      disabled={props.disabled}
    />
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
