import { createVaporApp } from "vue";
import "./App.css";
import EditorWindow from "./components/EditorWindow.vue";

createVaporApp(EditorWindow).mount("#root");
