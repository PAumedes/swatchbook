##
## Swatchbook — developer convenience targets
##
## All heavy lifting (compilation, packaging) runs inside the Incus container.
## Uses incus-build (the Rust CLI) when available, else falls back to the
## legacy bash script.
##
## Usage:
##   make               → show this help
##   make <TAB><TAB>    → shell completion (source build-aux/make-completion.bash)
##

# ── Configuration ────────────────────────────────────────────────────────────

CONTAINER     := swatchbook-builder
PROJECT_DIR   := $(shell pwd)
INCUS_EXEC    := incus exec $(CONTAINER) --

# Prefer the compiled incus-build CLI; fall back to the legacy bash script.
INCUS_BUILD_BIN := $(shell command -v incus-build 2>/dev/null)
ifeq ($(INCUS_BUILD_BIN),)
  INCUS_BUILD := bash $(PROJECT_DIR)/build-aux/incus-build.sh
  INCUS_UP    := bash $(PROJECT_DIR)/build-aux/incus-build.sh
else
  INCUS_BUILD := incus-build build
  INCUS_UP    := incus-build up
endif

# Colours for pretty output
BOLD  := \033[1m
RESET := \033[0m
GREEN := \033[32m
CYAN  := \033[36m
RED   := \033[31m

# ── Default target ────────────────────────────────────────────────────────────

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help message
	@printf '$(BOLD)Swatchbook$(RESET) — Markdown-powered style binder for GNOME\n\n'
	@printf '$(BOLD)Usage:$(RESET) make $(CYAN)<target>$(RESET)\n\n'
	@printf '$(BOLD)Targets:$(RESET)\n'
	@awk 'BEGIN { FS = ":.*##" } \
	      /^[a-zA-Z_-]+:.*##/ { \
	          printf "  $(CYAN)%-22s$(RESET) %s\n", $$1, $$2 \
	      } \
	      /^##@/ { \
	          printf "\n$(BOLD)%s$(RESET)\n", substr($$0, 5) \
	      }' $(MAKEFILE_LIST)
	@printf '\n$(BOLD)Shell completion:$(RESET)\n'
	@printf '  source build-aux/make-completion.bash\n\n'

# ── Build ─────────────────────────────────────────────────────────────────────

##@ Build

.PHONY: build
build: ## Build the app and produce swatchbook.deb (via Incus)
	@printf '$(BOLD)$(GREEN)▶ Building Swatchbook...$(RESET)\n'
	@$(INCUS_BUILD)

.PHONY: build-dev
build-dev: container-up ## Build a debug binary inside the container (no .deb)
	@printf '$(BOLD)$(GREEN)▶ Building debug binary...$(RESET)\n'
	@$(INCUS_EXEC) bash -c " \
	    cd /tmp/swatchbook 2>/dev/null || { cp -r /src /tmp/swatchbook; cd /tmp/swatchbook; }; \
	    rm -rf _build; \
	    meson setup _build --prefix=/usr -Dprofile=development; \
	    meson compile -C _build"

.PHONY: rebuild
rebuild: clean build ## Clean then do a full build

# ── Container ─────────────────────────────────────────────────────────────────

##@ Container

.PHONY: container-up
container-up: ## Start the build container (create if missing)
	@printf '$(BOLD)$(GREEN)▶ Starting container...$(RESET)\n'
	@$(INCUS_UP)

.PHONY: container-stop
container-stop: ## Stop the build container
	@printf '$(BOLD)▶ Stopping container...$(RESET)\n'
	@if [ -n "$(INCUS_BUILD_BIN)" ]; then \
	    incus-build stop; \
	else \
	    incus stop $(CONTAINER) 2>/dev/null || true; \
	fi

.PHONY: container-delete
container-delete: ## Delete the build container entirely (frees disk space)
	@printf '$(BOLD)$(RED)▶ Deleting container $(CONTAINER)...$(RESET)\n'
	@incus delete --force $(CONTAINER) 2>/dev/null || true

.PHONY: container-shell
container-shell: container-up ## Open a shell inside the build container
	@if [ -n "$(INCUS_BUILD_BIN)" ]; then \
	    incus-build shell; \
	else \
	    incus exec $(CONTAINER) -- bash; \
	fi

.PHONY: container-status
container-status: ## Show container and network status
	@printf '$(BOLD)Container:$(RESET)\n'
	@if [ -n "$(INCUS_BUILD_BIN)" ]; then \
	    incus-build status; \
	else \
	    incus list $(CONTAINER) --format table 2>/dev/null || echo '  (not found)'; \
	fi
	@printf '\n$(BOLD)Network bridge:$(RESET)\n'
	@ip addr show incusbr-1000 2>/dev/null | grep 'inet ' || echo '  (no IPv4)'

# ── Testing ───────────────────────────────────────────────────────────────────

##@ Testing

.PHONY: test
test: container-up ## Run all tests (Meson + Cargo) inside the container
	@printf '$(BOLD)$(GREEN)▶ Running tests...$(RESET)\n'
	@$(INCUS_EXEC) bash -c " \
	    cd /tmp/swatchbook 2>/dev/null || { cp -r /src /tmp/swatchbook; cd /tmp/swatchbook; }; \
	    [ -d _build ] || meson setup _build --prefix=/usr; \
	    meson test -C _build --print-errorlogs; \
	    cargo test --manifest-path Cargo.toml"

.PHONY: test-cargo
test-cargo: container-up ## Run only Cargo unit tests inside the container
	@printf '$(BOLD)$(GREEN)▶ Running cargo tests...$(RESET)\n'
	@$(INCUS_EXEC) bash -c " \
	    cd /tmp/swatchbook 2>/dev/null || cp -r /src /tmp/swatchbook; \
	    cargo test --manifest-path /tmp/swatchbook/Cargo.toml"

.PHONY: lint
lint: container-up ## Run cargo clippy + fmt check inside the container
	@printf '$(BOLD)$(GREEN)▶ Linting...$(RESET)\n'
	@$(INCUS_EXEC) bash -c " \
	    cd /tmp/swatchbook 2>/dev/null || cp -r /src /tmp/swatchbook; \
	    cargo clippy --manifest-path /tmp/swatchbook/Cargo.toml -- -D warnings; \
	    cargo fmt --manifest-path /tmp/swatchbook/Cargo.toml --check"

# ── Package ───────────────────────────────────────────────────────────────────

##@ Flatpak

.PHONY: flatpak-sources
flatpak-sources: ## Regenerate cargo-sources.json from Cargo.lock
	@if [ -n "$(INCUS_BUILD_BIN)" ]; then \
	    incus-build flatpak regen; \
	else \
	    command -v flatpak-cargo-generator.py >/dev/null 2>&1 || \
	        pip3 install --quiet toml aiohttp aiofiles; \
	    python3 build-aux/flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json; \
	    printf '$(BOLD)$(GREEN)▶ cargo-sources.json generated$(RESET)\n'; \
	fi

.PHONY: flatpak-validate
flatpak-validate: ## Validate metainfo, desktop file, and Flatpak manifest
	@if [ -n "$(INCUS_BUILD_BIN)" ]; then \
	    incus-build flatpak validate; \
	else \
	    appstreamcli validate --no-net data/io.github.patricioaumedes.Swatchbook.metainfo.xml; \
	    desktop-file-validate data/io.github.patricioaumedes.Swatchbook.desktop; \
	fi

.PHONY: flatpak
flatpak: cargo-sources.json ## Build a local Flatpak bundle (requires flatpak-builder)
	@printf '$(BOLD)$(GREEN)▶ Building Flatpak...$(RESET)\n'
	@flatpak-builder --user --install --force-clean _flatpak-build \
	    io.github.patricioaumedes.Swatchbook.json

##@ Package

.PHONY: deb
deb: build ## Alias for build — produces swatchbook.deb

.PHONY: install-deb
install-deb: swatchbook.deb ## Install the .deb package locally
	@printf '$(BOLD)$(GREEN)▶ Installing swatchbook.deb...$(RESET)\n'
	sudo dpkg -i swatchbook.deb

.PHONY: uninstall
uninstall: ## Uninstall the package
	@printf '$(BOLD)▶ Uninstalling swatchbook...$(RESET)\n'
	sudo dpkg -r swatchbook

# ── Release ───────────────────────────────────────────────────────────────────

##@ Release

.PHONY: release-patch
release-patch: ## Bump patch version (0.1.0 → 0.1.1), build .deb, tag git
	@bash $(PROJECT_DIR)/build-aux/release.sh patch

.PHONY: release-minor
release-minor: ## Bump minor version (0.1.0 → 0.2.0), build .deb, tag git
	@bash $(PROJECT_DIR)/build-aux/release.sh minor

.PHONY: release-major
release-major: ## Bump major version (0.1.0 → 1.0.0), build .deb, tag git
	@bash $(PROJECT_DIR)/build-aux/release.sh major

.PHONY: release-watch
release-watch: ## Watch the latest GitHub Actions CI run in real time
	@gh run watch --repo PAumedes/swatchbook

.PHONY: release-status
release-status: ## Show recent GitHub Actions runs and release assets
	@printf '$(BOLD)Recent CI runs:$(RESET)\n'
	@gh run list --repo PAumedes/swatchbook --limit 5
	@printf '\n$(BOLD)Published releases:$(RESET)\n'
	@gh release list --repo PAumedes/swatchbook --limit 5

.PHONY: changelog
changelog: ## Show the full changelog
	@cat $(PROJECT_DIR)/build-aux/changelog

# ── Clean ─────────────────────────────────────────────────────────────────────

##@ Clean

.PHONY: clean
clean: ## Remove the build tree inside the container
	@printf '$(BOLD)▶ Cleaning build tree...$(RESET)\n'
	@$(INCUS_EXEC) bash -c "rm -rf /tmp/swatchbook" 2>/dev/null || true

.PHONY: clean-deb
clean-deb: ## Remove the local swatchbook.deb
	@rm -f swatchbook.deb
	@printf '$(BOLD)▶ Removed swatchbook.deb$(RESET)\n'

.PHONY: clean-all
clean-all: clean clean-deb ## Remove all build artefacts

# ── Utility ───────────────────────────────────────────────────────────────────

##@ Utility

.PHONY: fmt
fmt: container-up ## Auto-format Rust source with cargo fmt
	@printf '$(BOLD)$(GREEN)▶ Formatting...$(RESET)\n'
	@$(INCUS_EXEC) cargo fmt --manifest-path /tmp/swatchbook/Cargo.toml

.PHONY: completion
completion: ## Print instructions for enabling shell tab-completion
	@printf '$(BOLD)Shell completion setup:$(RESET)\n\n'
	@printf '  Add to your ~/.zshrc or ~/.bashrc:\n\n'
	@printf '    $(CYAN)source $(PROJECT_DIR)/build-aux/make-completion.bash$(RESET)\n\n'
	@printf '  Then restart your shell or run:\n\n'
	@printf '    $(CYAN)source build-aux/make-completion.bash$(RESET)\n\n'

.PHONY: version
version: ## Show project version
	@grep '^version' Cargo.toml | head -1 | awk '{print $$3}' | tr -d '"'

# Prevent make from treating a file named 'build', 'test' etc. as up-to-date
swatchbook.deb:
	@$(MAKE) build
