import { mount } from "svelte";
import App from "./App.svelte";
import "./global.css";

// 幕が開き　ひとつの部屋に　灯が入る
mount(App, { target: document.getElementById("app")! });
