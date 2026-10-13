import React from "react";
import { createRoot } from "react-dom/client";
import { Settings } from "./Settings";
import "../styles.css";

document.body.classList.add("opaque");

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Settings />
  </React.StrictMode>,
);
