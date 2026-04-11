# Build Tailwind CSS
css:
    cd orchestrator && npm run build:css

# Watch Tailwind CSS for changes
css-watch:
    cd orchestrator && npm run watch:css

# Run the orchestrator server
serve:
    cargo run -p cocompute_orchestrator -- serve

# Build CSS + run the orchestrator (dev)
dev:
    just css
    just serve
