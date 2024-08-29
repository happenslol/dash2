import { format } from "date-fns"
import { createSignal, onCleanup } from "solid-js"

const getTime = () => format(new Date(), "p")

export const createClockSignal = () => {
  const [time, setTime] = createSignal(getTime())
  const interval = setInterval(() => setTime(getTime()), 1000)
  onCleanup(() => clearInterval(interval))

  return time
}
