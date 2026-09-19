import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { applyTheme, readTheme } from "./lib/theme";
import "./index.css";

// Apply persisted theme before first paint to avoid a flash.
applyTheme(readTheme());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
