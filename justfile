# Build Tailwind CSS
css:
    cd orchestrator && npm run build:css

# Watch Tailwind CSS for changes
css-watch:
    cd orchestrator && npm run watch:css

# Run the orchestrator server
serve:
    cargo run -p cocompute_orchestrator -- serve

# Development: bacon rebuilds CSS + restarts server on every change
dev:
    COCOMPUTE_SESSION_SECRET=dev-local-session-key-do-not-use-in-production-must-be-sixty-four-bytes bacon dev

# Optional local-only recipes (server IPs, personal shortcuts). Gitignored.
import? 'justfile.local'
