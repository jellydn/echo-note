import "@testing-library/jest-dom";
import { beforeEach, vi } from "vitest";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(),
	emit: vi.fn(),
}));

// Reset all mocks before each test
beforeEach(() => {
	vi.clearAllMocks();
});
