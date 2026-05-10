import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

interface SetupWizardProps {
	onComplete: () => void;
}

type SetupStep = "welcome" | "microphone" | "blackhole" | "complete";

interface BlackHoleStatus {
	installed: boolean;
	device_name: string | null;
}

interface BlackHoleInstallResult {
	success: boolean;
	method: string;
	message: string;
}

export function SetupWizard({ onComplete }: SetupWizardProps) {
	const [currentStep, setCurrentStep] = useState<SetupStep>("welcome");
	const [blackHoleStatus, setBlackHoleStatus] = useState<BlackHoleStatus | null>(null);
	const [isInstalling, setIsInstalling] = useState(false);
	const [installResult, setInstallResult] = useState<BlackHoleInstallResult | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [isLoading, setIsLoading] = useState(true);

	const checkBlackHoleStatus = useCallback(async () => {
		try {
			setIsLoading(true);
			const response = await invoke<{ data: BlackHoleStatus }>("check_blackhole_status_command");
			setBlackHoleStatus(response.data);
			setError(null);
		} catch (err) {
			console.error("Failed to check BlackHole status:", err);
			setError("Failed to check system audio status");
		} finally {
			setIsLoading(false);
		}
	}, []);

	// Check BlackHole status on mount
	useEffect(() => {
		checkBlackHoleStatus();
	}, [checkBlackHoleStatus]);

	const handleAutoInstall = async () => {
		try {
			setIsInstalling(true);
			setError(null);
			const response = await invoke<{ data: BlackHoleInstallResult }>(
				"auto_install_blackhole_command",
			);
			setInstallResult(response.data);

			// The Tauri command is already async and waits for installation to complete,
			// so we can recheck status and transition immediately.
			await checkBlackHoleStatus();
			if (response.data.success || response.data.method === "manual") {
				setCurrentStep("complete");
			}
		} catch (err) {
			console.error("Auto-install failed:", err);
			setError(`Installation failed: ${err}`);
		} finally {
			setIsInstalling(false);
		}
	};

	const handleComplete = async () => {
		try {
			await invoke("complete_first_launch_setup_command");
			onComplete();
		} catch (err) {
			console.error("Failed to complete setup:", err);
			// Complete anyway - user can finish setup
			onComplete();
		}
	};

	const handleSkip = () => {
		setCurrentStep("complete");
	};

	const renderWelcomeStep = () => (
		<div className="setup-step">
			<div className="setup-icon">👋</div>
			<h2>Welcome to EchoNote</h2>
			<p className="setup-description">
				EchoNote is a privacy-first meeting assistant that records, transcribes, and summarizes your
				meetings locally on your Mac.
			</p>
			<div className="setup-features">
				<div className="feature-item">
					<span className="feature-icon">🔒</span>
					<span>All processing happens locally on your device</span>
				</div>
				<div className="feature-item">
					<span className="feature-icon">🤖</span>
					<span>AI-powered transcription and summarization</span>
				</div>
				<div className="feature-item">
					<span className="feature-icon">🎙️</span>
					<span>Record both microphone and system audio</span>
				</div>
			</div>
			<div className="setup-actions">
				<button
					type="button"
					className="setup-button primary"
					onClick={() => setCurrentStep("microphone")}
				>
					Get Started
				</button>
			</div>
		</div>
	);

	const renderMicrophoneStep = () => (
		<div className="setup-step">
			<div className="setup-icon">🎙️</div>
			<h2>Microphone Access</h2>
			<p className="setup-description">
				EchoNote needs access to your microphone to record meetings. You will be prompted to grant
				permission when you first start recording.
			</p>
			<div className="setup-info-box">
				<h4>macOS Privacy Settings</h4>
				<p>If you don&apos;t see a permission prompt, you may need to manually grant access in:</p>
				<code>System Settings → Privacy & Security → Microphone</code>
			</div>
			<div className="setup-actions">
				<button type="button" className="setup-button secondary" onClick={handleSkip}>
					Skip
				</button>
				<button
					type="button"
					className="setup-button primary"
					onClick={() => setCurrentStep("blackhole")}
				>
					Continue
				</button>
			</div>
		</div>
	);

	const renderBlackHoleStep = () => {
		if (isLoading) {
			return (
				<div className="setup-step">
					<div className="setup-loading">Checking system audio...</div>
				</div>
			);
		}

		if (blackHoleStatus?.installed) {
			return (
				<div className="setup-step">
					<div className="setup-icon success">✓</div>
					<h2>System Audio Ready</h2>
					<p className="setup-description">
						BlackHole is already installed! EchoNote can now capture both your microphone and system
						audio (meeting participants) for complete meeting recordings.
					</p>
					{blackHoleStatus.device_name && (
						<p className="setup-device-name">Detected: {blackHoleStatus.device_name}</p>
					)}
					<div className="setup-actions">
						<button
							type="button"
							className="setup-button primary"
							onClick={() => setCurrentStep("complete")}
						>
							Continue
						</button>
					</div>
				</div>
			);
		}

		return (
			<div className="setup-step">
				<div className="setup-icon">🔊</div>
				<h2>Enable System Audio Capture</h2>
				<p className="setup-description">
					To capture meeting participants&apos; audio (e.g., from Zoom, Teams, or browser), EchoNote
					uses BlackHole — a virtual audio driver that creates a virtual speaker your Mac can record
					from.
				</p>

				<div className="setup-info-box warning">
					<p>
						<strong>Without BlackHole:</strong> Only your microphone will be recorded. You
						won&apos;t hear other meeting participants in the recording.
					</p>
				</div>

				{error && <div className="setup-error">{error}</div>}

				{installResult && (
					<div className={`setup-result ${installResult.success ? "success" : "info"}`}>
						<p>{installResult.message}</p>
						{installResult.method === "manual" && (
							<p className="setup-hint">
								Please download and install BlackHole, then restart EchoNote.
							</p>
						)}
						{installResult.method === "homebrew" && (
							<p className="setup-hint">
								Installation is running in Terminal. You may need to enter your password.
							</p>
						)}
					</div>
				)}

				<div className="setup-actions">
					<button type="button" className="setup-button secondary" onClick={handleSkip}>
						Skip (Mic Only)
					</button>
					<button
						type="button"
						className="setup-button primary"
						onClick={handleAutoInstall}
						disabled={isInstalling}
					>
						{isInstalling ? "Installing..." : "Install BlackHole"}
					</button>
				</div>

				<p className="setup-footer-hint">
					You can always install BlackHole later from Settings → System Audio Capture
				</p>
			</div>
		);
	};

	const renderCompleteStep = () => (
		<div className="setup-step">
			<div className="setup-icon success">🎉</div>
			<h2>You&apos;re All Set!</h2>
			<p className="setup-description">
				EchoNote is ready to record your meetings. Click the button below to start using the app.
			</p>
			<div className="setup-quick-tips">
				<h4>Quick Tips:</h4>
				<ul>
					<li>Click the 🔴 Record tab to start a new meeting recording</li>
					<li>View your meeting history in the 📋 History tab</li>
					<li>Configure audio devices and models in ⚙️ Settings</li>
				</ul>
			</div>
			<div className="setup-actions">
				<button type="button" className="setup-button primary" onClick={handleComplete}>
					Start Using EchoNote
				</button>
			</div>
		</div>
	);

	const renderCurrentStep = () => {
		switch (currentStep) {
			case "welcome":
				return renderWelcomeStep();
			case "microphone":
				return renderMicrophoneStep();
			case "blackhole":
				return renderBlackHoleStep();
			case "complete":
				return renderCompleteStep();
			default:
				return renderWelcomeStep();
		}
	};

	return (
		<div className="setup-wizard-overlay">
			<div className="setup-wizard">
				<div className="setup-progress">
					<div
						className={`progress-step ${currentStep === "welcome" ? "active" : ""} ${currentStep === "microphone" || currentStep === "blackhole" || currentStep === "complete" ? "completed" : ""}`}
					/>
					<div
						className={`progress-step ${currentStep === "microphone" ? "active" : ""} ${currentStep === "blackhole" || currentStep === "complete" ? "completed" : ""}`}
					/>
					<div
						className={`progress-step ${currentStep === "blackhole" ? "active" : ""} ${currentStep === "complete" ? "completed" : ""}`}
					/>
					<div className={`progress-step ${currentStep === "complete" ? "active" : ""}`} />
				</div>
				{renderCurrentStep()}
			</div>
		</div>
	);
}
