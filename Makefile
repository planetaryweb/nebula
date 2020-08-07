# Do not run tests on tavern_derive directly: panic=abort is not supported
CARGO_TEST_FLAGS=
CARGO_INCREMENTAL=0
CARGO_VERBOSE=false
CARGO_COVERAGE=false

ifeq (${CARGO_NIGHTLY}, true)
	CARGO_VERBOSE_FLAG=--verbose
	CARGO_COMMAND=rustup run nightly cargo
	RUSTUP_TARGET=rustup-nightly
	RUSTFLAGS=-Z macro-backtrace --cfg nightly --cfg procmacro2_semver_exempt
else
	CARGO_COMMAND=rustup run stable cargo
	CARGO_VERBOSE_FLAG=
	RUSTUP_TARGET=rustup-stable
	RUSTFLAGS=
endif

ifeq (${CARGO_COVERAGE}, true)
	RUSTFLAGS=${RUSTFLAGS} -Cpanic=abort -Zpanic_abort_tests -Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off
endif

.PHONY: all
all: test-ports

.PHONY: rustup
rustup:
	@if ! which rustup &> /dev/null; then\
		echo "Rustup is not available and is required. Press enter to install, or Ctrl-C to exit.";\
		read unused;\
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh;\
	fi

.PHONY: rustup-nightly
rustup-nightly: rustup
	@if [ -z "$(shell rustup toolchain list | grep "nightly-x86_64-unknown-linux-")" ]; then\
		echo "Installing nightly toolchain";\
		rustup toolchain install nightly;\
	fi

.PHONY: rustup-stable
rustup-stable: rustup
	@if [ -z "$(shell rustup toolchain list | grep "stable-x86_64-unknown-linux-")" ]; then\
		echo "Installing stable toolchain";\
		rustup toolchain install stable;\
	fi

.PHONY: test
test:
	${CARGO_COMMAND} test ${CARGO_TEST_FLAGS}

.PHONY: test-net
test-net:
	${CARGO_COMMAND} test --features test-ports

.PHONY: test-all
test-all:
	${CARGO_COMMAND} test --all-features

.PHONY: test-forms
test-forms: ${RUSTUP_TARGET}
	${CARGO_COMMAND} test --manifest-path nebula_forms/Cargo.toml ${CARGO_TEST_FLAGS}

.PHONY: test-rpc
test-rpc: ${RUSTUP_TARGET}
	${CARGO_COMMAND} test --manifest-path nebula_rpc/Cargo.toml ${CARGO_TEST_FLAGS}

.PHONY: test-rpc-net
test-rpc-net: export RUST_TEST_THREADS = 1
test-rpc-net: ${RUSTUP_TARGET}
	${CARGO_COMMAND} test --manifest-path nebula_rpc/Cargo.toml --features test-ports ${CARGO_TEST_FLAGS}

.PHONY: test-rpc-all
test-rpc-all: test-rpc-net

.PHONY: test-status
test-status: ${RUSTUP_TARGET}
	${CARGO_COMMAND} test --manifest-path nebula_status/Cargo.toml --all-features ${CARGO_TEST_FLAGS}

clean: ${RUSTUP_TARGET}
	${CARGO_COMMAND} clean

run: ${RUSTUP_TARGET}
	${CARGO_COMMAND} run --manifest-path nebula/Cargo.toml
