/* @refresh reload */
import { render } from "solid-js/web";
import { App } from "./App";
import "./index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error(
    "Root element #root not found. Check that index.html includes <div id=\"root\"></div>.",
  );
}

render(() => <App />, root);
