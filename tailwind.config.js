import { addDynamicIconSelectors } from "@iconify/tailwind"

const iconifyPlugin = addDynamicIconSelectors()

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./*.html", "./src/**/*.(ts|tsx)"],
  theme: { extend: {} },
  plugins: [iconifyPlugin],
}
