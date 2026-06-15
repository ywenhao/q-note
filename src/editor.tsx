import React from "react";
import ReactDOM from "react-dom/client";
import "./App.css";
import { EditorWindow } from "./components/EditorWindow";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <EditorWindow />
  </React.StrictMode>,
);
