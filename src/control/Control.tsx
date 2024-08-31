import { invoke } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import clsx from "clsx"
import { createSignal, For, JSX, onCleanup, onMount } from "solid-js"
import { createClockSignal } from "../clock"
import { createResizeObserver } from "@solid-primitives/resize-observer"
import { createEventListener } from "@solid-primitives/event-listener"

const HEIGHT = 600
const DEFAULT_ACTIVE_AREA_HEIGHT = 150
const PANEL_OPEN_ACTIVE_AREA_HEIGHT = 600

const current = getCurrentWebviewWindow()

export const Control = () => {
  const [timer, setTimer] = createSignal<number | null>(null)

  let unlistenEnter: VoidFunction | null = null
  let unlistenLeave: VoidFunction | null = null

  const [isVisible, setIsVisible] = createSignal(false)
  const [openedMenu, setOpenedMenu] = createSignal<string | null>()

  const close = async () => {
    if (timer() != null) return
    if (openedMenu() != null) return

    setIsVisible(false)
    setTimer(
      setTimeout(async () => {
        await invoke("hide_panel")
        setTimer(null)
      }, 250)
    )
  }

  onMount(async () => {
    unlistenEnter = await current.listen("enter", async () => {
      if (isVisible()) return

      await invoke("request_height", { height: HEIGHT })
      setIsVisible(true)

      const maybeTimer = timer()
      if (maybeTimer != null) {
        clearTimeout(maybeTimer)
        setTimer(null)
      }
    })

    unlistenLeave = await current.listen("leave", () => close())
  })

  let container!: HTMLDivElement
  createResizeObserver(
    () => container,
    rect => invoke("request_width", { width: Math.ceil(rect.width) + 150 * 2 })
  )

  createEventListener(window, "keydown", ev => {
    if (ev.key === "Escape") {
      setOpenedMenu(null)
      close()
    }
  })

  onCleanup(async () => {
    unlistenLeave?.()
    unlistenEnter?.()
  })

  let activeArea!: HTMLDivElement

  return (
    <div class="left-0 right-0 bottom-0 fixed select-none cursor-default top-0">
      <div
        ref={activeArea}
        style={{
          height:
            openedMenu() != null
              ? `${PANEL_OPEN_ACTIVE_AREA_HEIGHT}px`
              : `${DEFAULT_ACTIVE_AREA_HEIGHT}px`,
        }}
        class="fixed left-0 right-0 bottom-0 flex justify-center"
        onMouseMove={ev => {
          const rect = activeArea.getBoundingClientRect()
          if (ev.clientY < rect.top) close()
        }}
      >
        <WifiMenu
          close={close}
          setMenuOpen={open => setOpenedMenu(open ? "wifi" : null)}
          menuOpen={openedMenu() === "wifi"}
        />

        <BluetoothMenu
          close={close}
          setMenuOpen={open => setOpenedMenu(open ? "bluetooth" : null)}
          menuOpen={openedMenu() === "bluetooth"}
        />

        <div
          ref={container}
          class={clsx(
            "fixed bottom-3 rounded-2xl bg-gray-700 min-w-0 py-4 px-6 gap-6 flex items-center text-white transition-all duration-200",
            !isVisible() && "translate-y-[40px] opacity-0"
          )}
        >
          <Clock />

          <div class="flex gap-2 text-2xl items-center justify-center">
            <MenuButton
              text="19. June"
              icon="icon-[mdi--calendar]"
              menuOpen={false}
              setMenuOpen={() => {}}
            />

            <MenuButton
              text="Sacred Heart"
              icon="icon-[ic--round-wifi]"
              setMenuOpen={open => setOpenedMenu(open ? "wifi" : null)}
              menuOpen={openedMenu() === "wifi"}
            />

            <MenuButton
              text="Pixel Buds Pro 2"
              icon="icon-[ph--bluetooth-bold]"
              setMenuOpen={open => setOpenedMenu(open ? "bluetooth" : null)}
              menuOpen={openedMenu() === "bluetooth"}
            />

            <MenuButton
              text="Headphones 2"
              icon="icon-[ic--baseline-speaker]"
              menuOpen={false}
              setMenuOpen={() => {}}
            />

            <SimpleIcon icon="icon-[mingcute--power-fill] translate-y-[-1px]" />
          </div>
        </div>
      </div>
    </div>
  )
}

