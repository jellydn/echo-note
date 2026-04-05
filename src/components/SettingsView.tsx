import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

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

// Ollama status
interface OllamaStatusResponse {
	available: boolean;
	url: string;
}

// Settings keys
const SETTING_AUDIO_DEVICE = "audio_device";
const SETTING_WHISPER_MODEL_SIZE = "whisper_model_size";
const SETTING_LLM_PROVIDER = "llm_provider";
const SETTING_API_KEY = "api_key";
const SETTING_API_ENDPOINT = "api_endpoint";

// Provider options
const PROVIDER_LOCAL = "ollama";
const PROVIDER_API = "api";

// Model size options
const MODEL_SIZES = ["tiny", "base", "small", "medium"] as const;
type ModelSize = (typeof MODEL_SIZES)[number];

export function SettingsView() {
	// Settings state
	const [audioDevice, setAudioDevice] = useState<string>("");
	const [whisperModel, setWhisperModel] = useState<ModelSize>("small");
	const [llmProvider, setLlmProvider] = useState<string>(PROVIDER_LOCAL);
	const [apiKey, setApiKey] = useState<string>("");
	const [apiEndpoint, setApiEndpoint] = useState<string>("");

	// UI state
	const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
	const [modelInfo, setModelInfo] = useState<WhisperModelInfo[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [isSaving, setIsSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [successMessage, setSuccessMessage] = useState<string | null>(null);
	const [ollamaStatus, setOllamaStatus] = useState<OllamaStatusResponse | null>(null);
	const [isDownloading, setIsDownloading] = useState<string | null>(null);

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
				const model = whisperModelResponse.data as ModelSize;
				if (MODEL_SIZES.includes(model)) {
					setWhisperModel(model);
				}
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

			// Check Ollama status
			const ollamaResponse = await invoke<ApiResponse<OllamaStatusResponse>>(
				"check_ollama_status_command",
			);
			if (ollamaResponse.success && ollamaResponse.data) {
				setOllamaStatus(ollamaResponse.data);
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

	// Download whisper model
	const downloadModel = async (modelSize: string) => {
		setIsDownloading(modelSize);
		setError(null);

		try {
			const response = await invoke<ApiResponse<string>>("download_whisper_model_command", {
				model_size: modelSize,
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
			setError(err instanceof Error ? err.message : `Failed to download ${modelSize} model`);
		} finally {
			setIsDownloading(null);
		}
	};

	// Format file size for display
	const formatFileSize = (bytes: number): string => {
		const mb = bytes / (1024 * 1024);
		return `${Math.round(mb)} MB`;
	};

	// Get model display info
	const getModelInfo = (size: string): WhisperModelInfo | undefined => {
		return modelInfo.find((m) => m.size === size);
	};

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
				</section>

				{/* Whisper Model Settings */}
				<section className="settings-section">
					<h3 className="settings-section-title">Transcription Model</h3>

					<div className="settings-field">
						{/* biome-ignore lint/a11y/noLabelWithoutControl: Label serves as section header for radio group */}
						<label className="settings-label">Whisper Model Size</label>
						<div className="model-options">
							{MODEL_SIZES.map((size) => {
								const info = getModelInfo(size);
								const isDownloaded = info?.is_downloaded ?? false;
								const isSelected = whisperModel === size;
								const isDownloadingThis = isDownloading === size;

								return (
									<div key={size} className={`model-option ${isSelected ? "selected" : ""}`}>
										<div className="model-option-info">
											<input
												type="radio"
												id={`model-${size}`}
												name="whisper-model"
												value={size}
												checked={isSelected}
												onChange={() => handleWhisperModelChange(size)}
												disabled={isSaving}
											/>
											<label htmlFor={`model-${size}`} className="model-option-label">
												<span className="model-name">
													{size.charAt(0).toUpperCase() + size.slice(1)}
												</span>
												<span className="model-size">
													{info ? formatFileSize(info.expected_size) : ""}
												</span>
											</label>
										</div>
										{isDownloaded ? (
											<span className="model-status downloaded">✓ Downloaded</span>
										) : (
											<button
												type="button"
												className="model-download-button"
												onClick={() => downloadModel(size)}
												disabled={isDownloading !== null}
											>
												{isDownloadingThis ? "Downloading..." : "Download"}
											</button>
										)}
									</div>
								);
							})}
						</div>
						<p className="settings-hint">
							Larger models are more accurate but slower. Small is recommended for most use cases.
						</p>
					</div>
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
