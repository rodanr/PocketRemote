.PHONY: check desktop app mac fmt

check: desktop app

desktop:
	cd desktop && cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo clippy --features gui --all-targets --locked -- -D warnings && cargo test --locked && cargo deny check

app:
	cd app && npx tsc --noEmit && npx eslint . && npx jest && npm run test:interop

fmt:
	cd desktop && cargo fmt

mac:
	scripts/build-macos.sh