type WifiMenuProps = CommonMenuProps

const WifiMenu = (props: WifiMenuProps) => (
  <Menu
    title="Wi-Fi Networks"
    close={props.close}
    setMenuOpen={props.setMenuOpen}
    menuOpen={props.menuOpen}
    icon={<i class="icon-[ic--round-wifi] text-2xl" />}
  >
    <div class="flex rounded-lg flex-col flex-1 overflow-y-auto py-2">
      <For each={[...new Array(15)]}>
        {() => (
          <div class="px-6 py-2 cursor-pointer hover:bg-gray-800/90 transition-colors flex items-center gap-2 justify-between group/item rounded-full h-14 shrink-0">
            <div class="flex items-center gap-2">
              <i class="icon-[ic--round-wifi] text-xl mr-2" />
              <span>Network</span>
            </div>

            <div class="flex items-center gap-2">
              <i class="icon-[ph--lock] text-xl" />
            </div>
          </div>
        )}
      </For>
    </div>
  </Menu>
)

type BluetoothMenuProps = CommonMenuProps

const BluetoothMenu = (props: BluetoothMenuProps) => (
  <Menu
    title="Bluetooth"
    close={props.close}
    setMenuOpen={props.setMenuOpen}
    menuOpen={props.menuOpen}
    icon={<i class="icon-[ph--bluetooth-bold] text-xl" />}
  >
    <div class="flex rounded-lg bg-gray-800/80 flex-col flex-1 overflow-y-auto py-2">
      <For each={[...new Array(5)]}>
        {() => (
          <div class="px-6 py-2 cursor-pointer hover:bg-gray-800/90 transition-colors">
            Device
          </div>
        )}
      </For>
    </div>
  </Menu>
)

type CommonMenuProps = {
  close: VoidFunction
  setMenuOpen: (open: boolean) => void
  menuOpen: boolean
}

type MenuProps = CommonMenuProps & {
  title: string
  children: JSX.Element
  icon: JSX.Element
  actions?: JSX.Element
}

const Menu = (props: MenuProps) => (
  <div
    class={clsx(
      "fixed bottom-[100px] bg-gray-700 w-[600px] h-[400px] rounded-2xl transition-all duration-200 flex flex-col text-white p-4 gap-4",
      !props.menuOpen && "translate-y-[20px] opacity-0 pointer-events-none"
    )}
  >
    <div class="flex justify-between items-center w-full">
      <div class="flex items-center gap-3 pl-2">
        {props.icon}
        <h1 class="text-xl font-bold">{props.title}</h1>
      </div>

      <div class="flex items-center gap-2">
        {props.actions}

        <div
          class="w-8 h-8 rounded-full bg-gray-800 flex items-center justify-center cursor-pointer transition-all hover:bg-gray-900"
          onClick={() => {
            props.setMenuOpen(false)
            props.close()
          }}
        >
          <i class="icon-[ph--x-bold]" />
        </div>
      </div>
    </div>

    {props.children}
  </div>
)

type SimpleIconProps = { icon: string; onClick?: VoidFunction }

const SimpleIcon = (props: SimpleIconProps) => (
  <div
    class="bg-gray-800 rounded-full w-[40px] h-[40px] flex items-center justify-center hover:bg-gray-900 transition cursor-pointer"
    onClick={props.onClick}
  >
    <i class={`${props.icon} w-[20px]`}></i>
  </div>
)

type TextIconProps = {
  icon: string
  text: string
  setMenuOpen: (open: boolean) => void
  menuOpen: boolean
}

const MenuButton = (props: TextIconProps) => (
  <div
    class="px-3 py-2 bg-gray-800 rounded-full hover:bg-gray-900 cursor-pointer whitespace-nowrap"
    onClick={() => props.setMenuOpen(!props.menuOpen)}
  >
    <div
      class={clsx(
        "flex items-center transition-all",
        props.menuOpen && "translate-y-[-10px] opacity-0"
      )}
    >
      <i class={`${props.icon}`} />
      <span class="text-sm mx-2">{props.text}</span>
    </div>
  </div>
)

const Clock = () => {
  const time = createClockSignal()
  return (
    <div class="text-4xl text-gray-200 font-bold select-none">{time()}</div>
  )
}
