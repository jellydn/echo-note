import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

// Download progress event payload
interface DownloadProgress {
	model_size: string;
	bytes_downloaded: number;
	total_bytes: number;
	percentage: number;
}

interface DiarizationDownloadProgress {
	model_id: string;
	bytes_downloaded: number;
	total_bytes: number;
	percentage: number;
}

// Tauri API response wrapper
interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

// Audio device info
interface AudioDeviceInfo {
	id: string;
	name: string;
}

// Whisper model info
interface WhisperModelInfo {
	size: string;
	filename: string;
	expected_size: number;
	is_downloaded: boolean;
	actual_size: number | null;
}

interface DiarizationModelStatus {
	id: string;
	filename: string;
	expected_size: number;
	is_downloaded: boolean;
	actual_size: number | null;
	model_path: string | null;
}

// Ollama status
interface OllamaStatusResponse {
	available: boolean;
	url: string;
}

// BlackHole status
interface BlackHoleStatusResponse {
	installed: boolean;
	device_name: string | null;
}

// Settings keys
const SETTING_AUDIO_DEVICE = "audio_device";
const SETTING_WHISPER_MODEL_SIZE = "whisper_model_size";
const SETTING_LLM_PROVIDER = "llm_provider";
const SETTING_API_KEY = "api_key";
const SETTING_API_ENDPOINT = "api_endpoint";
const SETTING_DIARIZATION_ENABLED = "diarization_enabled";
const SETTING_DIARIZATION_THRESHOLD = "diarization_threshold";

const DEFAULT_DIARIZATION_THRESHOLD = 0.75;

// Provider options
const PROVIDER_LOCAL = "ollama";
const PROVIDER_API = "api";

// Model size type - driven by backend model list
type ModelSize = string;

