# Invoice — run `make` (or `make help`) for the target list.

SHELL := /usr/bin/env bash

export CARGO_BUILD_JOBS ?= 4

# The S3 storage tests read TEST_S3_* (see .env.example), taken from .env when present.
-include .env
export TEST_S3_ENDPOINT TEST_S3_BUCKET TEST_S3_REGION TEST_S3_ACCESS_KEY_ID TEST_S3_SECRET_ACCESS_KEY

# Integration tests use the local Postgres (see .env.example for the one-time setup).
TEST_DATABASE_URL ?= postgres://invoice:invoice@localhost:5432/invoice_test
export TEST_DATABASE_URL

IMAGE ?= ghcr.io/xmiksay/invoice:dev

.DEFAULT_GOAL := help

help: ## Show this help
	@grep -hE '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ===== Build / run ===========================================================
build: frontend-build ## Build the SPA, then the release binary (SPA embedded)
	cargo build --release

run: ## Run the server (cargo run -- serve; needs INVOICE__* env)
	cargo run -- serve

dev-server: ## Run the backend for development (debug build, serves frontend/dist from disk)
	cargo run -- serve

dev-frontend: ## Run the Vite dev server
	npm --prefix frontend run dev

# ===== Frontend ==============================================================
frontend-install: ## Install frontend dependencies (npm ci)
	npm --prefix frontend ci

frontend-build: ## Build the SPA into frontend/dist
	npm --prefix frontend run build

# ===== Lint ==================================================================
fmt: ## Format Rust sources
	cargo fmt --all

fmt-check: ## Check Rust formatting
	cargo fmt --all -- --check

clippy: ## Clippy, warnings are errors
	cargo clippy --all-targets -- -D warnings

lint-backend: fmt-check clippy ## Backend lint (fmt-check + clippy)

lint-frontend: ## Frontend lint + type-check
	npm --prefix frontend run lint
	npm --prefix frontend run type-check

lint: lint-backend lint-frontend ## Lint backend and frontend

# ===== Test ==================================================================
test-unit: ## Rust unit tests (no DB)
	cargo test --lib --bins

test-integration: ## Rust integration tests (needs local Postgres + the S3 test bucket, TEST_DATABASE_URL / TEST_S3_*)
	cargo test --test '*'

test-backend: test-unit test-integration ## All Rust tests

test-frontend: ## Frontend tests
	npm --prefix frontend run test

test: test-backend test-frontend ## All tests

# ===== Migrations ============================================================
migrate: ## Apply pending migrations (needs INVOICE__DATABASE_URL)
	cargo run -- migrate up

migrate-status: ## Show migration status (needs INVOICE__DATABASE_URL)
	cargo run -- migrate status

# ===== Docker / housekeeping =================================================
docker-build: ## Build the Docker image (override tag with IMAGE=...)
	docker build -t $(IMAGE) .

clean: ## Remove build artifacts
	cargo clean
	rm -rf frontend/dist

.PHONY: help build run dev-server dev-frontend frontend-install frontend-build \
	fmt fmt-check clippy lint-backend lint-frontend lint \
	test-unit test-integration test-backend test-frontend test \
	migrate migrate-status docker-build clean
