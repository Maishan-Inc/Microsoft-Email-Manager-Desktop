import { mount } from "svelte";
import "./app.css";
import "./lib/theme.svelte"; // 在挂载前应用主题（light/dark/system）
import App from "./App.svelte";

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
