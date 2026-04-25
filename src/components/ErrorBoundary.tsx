import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
	children: ReactNode;
	/** Optional label shown in the fallback to identify which section crashed */
	section?: string;
}

interface State {
	hasError: boolean;
	error: Error | null;
}

/**
 * Catches render errors in a subtree and shows a fallback UI instead of
 * crashing the entire app. Wrap each major view with this component.
 */
export class ErrorBoundary extends Component<Props, State> {
	constructor(props: Props) {
		super(props);
		this.state = { hasError: false, error: null };
	}

	static getDerivedStateFromError(error: Error): State {
		return { hasError: true, error };
	}

	componentDidCatch(error: Error, info: ErrorInfo) {
		console.error(
			`[ErrorBoundary] Uncaught error in "${this.props.section ?? "unknown"}":`,
			error,
			info,
		);
	}

	render() {
		if (this.state.hasError) {
			return (
				<div style={{ padding: "2rem", textAlign: "center", color: "var(--text-secondary, #888)" }}>
					<p style={{ fontSize: "1.5rem", marginBottom: "0.5rem" }}>⚠️</p>
					<p style={{ fontWeight: 600, marginBottom: "0.25rem" }}>
						{this.props.section ? `${this.props.section} failed to load` : "Something went wrong"}
					</p>
					<p style={{ fontSize: "0.85rem", marginBottom: "1rem", opacity: 0.7 }}>
						{this.state.error?.message ?? "An unexpected error occurred"}
					</p>
					<button
						type="button"
						onClick={() => this.setState({ hasError: false, error: null })}
						style={{ padding: "0.4rem 1rem", cursor: "pointer" }}
					>
						Try again
					</button>
				</div>
			);
		}

		return this.props.children;
	}
}