export function SettingsView() {
	// Settings state
	const [audioDevice, setAudioDevice] = useState<string>("");
	const [whisperModel, setWhisperModel] = useState<ModelSize>("small");
	const [llmProvider, setLlmProvider] = useState<string>(PROVIDER_LOCAL);
	const [apiKey, setApiKey] = useState<string>("");
	const [apiEndpoint, setApiEndpoint] = useState<string>("");
	const [diarizationEnabled, setDiarizationEnabled] = useState<boolean>(true);
	const [diarizationThreshold, setDiarizationThreshold] = useState<number>(
		DEFAULT_DIARIZATION_THRESHOLD,
	);

	// UI state
	const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
	const [modelInfo, setModelInfo] = useState<WhisperModelInfo[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [isSaving, setIsSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [successMessage, setSuccessMessage] = useState<string | null>(null);
	const [ollamaStatus, setOllamaStatus] = useState<OllamaStatusResponse | null>(null);
	const [blackholeStatus, setBlackholeStatus] = useState<BlackHoleStatusResponse | null>(null);
	const [hasHomebrew, setHasHomebrew] = useState<boolean>(false);
	const [isInstallingBlackhole, setIsInstallingBlackhole] = useState(false);
	const [isDownloading, setIsDownloading] = useState<string | null>(null);
	const [diarizationModelStatus, setDiarizationModelStatus] =
		useState<DiarizationModelStatus | null>(null);
	const [isDownloadingDiarizationModel, setIsDownloadingDiarizationModel] = useState(false);

	// Download progress state
	const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
	const [diarizationDownloadProgress, setDiarizationDownloadProgress] =
		useState<DiarizationDownloadProgress | null>(null);
	const unlistenRef = useRef<UnlistenFn | null>(null);
	const diarizationUnlistenRef = useRef<UnlistenFn | null>(null);

	// Fetch all settings and data on mount
	const loadSettings = useCallback(async () => {
		setIsLoading(true);
		setError(null);

		try {
			// Fetch audio devices
			const devicesResponse = await invoke<ApiResponse<AudioDeviceInfo[]>>(
				"list_audio_devices_command",
			);
			if (devicesResponse.success && devicesResponse.data) {
				setAudioDevices(devicesResponse.data);
			}

			// Fetch whisper models info
			const modelsResponse = await invoke<ApiResponse<WhisperModelInfo[]>>(
				"list_whisper_models_command",
			);
			if (modelsResponse.success && modelsResponse.data) {
				setModelInfo(modelsResponse.data);
			}

			const diarizationStatusResponse = await invoke<ApiResponse<DiarizationModelStatus>>(
				"check_diarization_status_command",
			);
			if (diarizationStatusResponse.success && diarizationStatusResponse.data) {
				setDiarizationModelStatus(diarizationStatusResponse.data);
			}

			// Fetch current settings
			const audioDeviceResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_AUDIO_DEVICE },
			});
			if (audioDeviceResponse.success && audioDeviceResponse.data) {
				setAudioDevice(audioDeviceResponse.data);
			}

			const whisperModelResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_WHISPER_MODEL_SIZE },
			});
			if (whisperModelResponse.success && whisperModelResponse.data) {
				setWhisperModel(whisperModelResponse.data);
			}

			const llmProviderResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_LLM_PROVIDER },
			});
			if (llmProviderResponse.success && llmProviderResponse.data) {
				setLlmProvider(llmProviderResponse.data);
			}

			const apiKeyResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_API_KEY },
			});
			if (apiKeyResponse.success && apiKeyResponse.data) {
				setApiKey(apiKeyResponse.data);
			}

			const apiEndpointResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_API_ENDPOINT },
			});
			if (apiEndpointResponse.success && apiEndpointResponse.data) {
				setApiEndpoint(apiEndpointResponse.data);
			}

			const diarizationEnabledResponse = await invoke<ApiResponse<string>>("get_setting_command", {
				request: { key: SETTING_DIARIZATION_ENABLED },
			});
			if (diarizationEnabledResponse.success && diarizationEnabledResponse.data) {
				setDiarizationEnabled(diarizationEnabledResponse.data.trim().toLowerCase() === "true");
			}

			const diarizationThresholdResponse = await invoke<ApiResponse<string>>(
				"get_setting_command",
				{ request: { key: SETTING_DIARIZATION_THRESHOLD } },
			);
			if (diarizationThresholdResponse.success && diarizationThresholdResponse.data) {
				const parsed = Number.parseFloat(diarizationThresholdResponse.data);
				if (Number.isFinite(parsed)) {
					setDiarizationThreshold(parsed);
				}
			}

			// Check Ollama status
			const ollamaResponse = await invoke<ApiResponse<OllamaStatusResponse>>(
				"check_ollama_status_command",
			);
			if (ollamaResponse.success && ollamaResponse.data) {
				setOllamaStatus(ollamaResponse.data);
			}

			// Check BlackHole status
			const blackholeResponse = await invoke<ApiResponse<BlackHoleStatusResponse>>(
				"check_blackhole_status_command",
			);
			if (blackholeResponse.success && blackholeResponse.data) {
				setBlackholeStatus(blackholeResponse.data);
			}

			// Check Homebrew status
			const homebrewResponse = await invoke<ApiResponse<boolean>>("check_homebrew_status_command");
			if (homebrewResponse.success && homebrewResponse.data !== null) {
				setHasHomebrew(homebrewResponse.data);
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to load settings");
		} finally {
			setIsLoading(false);
		}
	}, []);

	useEffect(() => {
		loadSettings();
	}, [loadSettings]);

	// Listen for download progress events
	useEffect(() => {
		const setupListener = async () => {
			const unlisten = await listen<DownloadProgress>("whisper-download-progress", (event) => {
				setDownloadProgress(event.payload);
				// If download completed (100%), refresh model info
				if (event.payload.percentage >= 100) {
					// Small delay to let the file be fully written
					setTimeout(async () => {
						const modelsResponse = await invoke<ApiResponse<WhisperModelInfo[]>>(
							"list_whisper_models_command",
						);
						if (modelsResponse.success && modelsResponse.data) {
							setModelInfo(modelsResponse.data);
						}
						setIsDownloading(null);
						setDownloadProgress(null);
					}, 500);
				}
			});
			unlistenRef.current = unlisten;
		};

		setupListener();

		return () => {
			if (unlistenRef.current) {
				unlistenRef.current();
			}
		};
	}, []);

	useEffect(() => {
		const setupListener = async () => {
			const unlisten = await listen<DiarizationDownloadProgress>(
				"diarization-download-progress",
				(event) => {
					setDiarizationDownloadProgress(event.payload);
					if (event.payload.percentage >= 100) {
						setTimeout(async () => {
							const response = await invoke<ApiResponse<DiarizationModelStatus>>(
								"check_diarization_status_command",
							);
							if (response.success && response.data) {
								setDiarizationModelStatus(response.data);
							}
							setIsDownloadingDiarizationModel(false);
							setDiarizationDownloadProgress(null);
						}, 500);
					}
				},
			);
			diarizationUnlistenRef.current = unlisten;
		};

		setupListener();

		return () => {
			if (diarizationUnlistenRef.current) {
				diarizationUnlistenRef.current();
			}
		};
	}, []);

	// Save a setting value
	const saveSetting = async (key: string, value: string) => {
		setIsSaving(true);
		setError(null);
		setSuccessMessage(null);

		try {
			const response = await invoke<ApiResponse<boolean>>("set_setting_command", {
				request: { key, value },
			});

			if (response.success && response.data) {
				setSuccessMessage("Settings saved successfully");
				setTimeout(() => setSuccessMessage(null), 3000);
				return true;
			}
			setError(response.error || `Failed to save ${key}`);
			return false;
		} catch (err) {
			setError(err instanceof Error ? err.message : `Failed to save ${key}`);
			return false;
		} finally {
			setIsSaving(false);
		}
	};

	// Handle audio device change
	const handleAudioDeviceChange = async (deviceId: string) => {
		setAudioDevice(deviceId);
		await saveSetting(SETTING_AUDIO_DEVICE, deviceId);
	};

	// Handle whisper model change
	const handleWhisperModelChange = async (modelSize: ModelSize) => {
		setWhisperModel(modelSize);
		await saveSetting(SETTING_WHISPER_MODEL_SIZE, modelSize);
	};

	// Handle LLM provider change
	const handleProviderChange = async (provider: string) => {
		setLlmProvider(provider);
		await saveSetting(SETTING_LLM_PROVIDER, provider);
	};

	// Handle API key change
	const handleApiKeyChange = async (key: string) => {
		setApiKey(key);
		await saveSetting(SETTING_API_KEY, key);
	};

	// Handle API endpoint change
	const handleApiEndpointChange = async (endpoint: string) => {
		setApiEndpoint(endpoint);
		await saveSetting(SETTING_API_ENDPOINT, endpoint);
	};

	// Handle diarization enable/disable
	const handleDiarizationToggle = async (enabled: boolean) => {
		setDiarizationEnabled(enabled);
		await saveSetting(SETTING_DIARIZATION_ENABLED, enabled ? "true" : "false");
	};

	// Handle diarization threshold change (debounced via blur/commit)
	const handleDiarizationThresholdCommit = async (value: number) => {
		const clamped = Math.min(0.95, Math.max(0.4, value));
		setDiarizationThreshold(clamped);
		await saveSetting(SETTING_DIARIZATION_THRESHOLD, clamped.toString());
	};

	// Download whisper model
	const downloadModel = async (modelSize: string) => {
		setIsDownloading(modelSize);
		setError(null);

		try {
			const response = await invoke<ApiResponse<string>>("download_whisper_model_command", {
				modelSize,
			});

			if (response.success && response.data) {
				// Refresh model info
				const modelsResponse = await invoke<ApiResponse<WhisperModelInfo[]>>(
					"list_whisper_models_command",
				);
				if (modelsResponse.success && modelsResponse.data) {
					setModelInfo(modelsResponse.data);
				}
				setSuccessMessage(`${modelSize} model downloaded successfully`);
				setTimeout(() => setSuccessMessage(null), 3000);
			} else {
				setError(response.error || `Failed to download ${modelSize} model`);
			}
		} catch (err) {
			const msg =
				err instanceof Error
					? err.message
					: typeof err === "string"
						? err
						: `Failed to download ${modelSize} model`;
			setError(msg);
		} finally {
			setIsDownloading(null);
		}
	};

	const downloadDiarizationModel = async () => {
		setIsDownloadingDiarizationModel(true);
		setError(null);
		setSuccessMessage(null);

		try {
			const response = await invoke<ApiResponse<string>>("download_diarization_model_command");

			if (response.success && response.data) {
				const statusResponse = await invoke<ApiResponse<DiarizationModelStatus>>(
					"check_diarization_status_command",
				);
				if (statusResponse.success && statusResponse.data) {
					setDiarizationModelStatus(statusResponse.data);
				}
				setSuccessMessage("Diarization model downloaded successfully");
				setTimeout(() => setSuccessMessage(null), 3000);
			} else {
				setError(response.error || "Failed to download diarization model");
			}
		} catch (err) {
			const msg =
				err instanceof Error
					? err.message
					: typeof err === "string"
						? err
						: "Failed to download diarization model";
			setError(msg);
		} finally {
			setIsDownloadingDiarizationModel(false);
		}
	};

	// Install BlackHole driver - opens download page in browser
	const installBlackHole = async () => {
		setIsInstallingBlackhole(true);
		setError(null);
		setSuccessMessage(null);

		try {
			const response = await invoke<ApiResponse<boolean>>("install_blackhole_command");

			if (response.success && response.data) {
				setSuccessMessage(
					"Opened BlackHole download page in your browser. Download the .pkg file, open it, and follow the installation steps. " +
						"After installation, click 'Check Again' to verify.",
				);
			} else {
				setError(response.error || "Failed to open BlackHole download page");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to open BlackHole download page");
		} finally {
			setIsInstallingBlackhole(false);
		}
	};

	// Install BlackHole via Homebrew
	const installBlackHoleHomebrew = async () => {
		setIsInstallingBlackhole(true);
		setError(null);
		setSuccessMessage(null);

		try {
			const response = await invoke<ApiResponse<boolean>>("install_blackhole_homebrew_command");

			if (response.success && response.data) {
				setSuccessMessage(
					"Terminal opened with the install command. Enter your password if prompted, then reboot your Mac. " +
						"After rebooting, click 'Check Again' to verify the installation.",
				);
			} else {
				setError(response.error || "Failed to install BlackHole via Homebrew");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to install BlackHole via Homebrew");
		} finally {
			setIsInstallingBlackhole(false);
		}
	};

	// Format file size for display
	const formatFileSize = (bytes: number): string => {
		const mb = bytes / (1024 * 1024);
		return `${Math.round(mb)} MB`;
	};

	// Check if any model is downloaded (for first-launch prompt)
	const hasAnyModelDownloaded = modelInfo.some((m) => m.is_downloaded);
	const needsModelDownload = !hasAnyModelDownloaded && !isLoading;

	if (isLoading) {
		return (
			<div className="settings-view">
				<div className="settings-container">
					<div className="settings-loading">
						<div className="settings-spinner" />
						<p>Loading settings...</p>
					</div>
				</div>
			</div>
		);
	}

	return (
		<div className="settings-view">
			<div className="settings-container">
				<div className="settings-header">
					<h2>Settings</h2>
					<p className="settings-subtitle">Configure your EchoNote preferences</p>
				</div>

				{error && (
					<div className="settings-error">
						<span className="error-icon">⚠️</span>
						<span>{error}</span>
					</div>
				)}

				{successMessage && (
					<div className="settings-success">
						<span className="success-icon">✓</span>
						<span>{successMessage}</span>
					</div>
				)}

				{/* Audio Settings */}
				<section className="settings-section">
					<h3 className="settings-section-title">Audio</h3>

					<div className="settings-field">
						<label htmlFor="audio-device" className="settings-label">
							Audio Input Device
						</label>
						<select
							id="audio-device"
							className="settings-select"
							value={audioDevice}
							onChange={(e) => handleAudioDeviceChange(e.target.value)}
							disabled={isSaving}
						>
							{audioDevices.length === 0 && <option value="">Default Device</option>}
							{audioDevices.map((device) => (
								<option key={device.id} value={device.id}>
									{device.name}
								</option>
							))}
						</select>
						<p className="settings-hint">Select the microphone to use for recording</p>
					</div>

					{/* BlackHole System Audio */}
					<div className="settings-field">
						{/* biome-ignore lint/a11y/noLabelWithoutControl: Label serves as section header */}
						<label className="settings-label">System Audio Capture</label>
						<div className="blackhole-status">
							{blackholeStatus?.installed ? (
								<div className="blackhole-installed">
									<span className="status-icon">✓</span>
									<span className="status-text">
										BlackHole installed -{" "}
										{blackholeStatus.device_name || "System audio will be recorded"}
									</span>
								</div>
							) : (
								<div className="blackhole-missing">
									<div className="blackhole-warning">
										<span className="warning-icon">⚠️</span>
										<span>
											<strong>BlackHole not installed.</strong> Only microphone audio will be
											recorded.
										</span>
									</div>
									<div className="blackhole-buttons">
										{hasHomebrew && (
											<button
												type="button"
												className="blackhole-homebrew-button"
												onClick={installBlackHoleHomebrew}
												disabled={isInstallingBlackhole}
											>
												{isInstallingBlackhole
													? "Installing via Homebrew..."
													: "Install via Homebrew (Recommended)"}
											</button>
										)}
										<button
											type="button"
											className="blackhole-install-button"
											onClick={installBlackHole}
											disabled={isInstallingBlackhole}
										>
											{isInstallingBlackhole ? "Opening Download Page..." : "Download Installer"}
										</button>
										<button
											type="button"
											className="blackhole-check-button"
											onClick={loadSettings}
											disabled={isLoading}
										>
											Check Again
										</button>
									</div>
									<p className="settings-hint">
										{hasHomebrew
											? "Homebrew is the easiest way to install BlackHole. Alternatively, download the installer manually from GitHub."
											: "BlackHole enables recording of system audio (e.g., meeting participants). Click 'Download Installer' to open the GitHub page, then download and install the driver. After installation, click 'Check Again'."}
									</p>
								</div>
							)}
						</div>
					</div>
				</section>

				{/* Whisper Model Settings */}
				<section className="settings-section">
					<h3 className="settings-section-title">Transcription Model</h3>

					<div className="settings-field">
						{/* biome-ignore lint/a11y/noLabelWithoutControl: Label serves as section header for radio group */}
						<label className="settings-label">Whisper Model Size</label>
						<div className="model-options">
							{modelInfo.map((info) => {
								const isSelected = whisperModel === info.size;
								const isDownloadingThis = isDownloading === info.size;

								return (
									<div key={info.size} className={`model-option ${isSelected ? "selected" : ""}`}>
										<div className="model-option-info">
											<input
												type="radio"
												id={`model-${info.size}`}
												name="whisper-model"
												value={info.size}
												checked={isSelected}
												onChange={() => handleWhisperModelChange(info.size)}
												disabled={isSaving}
											/>
											<label htmlFor={`model-${info.size}`} className="model-option-label">
												<span className="model-name">{info.size}</span>
												<span className="model-size">{formatFileSize(info.expected_size)}</span>
											</label>
										</div>
										{info.is_downloaded ? (
											<span className="model-status downloaded">✓ Downloaded</span>
										) : isDownloadingThis ? (
											<div className="model-download-progress">
												<div className="progress-bar">
													<div
														className="progress-fill"
														style={{ width: `${downloadProgress?.percentage ?? 0}%` }}
													/>
												</div>
												<span className="progress-text">
													{Math.round(downloadProgress?.percentage ?? 0)}%
												</span>
											</div>
										) : (
											<button
												type="button"
												className="model-download-button"
												onClick={() => downloadModel(info.size)}
												disabled={isDownloading !== null}
											>
												Download
											</button>
										)}
									</div>
								);
							})}
						</div>
						{needsModelDownload && (
							<div className="model-prompt">
								<span className="prompt-icon">📥</span>
								<span>
									<strong>No model downloaded yet.</strong> Select a model above and click Download
									to enable transcription.
								</span>
							</div>
						)}
						<p className="settings-hint">
							Larger models are more accurate but slower. Quantized (Q5) variants are smaller with
							minimal quality loss. Small is recommended for most use cases.
						</p>
						<div className="model-links">
							<button
								type="button"
								className="model-link-button"
								onClick={async () => {
									try {
										await invoke("open_models_folder_command");
									} catch {
										/* ignore */
									}
								}}
							>
								📂 Open Models Folder
							</button>
							<a
								href="https://huggingface.co/ggerganov/whisper.cpp"
								target="_blank"
								rel="noopener noreferrer"
								className="model-link-button"
								onClick={(e) => {
									e.preventDefault();
									invoke("plugin:opener|open_url", {
										url: "https://huggingface.co/ggerganov/whisper.cpp",
									}).catch(() => {
										window.open("https://huggingface.co/ggerganov/whisper.cpp", "_blank");
									});
								}}
							>
								🌐 Whisper Models on HuggingFace
							</a>
						</div>
					</div>
				</section>

				{/* Speaker Diarization Settings */}
				<section className="settings-section">
					<h3 className="settings-section-title">Speaker Diarization</h3>

					<div className="settings-field">
						<div className="model-option selected">
							<div className="model-option-info">
								<div className="model-option-label">
									<span className="model-name">ECAPA-TDNN speaker model</span>
									<span className="model-size">
										{formatFileSize(diarizationModelStatus?.expected_size ?? 9_337_463)}
									</span>
								</div>
							</div>
							{diarizationModelStatus?.is_downloaded ? (
								<span className="model-status downloaded">✓ Downloaded</span>
							) : isDownloadingDiarizationModel ? (
								<div className="model-download-progress">
									<div className="progress-bar">
										<div
											className="progress-fill"
											style={{ width: `${diarizationDownloadProgress?.percentage ?? 0}%` }}
										/>
									</div>
									<span className="progress-text">
										{Math.round(diarizationDownloadProgress?.percentage ?? 0)}%
									</span>
								</div>
							) : (
								<button
									type="button"
									className="model-download-button"
									onClick={downloadDiarizationModel}
									disabled={isDownloading !== null || isSaving}
								>
									Download
								</button>
							)}
						</div>
						{!diarizationModelStatus?.is_downloaded && (
							<div className="model-prompt">
								<span className="prompt-icon">📥</span>
								<span>
									<strong>No diarization model downloaded yet.</strong> Download it to identify
									speakers during transcription.
								</span>
							</div>
						)}
					</div>

					<div className="settings-field">
						<label
							htmlFor="diarization-enabled"
							className="settings-label"
							style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
						>
							<input
								type="checkbox"
								id="diarization-enabled"
								checked={diarizationEnabled}
								onChange={(e) => handleDiarizationToggle(e.target.checked)}
								disabled={isSaving}
							/>
							<span>Identify speakers in transcripts</span>
						</label>
						<p className="settings-hint">
							Groups transcript segments by voice and labels them "Speaker A", "Speaker B", etc.
							Runs locally after transcription.
						</p>
					</div>

					{diarizationEnabled && (
						<div className="settings-field">
							<label htmlFor="diarization-threshold" className="settings-label">
								Cluster sensitivity ({diarizationThreshold.toFixed(2)})
							</label>
							<input
								type="range"
								id="diarization-threshold"
								min={0.4}
								max={0.95}
								step={0.05}
								value={diarizationThreshold}
								onChange={(e) => setDiarizationThreshold(Number.parseFloat(e.target.value))}
								onMouseUp={(e) =>
									handleDiarizationThresholdCommit(
										Number.parseFloat((e.target as HTMLInputElement).value),
									)
								}
								onKeyUp={(e) =>
									handleDiarizationThresholdCommit(
										Number.parseFloat((e.target as HTMLInputElement).value),
									)
								}
								disabled={isSaving}
							/>
							<p className="settings-hint">
								Higher values create more distinct speakers (stricter matching); lower values merge
								similar voices.
							</p>
						</div>
					)}
				</section>

				{/* LLM Provider Settings */}
				<section className="settings-section">
					<h3 className="settings-section-title">Summary Generation</h3>

					<div className="settings-field">
						{/* biome-ignore lint/a11y/noLabelWithoutControl: Label serves as section header for radio group */}
						<label className="settings-label">LLM Provider</label>
						<div className="provider-options">
							<div className="provider-option">
								<input
									type="radio"
									id="provider-local"
									name="llm-provider"
									value={PROVIDER_LOCAL}
									checked={llmProvider === PROVIDER_LOCAL}
									onChange={() => handleProviderChange(PROVIDER_LOCAL)}
									disabled={isSaving}
								/>
								<label htmlFor="provider-local" className="provider-option-label">
									<span className="provider-name">Local (Ollama)</span>
									<span className="provider-description">
										{ollamaStatus?.available
											? "✓ Ollama is running"
											: "⚠ Ollama not detected - install from ollama.com"}
									</span>
								</label>
							</div>

							<div className="provider-option">
								<input
									type="radio"
									id="provider-api"
									name="llm-provider"
									value={PROVIDER_API}
									checked={llmProvider === PROVIDER_API}
									onChange={() => handleProviderChange(PROVIDER_API)}
									disabled={isSaving}
								/>
								<label htmlFor="provider-api" className="provider-option-label">
									<span className="provider-name">API (OpenAI-compatible)</span>
									<span className="provider-description">
										Use a cloud API for summary generation
									</span>
								</label>
							</div>
						</div>

						{llmProvider === PROVIDER_API && (
							<div className="api-settings">
								<div className="privacy-warning">
									<span className="warning-icon">⚠️</span>
									<span>Data will leave your device</span>
								</div>

								<div className="api-field">
									<label htmlFor="api-key" className="settings-label">
										API Key
									</label>
									<input
										type="password"
										id="api-key"
										className="settings-input"
										value={apiKey}
										onChange={(e) => handleApiKeyChange(e.target.value)}
										placeholder="sk-..."
										disabled={isSaving}
									/>
								</div>

								<div className="api-field">
									<label htmlFor="api-endpoint" className="settings-label">
										API Endpoint
									</label>
									<input
										type="text"
										id="api-endpoint"
										className="settings-input"
										value={apiEndpoint}
										onChange={(e) => handleApiEndpointChange(e.target.value)}
										placeholder="https://api.openai.com/v1"
										disabled={isSaving}
									/>
									<p className="settings-hint">
										OpenAI-compatible endpoint URL (optional, uses default if empty)
									</p>
								</div>
							</div>
						)}
					</div>
				</section>
			</div>
		</div>
	);
}

export default SettingsView;
