import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { tauriArkLogApi } from "./tauri-api";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("ArkLog root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <App api={tauriArkLogApi} />
  </StrictMode>,
);
