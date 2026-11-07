import { mount } from "svelte";
import "./theme.css";
import App from "./App.svelte";
import { initTheme } from "./shared/theme";

initTheme(); // before mounting, so a saved dark theme never flashes light

export default mount(App, { target: document.getElementById("app")! });
