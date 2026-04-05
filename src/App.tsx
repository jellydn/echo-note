import { useState } from "react";
import "./App.css";
import { HistoryView } from "./components/HistoryView";
import { RecordView } from "./components/RecordView";

type View = "record" | "history" | "settings" | "meeting-detail";

function App() {
	const [currentView, setCurrentView] = useState<View>("record");

	const renderView = () => {
		switch (currentView) {
			case "record":
				return (
					<RecordView
						onMeetingCreated={(meetingId) => {
							// Navigate to history and show the new meeting
							console.log("Meeting created:", meetingId);
							setCurrentView("history");
						}}
					/>
				);
			case "history":
				return (
					<HistoryView
						onMeetingClick={(meetingId) => {
							console.log("Meeting clicked:", meetingId);
							// Future: Navigate to meeting detail view (US-015)
						}}
						onDeleteMeeting={(meetingId) => {
							console.log("Meeting deleted:", meetingId);
						}}
					/>
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
