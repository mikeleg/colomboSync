import { mount } from "svelte";
import "@fontsource/nunito/latin-500.css";
import "@fontsource/nunito/latin-600.css";
import "@fontsource/nunito/latin-700.css";
import "@fontsource/nunito/latin-800.css";
import "./theme.css";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
