/* @refresh reload */
import { render } from "solid-js/web"
import { Workspace } from "./Workspace"

const root = document.getElementById("root")

render(() => <Workspace />, root!)
