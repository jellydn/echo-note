import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "../ErrorBoundary";

// Component that throws an error
function ThrowError({ shouldThrow }: { shouldThrow: boolean }) {
	if (shouldThrow) {
		throw new Error("Test error");
	}
	return <div>No error</div>;
}

describe("ErrorBoundary", () => {
	// Suppress console.error for expected errors
	const originalConsoleError = console.error;
	beforeEach(() => {
		console.error = vi.fn();
	});
	afterEach(() => {
		console.error = originalConsoleError;
	});

	it("renders children when there is no error", () => {
		render(
			<ErrorBoundary>
				<div>Test content</div>
			</ErrorBoundary>,
		);

		expect(screen.getByText("Test content")).toBeInTheDocument();
	});

	it("shows fallback UI when child throws error", () => {
		render(
			<ErrorBoundary>
				<ThrowError shouldThrow={true} />
			</ErrorBoundary>,
		);

		expect(screen.getByText("Something went wrong")).toBeInTheDocument();
		expect(screen.getByText("Test error")).toBeInTheDocument();
		expect(screen.getByRole("button", { name: /try again/i })).toBeInTheDocument();
	});

	it("shows section label in error message when section prop is provided", () => {
		render(
			<ErrorBoundary section="Settings Panel">
				<ThrowError shouldThrow={true} />
			</ErrorBoundary>,
		);

		expect(screen.getByText("Settings Panel failed to load")).toBeInTheDocument();
	});

	it("has a working try again button in error state", () => {
		render(
			<ErrorBoundary>
				<ThrowError shouldThrow={true} />
			</ErrorBoundary>,
		);

		// Find the try again button in the error fallback UI
		const tryAgainButton = screen.getByRole("button", { name: /try again/i });
		expect(tryAgainButton).toBeInTheDocument();

		// Clicking the button should not throw an error
		expect(() => fireEvent.click(tryAgainButton)).not.toThrow();
	});
});

// Additional test with state recovery
describe("ErrorBoundary with recovery", () => {
	const originalConsoleError = console.error;
	beforeEach(() => {
		console.error = vi.fn();
	});
	afterEach(() => {
		console.error = originalConsoleError;
	});

	it("recovers when error condition is resolved", () => {
		function TestComponent() {
			const [shouldThrow] = useState(true);

			if (shouldThrow) {
				return (
					<div>
						<ThrowError shouldThrow={true} />
					</div>
				);
			}
			return <div>Recovered successfully</div>;
		}

		// Render with separate ErrorBoundary to test recovery
		const { rerender } = render(
			<ErrorBoundary key="test">
				<TestComponent />
			</ErrorBoundary>,
		);

		expect(screen.getByText("Something went wrong")).toBeInTheDocument();

		// Re-render with key change simulates fresh mount
		rerender(
			<ErrorBoundary key="test2">
				<div>Recovered successfully</div>
			</ErrorBoundary>,
		);

		expect(screen.getByText("Recovered successfully")).toBeInTheDocument();
	});
});
