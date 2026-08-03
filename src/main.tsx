import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { installTauriMock } from "./mocks/tauriMock";

// In a plain browser (no Tauri shell) the backend IPC is missing, so the
// Record/History/Settings views would throw on every invoke(). Provide a
// stateful mock that serves realistic data. Native Tauri builds are unaffected.
installTauriMock();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<App />
	</React.StrictMode>,
);
