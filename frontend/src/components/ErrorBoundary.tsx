import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-surface flex items-center justify-center px-6">
          <div className="bg-surface-container-low rounded-xl p-12 max-w-md text-center">
            <div className="text-5xl mb-6">🐾</div>
            <h1 className="font-display text-2xl font-bold text-on-surface mb-3">
              Something went wrong
            </h1>
            <p className="text-on-surface-variant font-body mb-8">
              An unexpected error occurred. Please try reloading the page.
            </p>
            <button
              onClick={() => window.location.reload()}
              className="bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-xl px-8 py-3 font-display font-semibold"
            >
              Reload Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
