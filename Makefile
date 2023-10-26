UNAME := $(shell uname)
WINDOWS := $(filter Windows_NT,$(OS))

ifeq ($(WINDOWS), Windows_NT)
	SHELL := pwsh
endif



.PHONY: install-tools
ifeq ($(UNAME), Linux)
install-tools:
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
	cargo-binstall sqlx-cli mprocs cargo-watch
endif

ifeq ($(UNAME), Darwin)
install-tools:
	echo Darwin
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
	cargo-binstall sqlx-cli mprocs cargo-watch
endif

ifeq ($(WINDOWS), Windows_NT)
install-tools:
	powershell -Command "Set-ExecutionPolicy Unrestricted -Scope Process; iex (iwr 'https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.ps1').Content"
	cargo-binstall sqlx-cli mprocs cargo-watch
endif


.PHONY: new-service
new-service:
	# Create a new Rust project
	cargo new $(name)
	# Add a new proc to mprocs.yaml
	echo "  $(name):\n    shell: cd $(name) && sqlx migrate run && cargo watch -x \"run -p $(name)\"" >> mprocs.yaml
	# Insert the new service name into Cargo.toml
	sed -i '4i\    "$(name)",' Cargo.toml
