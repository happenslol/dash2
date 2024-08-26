import { addDynamicIconSelectors } from "@iconify/tailwind"

const iconifyPlugin = addDynamicIconSelectors()

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.(ts|tsx|html)"],
  theme: { extend: {} },
  plugins: [iconifyPlugin],
}
