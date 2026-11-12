import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { SettingsProvider } from "./lib/settings";
import "./index.css";

const root = document.getElementById("root");
if (root === null) throw new Error("index.html has no #root");
createRoot(root).render(
  <StrictMode>
    <SettingsProvider>
      <App />
    </SettingsProvider>
  </StrictMode>,
);
