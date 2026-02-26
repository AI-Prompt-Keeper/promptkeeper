# Prompt Keeper CLI — build for Linux, macOS, Windows
BINARY_NAME := prke
LDFLAGS := -ldflags "-s -w"

.PHONY: all build build-linux build-darwin build-windows install clean

all: build

build:
	go build $(LDFLAGS) -o bin/$(BINARY_NAME) .

build-linux:
	GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-linux-amd64 .
	GOOS=linux GOARCH=arm64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-linux-arm64 .

build-darwin:
	GOOS=darwin GOARCH=amd64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-darwin-amd64 .
	GOOS=darwin GOARCH=arm64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-darwin-arm64 .

build-windows:
	GOOS=windows GOARCH=amd64 go build $(LDFLAGS) -o bin/$(BINARY_NAME)-windows-amd64.exe .

build-all: build-linux build-darwin build-windows
	@echo "Built all platform binaries in bin/"

install: build
	cp bin/$(BINARY_NAME) $(shell go env GOPATH)/bin/$(BINARY_NAME)
	@echo "Installed. Create symlink for promptkeeper: ln -sf $(BINARY_NAME) $(shell go env GOPATH)/bin/promptkeeper"

clean:
	rm -rf bin/
