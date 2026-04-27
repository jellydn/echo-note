import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import "./App.css";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { HistoryView } from "./components/HistoryView";
import { MeetingDetailView } from "./components/MeetingDetailView";
import { RecordView } from "./components/RecordView";
import { SettingsView } from "./components/SettingsView";
import { SetupWizard } from "./components/SetupWizard";

type View = "record" | "history" | "settings" | "meeting-detail";

function App() {
	const [currentView, setCurrentView] = useState<View>("record");
	const [selectedMeetingId, setSelectedMeetingId] = useState<number | null>(null);
	const [isFirstLaunch, setIsFirstLaunch] = useState<boolean | null>(null);
	const [isCheckingFirstLaunch, setIsCheckingFirstLaunch] = useState(true);

	// Check if this is the first launch
	useEffect(() => {
		const checkFirstLaunch = async () => {
			try {
				const response = await invoke<{ data: boolean }>("check_first_launch_status_command");
				setIsFirstLaunch(response.data);
			} catch (err) {
				console.error("Failed to check first launch status:", err);
				// Assume not first launch if check fails
				setIsFirstLaunch(false);
			} finally {
				setIsCheckingFirstLaunch(false);
			}
		};

		checkFirstLaunch();
	}, []);

	const handleSetupComplete = () => {
		setIsFirstLaunch(false);
	};

	// Show loading state while checking first launch
	if (isCheckingFirstLaunch) {
		return (
			<div className="app-loading">
				<div className="loading-spinner" />
				<p>Loading EchoNote...</p>
			</div>
		);
	}

	// Show setup wizard on first launch
	if (isFirstLaunch) {
		return <SetupWizard onComplete={handleSetupComplete} />;
	}

	const renderView = () => {
		switch (currentView) {
			case "record":
				return (
					<ErrorBoundary section="Record">
						<RecordView
							onMeetingCreated={(meetingId) => {
								// Navigate to meeting detail view after recording/processing completes
								console.log("Meeting created:", meetingId);
								setSelectedMeetingId(meetingId);
								setCurrentView("meeting-detail");
							}}
							onNavigateToSettings={() => setCurrentView("settings")}
						/>
					</ErrorBoundary>
				);
			case "history":
				return (
					<ErrorBoundary section="History">
						<HistoryView
							onMeetingClick={(meetingId) => {
								console.log("Meeting clicked:", meetingId);
								setSelectedMeetingId(meetingId);
								setCurrentView("meeting-detail");
							}}
							onDeleteMeeting={(meetingId) => {
								console.log("Meeting deleted:", meetingId);
								// If we're viewing the deleted meeting, go back to history
								if (selectedMeetingId === meetingId) {
									setSelectedMeetingId(null);
								}
							}}
						/>
					</ErrorBoundary>
				);
			case "meeting-detail":
				if (selectedMeetingId === null) {
					setCurrentView("history");
					return null;
				}
				return (
					<ErrorBoundary section="Meeting Detail">
						<MeetingDetailView
							meetingId={selectedMeetingId}
							onBack={() => {
								setSelectedMeetingId(null);
								setCurrentView("history");
							}}
						/>
					</ErrorBoundary>
				);
			case "settings":
				return (
					<ErrorBoundary section="Settings">
						<SettingsView />
					</ErrorBoundary>
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
