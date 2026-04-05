import { useState } from "react";
import "./App.css";

type View = "record" | "history" | "settings";

function App() {
	const [currentView, setCurrentView] = useState<View>("record");

	const renderView = () => {
		switch (currentView) {
			case "record":
				return (
					<div className="view-container">
						<h2>Record</h2>
						<p>Recording interface will be implemented here.</p>
					</div>
				);
			case "history":
				return (
					<div className="view-container">
						<h2>History</h2>
						<p>Meeting history will be displayed here.</p>
					</div>
				);
			case "settings":
				return (
					<div className="view-container">
						<h2>Settings</h2>
						<p>App settings will be configured here.</p>
					</div>
				);
			default:
				return null;
		}
	};

	return (
		<div className="app-shell">
			<aside className="sidebar">
				<div className="sidebar-header">
					<h1>EchoNote</h1>
				</div>
				<nav className="sidebar-nav">
					<button
						type="button"
						className={`nav-button ${currentView === "record" ? "active" : ""}`}
						onClick={() => setCurrentView("record")}
					>
						<span className="nav-icon">🔴</span>
						<span>Record</span>
					</button>
					<button
						type="button"
						className={`nav-button ${currentView === "history" ? "active" : ""}`}
						onClick={() => setCurrentView("history")}
					>
						<span className="nav-icon">📋</span>
						<span>History</span>
					</button>
					<button
						type="button"
						className={`nav-button ${currentView === "settings" ? "active" : ""}`}
						onClick={() => setCurrentView("settings")}
					>
						<span className="nav-icon">⚙️</span>
						<span>Settings</span>
					</button>
				</nav>
			</aside>
			<main className="main-content">{renderView()}</main>
		</div>
	);
}

export default App;
