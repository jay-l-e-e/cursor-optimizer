import { render } from "solid-js/web";

import App from "./App";
import "./app.css";

const mountPoint = document.getElementById("root");
if (mountPoint) {
  render(() => <App />, mountPoint);
}
