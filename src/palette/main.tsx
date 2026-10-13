import React from "react";
import { createRoot } from "react-dom/client";
import { Palette } from "../components/Palette";
import "../styles.css";

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Palette />
  </React.StrictMode>,
);
